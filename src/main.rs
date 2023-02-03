mod event_parser;

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
    events: Vec<event_parser::Event>,
}

fn parse_config() -> Kanban {
    let file = File::open("input.yml").unwrap();
    let input: serde_yaml::Mapping = serde_yaml::from_reader(file).unwrap();
    log::warn!("{:?}", input);

    let input_contracts = input["contracts"].as_mapping().unwrap();
    let input_events = input["sources"].as_mapping().unwrap();
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
                .as_mapping()
                .unwrap()["events"]
                .as_sequence()
                .unwrap()
                .to_owned()
                .iter()
                .map(|c| event_parser::Event::new(String::from(c.as_str().unwrap())))
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
    events: Vec<event_parser::Event>,
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
        for p in e.topics.iter() {
            fields.push(&p.name)
        }
        for p in e.data.iter() {
            fields.push(&p.name)
        }
        event_writers[i].write_record(fields).unwrap();
    }

    log::warn!("{:?}", event_signatures);
    let filter = Filter::new()
        // .from_block(16_000_000)
        .from_block(0_000_000)
        .to_block(16_200_000)
        // .events(event_signatures)
        .address(addrs);
    let mut stream = provider.get_logs_paginated(&filter, 100);
    while let Some(log) = stream.next().await {
        let log = log.unwrap();
        log::debug!("{:?}", log);

        let mut record: Vec<String> = Vec::new();
        record.push(log.block_number.unwrap().to_string());
        record.push(format!("{:#x}", log.transaction_hash.unwrap()));
        // record.push(log.transaction_log_index.unwrap().to_string());

        let fn_hash = log.topics[0];
        let event_index = event_hashes
            .iter()
            .position(|&h| fn_hash == h)
            .unwrap();
        let event = &events[event_index];
        log::debug!("found event {:?}", event);

        for (index, param) in event.topics.iter().enumerate() {
            let raw = log.topics[index+1]; // step over fn name
            let value = match param.evm_type.as_str() {
                "address" => format!("{:#x}", Address::from(raw)),
                "uint256" => U256::from(raw.as_bytes()).to_string(),
                "uint16" => U256::from(raw.as_bytes()).to_string(),
                "bool" => (!raw.is_zero()).to_string(),
                // "string" => 
                _ => format!("{:#x}", Address::from(raw)), // as address
            };

            record.push(value);
        }

        // read from data
        let raw_data = log.data;  
        let mut pos: usize = 0;
        for (index, param) in event.data.iter().enumerate() {
            let value = match param.evm_type.as_str() {
                "address" => {
                    let raw = &raw_data[pos..pos+32];
                    pos += 32;
                    format!("{:#x}", Address::from(H256::from_slice(raw)))
                },
                "uint256" => {
                    let raw = &raw_data[pos..pos+32];
                    pos += 32;
                    U256::from(raw).to_string()
                },
                "uint16" => {
                    let raw = &raw_data[pos..pos+32];
                    pos += 32;
                    U256::from(raw).to_string()
                },
                "bool" => {
                    let raw = &raw_data[pos..pos+32];
                    pos += 32;
                    (U256::from(raw).is_zero()).to_string()
                },
                // "string" => 
                _ => todo!(),
            };

            record.push(value);
        }
        assert!(pos==raw_data.len(), "{} != {}", pos, raw_data.len());

        event_writers[event_index].write_record(record).unwrap();

        event_writers[event_index].flush().unwrap();
    }

    return;
}
