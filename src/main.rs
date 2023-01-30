use csv::Writer;
use ethers::{
    prelude::*,
    providers::Provider,
    utils::{hex::ToHex, keccak256},
};
use rocksdb::DB;
use std::{any::Any, borrow::Borrow, collections::BTreeMap, fs::File, str::FromStr, sync::Arc};
use std::{borrow::BorrowMut, fmt::Debug, ops::Index};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    // let db = DB::open_default("./db").unwrap();
    let provider = Provider::<Ws>::connect("ws://172.24.1.2:8545")
        .await
        .unwrap(); // // Provider::<Ws>::connect("wss://mainnet.infura.io/ws/v3/dc6980e1063b421bbcfef8d7f58ccd43")
    let v = provider.client_version().await.unwrap();
    log::warn!("{v}");

    let kanban = parse_config();
    for task in kanban.tasks {
        log::warn!("Start task {}", task.name);
        dump_event_logs_from_contract(provider.borrow(), task.name, task.contracts, task.events)
            .await;
    }
}

#[derive(Debug, Clone)]
struct Kanban {
    tasks: Vec<Task>,
}

#[derive(Debug, Clone)]
struct Task {
    name: String,
    contracts: Vec<Address>,
    events: Vec<Event>,
}

fn parse_config() -> Kanban {
    let file = File::open("input.yml").unwrap();
    let input: serde_yaml::Mapping = serde_yaml::from_reader(file).unwrap();
    log::warn!("{:?}", input["events"]);

    let input_contracts = input["contracts"].as_mapping().unwrap();
    let input_events = input["events"].as_mapping().unwrap();
    let task_names = input_contracts.keys();
    let mut tasks = Vec::new();
    for task_name in task_names {
        let task = Task {
            name: task_name.as_str().unwrap().to_owned(),
            contracts: input_contracts
                .get(task_name)
                .unwrap()
                .as_sequence()
                .unwrap()
                .to_owned()
                .iter()
                .map(|c| Address::from_str(c.as_str().unwrap()).unwrap())
                .collect(),
            events: input_events
                .get(task_name)
                .unwrap()
                .as_sequence()
                .unwrap()
                .to_owned()
                .iter()
                .map(|c| Event::new(String::from(c.as_str().unwrap())))
                .collect(),
        };
        tasks.push(task);
    }

    return Kanban { tasks: tasks };
}

async fn dump_event_logs_from_contract(
    provider: &Provider<Ws>,
    task: String,
    addrs: Vec<Address>,
    events: Vec<Event>,
) {
    let event_signatures: Vec<String> = events.iter().map(|e| e.to_signature()).collect();
    let event_hashes: Vec<H256> = events.iter().map(|e| e.hash()).collect();
    let mut event_writers: Vec<Writer<File>> = events
        .iter()
        .map(|e| {
            csv::Writer::from_writer(File::create(format!("{}_{}.csv", task, e.name)).unwrap())
        })
        .collect();
    let fixed_fields = [
        "block_number",
        "transaction_hash",
        // "transaction_log_index",
    ];
    for (i, e) in events.iter().enumerate() {
        let mut fields = Vec::from(fixed_fields);
        for p in e.sorted.iter() {
            fields.push(&p.name)
        }

        event_writers[i].write_record(fields).unwrap();
    }

    let filter = Filter::new()
        .from_block(9241323)
        //.from_block(0_000_000)
        .to_block(16_200_000)
        .events(event_signatures)
        .address(addrs);
    let mut stream = provider.get_logs_paginated(&filter, 100);
    while let Some(log) = stream.next().await {
        let log = log.unwrap();
        log::debug!("{:?}", log);

        let mut topics_with_data = Vec::new();
        topics_with_data.extend(log.topics.clone());
        for chuck in log.data.chunks(32) {
            let h256_chunk = H256::from_slice(chuck);
            topics_with_data.push(h256_chunk);
        }

        let mut record: Vec<String> = Vec::new();
        record.push(log.block_number.unwrap().to_string());
        record.push(format!("{:#x}", log.transaction_hash.unwrap()));
        // record.push(log.transaction_log_index.unwrap().to_string());

        let fn_hash = topics_with_data[0].to_string();
        let event_index = event_hashes
            .iter()
            .position(|&h| h.to_string() == fn_hash)
            .unwrap();
        let event = &events[event_index];
        let mut index = 1;
        for param in &event.sorted {
            let value = match param.evm_type.as_str() {
                "address" => format!("{:#x}", Address::from(topics_with_data[index])),
                "uint256" => U256::from(topics_with_data[index].as_bytes()).to_string(),
                "uint16" => U256::from(topics_with_data[index].as_bytes()).to_string(),
                _ => todo!(),
            };

            record.push(value);

            index += 1;
        }

        event_writers[event_index].write_record(record).unwrap();

        event_writers[event_index].flush().unwrap();
    }

    return;
}

#[derive(Debug, Clone)]
struct Event {
    name: String,
    params: Vec<EventParam>,
    sorted: Vec<EventParam>,
}

impl Event {
    pub fn new(event: String) -> Self {
        // event = "Deposit(address indexed reverse, address indexed address , uint256 amount, uint16 indexed referral, uint256 timestamp);"
        let name_with_body: Vec<&str> = event.split("(").collect();
        let name = String::from(name_with_body[0]);
        let body = name_with_body[1].replace(")", "").replace(";", "");
        let mut params = Vec::new();
        let mut sorted = Vec::new();
        let mut index: i32 = 0;
        let params_str_list: Vec<&str> = body.split(",").collect();
        let params_count: i32 = params_str_list.len().try_into().unwrap();
        for params_str in params_str_list {
            let triple: Vec<&str> = params_str.trim().split(" ").collect();
            assert!(triple.len() >= 2, "triple len incorrect");
            let indexed = triple[1] == "indexed";
            let param = EventParam {
                rank: if indexed {
                    -params_count + index
                } else {
                    index
                },
                name: triple[triple.len() - 1].to_string(),
                indexed: indexed,
                evm_type: triple[0].to_string(),
            };

            let cloned_param = param.clone();

            params.push(param);
            sorted.push(cloned_param);
            index += 1;
        }
        sorted.sort_by(|a: &EventParam, b: &EventParam| a.rank.cmp(&b.rank));

        let event = Event {
            name: name,
            params: params,
            sorted,
        };
        return event;
    }

    fn to_signature(&self) -> String {
        let body: Vec<_> = self.params.iter().map(|x| x.to_signature()).collect();

        return self.name.to_owned() + "(" + &body.join(",") + ")";
    }

    fn hash(&self) -> H256 {
        return H256::from(keccak256(self.to_signature().as_bytes()));
    }
}

#[derive(Debug, Clone)]
struct EventParam {
    rank: i32,
    name: String,
    indexed: bool,
    evm_type: String,
}

impl EventParam {
    fn to_string(&self) -> String {
        if self.indexed {
            return vec![
                self.evm_type.to_owned(),
                "indexed".to_string(),
                self.name.to_owned(),
            ]
            .join(" ");
        }

        return vec![self.evm_type.to_owned(), self.name.to_owned()].join(" ");
    }

    fn to_signature(&self) -> String {
        return self.evm_type.to_owned();
    }
}

#[cfg(test)]
mod tests {
    use crate::Event;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_() {
        let e = Event::new("Deposit(address indexed reverse, address indexed address , uint256 amount, uint16 indexed referral, uint256 timestamp);".to_owned());
        println!("{:?}", e);
        println!("{}", e.to_signature());
        // assert_eq!((1, 2), 3);
    }
}
