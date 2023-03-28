use std::{
    collections::HashMap,
    fs::{self, File},
    str::FromStr,
};

use ethers::{
    types::{Address, Filter, H160, H256},
    utils::{keccak256, to_checksum}, providers::{Provider, Ws, Middleware, StreamExt}, abi::Abi,
};
use serde_yaml::Mapping;



#[derive(Debug)]
pub struct Task {
    pub name: String,
    pub contracts: Vec<Address>,
    pub simple_events: Vec<Event>,
    pub abi: Option<Abi>,
}

pub async fn parse_config(provider: Provider<Ws>, path: String) -> Vec<Task> {
    let meta = fs::metadata(&path).unwrap();
    let input: serde_yaml::Mapping = if meta.is_dir() {
        let mut map = serde_yaml::Mapping::new();
        for file in fs::read_dir(path).unwrap() {
            let file = file.unwrap();

            if file.file_name().to_str().unwrap().ends_with(".yml") {
                let file = File::open(file.path()).unwrap();
                let unit: serde_yaml::Mapping = serde_yaml::from_reader(file).unwrap();
                for (k, v) in unit {
                    map.insert(k, v);
                }
            }
        }
        map
    } else {
        let file = File::open(path).unwrap();
        serde_yaml::from_reader(file).unwrap()
    };

    log::debug!("{:?}", input);

    let mut tasks: Vec<Task> = Vec::new();
    for (task_name, v) in input {
        log::debug!("{:?}", task_name);
        let task_detail = v.as_mapping().unwrap();
        let mut contracts = match task_detail.get("contracts") {
            Some(contracts) => contracts
                .as_sequence()
                .unwrap()
                .iter()
                .map(|addr| Address::from_str(addr.as_str().unwrap()).unwrap())
                .collect(),
            None => Vec::new(),
        };
        if task_detail.contains_key("factory") {
            let contracts_from_factory = get_contracts_from_factory(
                provider.clone(),
                task_detail["factory"].as_mapping().unwrap(),
            )
            .await;
            contracts.extend(contracts_from_factory);
        }

        if contracts.len() == 0 {
            continue;
        }

        // optional
        let simple_events = match task_detail.get("events") {
            Some(input_events) => input_events.as_sequence().unwrap()
                .to_owned()
                .iter()
                .map(|c| Event::new(c.as_str().unwrap().to_string()))
                .collect(),
            None => Vec::new(),
        };

        let abi = match task_detail.get("abi") {
            Some(abi_json_v) => {
                // println!("{}", abi_json_v.as_str().unwrap());
                Some(serde_json::from_str::<Abi>(abi_json_v.as_str().unwrap()).unwrap())
            },
            None => None,
        };


        let task = Task {
            name: task_name.as_str().unwrap().to_owned(),
            contracts,
            simple_events,
            abi
        };

        tasks.push(task);
    }

    return tasks;
}

async fn get_contracts_from_factory(provider: Provider<Ws>, factory_config: &Mapping) -> Vec<H160> {
    let mut contracts = Vec::new();
    if factory_config.contains_key("event") {
        // catch addresses by events
        let factories: Vec<H160> = factory_config["contracts"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|addr| Address::from_str(addr.as_str().unwrap()).unwrap())
            .collect();
        let event = Event::new(factory_config["event"].as_str().unwrap().to_string());
        let filter = Filter::new()
            // .from_block(16_000_000)
            .from_block(0_000_000)
            // .to_block(16_200_000)
            .event(&event.to_signature())
            .address(factories);
        let arg_index: usize = factory_config["arg"]
            .as_i64()
            .unwrap_or(0)
            .try_into()
            .unwrap();
        let mut stream = provider.get_logs_paginated(&filter, 100);
        while let Some(log) = stream.next().await {
            let log = log.unwrap();
            if arg_index < log.topics.len() {
                let new_contract_addr = Address::from(log.topics[1 + arg_index]);
                contracts.push(new_contract_addr);
                log::debug!(
                    "got contract {:#x} from factory {:#x}",
                    new_contract_addr,
                    log.address
                );
            }
            // TODO: support data
        }
    }

    return contracts;
}

#[derive(Debug, Clone)]
pub struct Event {
    pub name: String,
    pub params: Vec<EventParam>,
    pub topics: Vec<EventParam>,
    pub data: Vec<EventParam>,
}

impl Event {
    pub fn new(event: String) -> Self {
        // event = "Deposit(address indexed reverse, address indexed address , uint256 amount, uint16 indexed referral, uint256 timestamp);"
        let event = event.trim().replace("event", "");
        let name_with_body: Vec<&str> = event.split("(").collect();
        let name = name_with_body[0].trim().to_string();
        let body = name_with_body[1].replace(")", "").replace(";", "");
        let mut params = Vec::new();
        let mut topics = Vec::new();
        let mut data = Vec::new();

        let params_str_list: Vec<&str> = body.split(",").collect();
        let params_count: i32 = params_str_list.len().try_into().unwrap();
        for params_str in params_str_list {
            let triple: Vec<&str> = params_str.trim().split(" ").collect();
            assert!(triple.len() >= 2, "triple len incorrect");
            let name = triple[triple.len() - 1].trim().to_string();
            let mut evm_type = triple[0].trim().to_string();
            if evm_type == "uint" {
                evm_type = "uint256".to_string();
            }
            let indexed = triple[1] == "indexed"; // && !["string".to_string()].contains(&evm_type);
            let param = EventParam {
                name,
                indexed,
                evm_type,
            };

            params.push(param.clone());
            if indexed {
                topics.push(param);
            } else {
                data.push(param);
            }
        }

        let event = Event {
            name,
            params,
            topics,
            data,
        };
        return event;
    }

    pub fn to_signature(&self) -> String {
        let body: Vec<_> = self.params.iter().map(|x| x.to_signature()).collect();

        return self.name.to_string() + "(" + &body.join(",") + ")";
    }

    pub fn hash(&self) -> H256 {
        return H256::from(keccak256(self.to_signature().as_bytes()));
    }
}

#[derive(Debug, Clone)]
pub struct EventParam {
    pub name: String,
    pub indexed: bool,
    pub evm_type: String,
}

impl EventParam {
    pub fn to_string(&self) -> String {
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

    pub fn to_signature(&self) -> String {
        return self.evm_type.to_owned();
    }
}
