use ethers::{
    abi::{Abi, Hash},
    providers::{Middleware, Provider, StreamExt, Ws},
    types::{Address, Filter, H160, H256, I256, U256},
    utils::keccak256,
};
use serde_yaml::Mapping;
use std::{
    fs::{self, File},
    str::FromStr,
};

use crate::csv_output::CsvOutput;

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

        if contracts.is_empty() {
            continue;
        }

        // optional
        let simple_events = match task_detail.get("events") {
            Some(input_events) => input_events
                .as_sequence()
                .unwrap()
                .to_owned()
                .iter()
                .map(|c| Event::new(c.as_str().unwrap().to_string()))
                .collect(),
            None => Vec::new(),
        };

        let abi = task_detail.get("abi").map(|abi_json_v| serde_json::from_str::<Abi>(abi_json_v.as_str().unwrap()).unwrap());

        let task = Task {
            name: task_name.as_str().unwrap().to_owned(),
            contracts,
            simple_events,
            abi,
        };

        tasks.push(task);
    }

    tasks
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

        let save_path = factory_config.get("save");
        let mut csv_output = if let Some(save_path) = save_path {
            let save_path = save_path.as_str().unwrap();
            let mut csv_output = CsvOutput::new(save_path);
            let mut fields = Vec::new();
            for p in event.topics.iter() {
                fields.push(p.name.clone())
            }
            for p in event.data.iter() {
                fields.push(p.name.clone())
            }
            csv_output.add_headers(fields);
            csv_output.write_headers();
            Some(csv_output)
        } else {
            None
        };

        let mut stream = provider.get_logs_paginated(&filter, 100);
        while let Some(log) = stream.next().await {
            let log = log.unwrap();

            let tx = provider
                .get_transaction(log.transaction_hash.unwrap())
                .await
                .unwrap()
                .unwrap();

            let mut fixed_columns: Vec<String> = Vec::new();
            fixed_columns.push(log.block_number.unwrap().to_string());
            fixed_columns.push(format!("{:#x}", log.transaction_hash.unwrap()));
            fixed_columns.push(format!("{:#x}", tx.from));
            fixed_columns.push(tx.value.to_string());
            fixed_columns.push(match tx.to {
                None => "".to_owned(),
                Some(to) => format!("{:#x}", to),
            });
            fixed_columns.push(format!("{:#x}", log.address));

            let mut arg_columns: Vec<String> = Vec::new();
            for (index, param) in event.topics.iter().enumerate() {
                let raw = log.topics[index + 1]; // step over fn name
                let value = match param.evm_type.as_str() {
                    "address" => format!("{:#x}", Address::from(raw)),
                    "uint256" | "uint128" | "uint64" | "uint32" | "uint16" | "uint8" => {
                        U256::from(raw.as_bytes()).to_string()
                    }
                    "bool" => (!raw.is_zero()).to_string(),
                    "string" => {
                        // log::error!("{}.{}: {:#x} is a hash of string", task, event.name, raw);
                        // panic!("string in index?")
                        format!("{:#x}", raw) // = keccak(the_string)
                    }
                    "bytes32" => {
                        // log::error!("{}.{}: {:#x} is a byte32", task, event.name, raw);
                        // panic!("string in index?")
                        format!("{:#x}", raw)
                    }
                    _ => todo!(), // _ => format!("{:#x}", Address::from(raw)), // as address
                };

                arg_columns.push(value);
            }

            let raw_data = log.data;
            let mut pos: usize = 0;
            for param in &event.data {
                let evm_type = param.evm_type.as_str();
                let value = match evm_type {
                    "address" => {
                        let raw = &raw_data[pos..pos + 32];
                        pos += 32;
                        format!("{:#x}", Address::from(Hash::from_slice(raw)))
                    }
                    "uint256" | "uint128" | "uint96" | "uint64" | "uint32" | "uint16" | "uint8"
                    | "uint" => {
                        let raw = &raw_data[pos..pos + 32];
                        pos += 32;
                        U256::from(raw).to_string()
                    }
                    "int256" | "int128" | "int96" | "int64" | "int32" | "int16" | "int8"
                    | "int" => {
                        let raw = &raw_data[pos..pos + 32];
                        pos += 32;
                        I256::from_raw(raw.into()).to_string()
                    }
                    "bool" => {
                        let raw = &raw_data[pos..pos + 32];
                        pos += 32;
                        (U256::from(raw).is_zero()).to_string()
                    }
                    "string" => {
                        // read offset
                        // https://ethereum.stackexchange.com/questions/114592/how-is-function-data-encoded-decoded-if-a-string-exceeds-the-32-byte-length
                        // https://ethereum.stackexchange.com/questions/143471/how-does-etherscan-get-such-data
                        let raw = &raw_data[pos..pos + 32]; // must be 0x20
                        pos += 32;
                        let offset = U256::from(raw).as_usize();
                        // read_length
                        let raw = &raw_data[offset..offset + 32];

                        let len_str = U256::from(raw).as_usize();
                        let mut len_b32 = len_str / 32;
                        if len_b32 * 32 < len_str {
                            len_b32 += 1
                        }

                        let raw = &raw_data[offset + 32..offset + 32 + 32 * len_b32];
                        let raw_str = &raw[..len_str];

                        // suffix += 32 + 32 * len_b32;
                        String::from_utf8(raw_str.to_vec()).unwrap()
                    }
                    "bytes" => {
                        // read offset
                        // https://ethereum.stackexchange.com/questions/114592/how-is-function-data-encoded-decoded-if-a-string-exceeds-the-32-byte-length
                        let raw = &raw_data[pos..pos + 32]; // must be 0x20
                        pos += 32;
                        let offset = U256::from(raw).as_usize();
                        // read_length
                        let raw = &raw_data[offset..offset + 32];

                        let len_bytes = U256::from(raw).as_usize();
                        let mut len_b32 = len_bytes / 32;
                        if len_b32 * 32 < len_bytes {
                            len_b32 += 1
                        }

                        let raw = &raw_data[offset + 32..offset + 32 + 32 * len_b32];
                        let raw_bytes = &raw[..len_bytes];

                        // suffix += 32 + 32 * len_b32;
                        hex::encode(raw_bytes).to_string()
                    }
                    "bytes32" => {
                        let raw = &raw_data[pos..pos + 32];
                        pos += 32;
                        format!("{:#x}", Hash::from_slice(raw))
                    }
                    _ => panic!(
                        "unknown type {} in data, suggest to use abi instead",
                        param.evm_type
                    ),
                };

                arg_columns.push(value);
            }

            let new_contract_addr = Address::from_str(&arg_columns[arg_index]).unwrap();
            contracts.push(new_contract_addr);
            log::debug!(
                "got contract {:#x} from factory {:#x}",
                new_contract_addr,
                log.address
            );

            if let Some(csv_output) = &mut csv_output {
                let mut record = fixed_columns;
                record.extend(arg_columns);
                csv_output.write(record);
            }
        }
    }

    contracts
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
        let name_with_body: Vec<&str> = event.split('(').collect();
        let name = name_with_body[0].trim().to_string();
        let body = name_with_body[1].replace([')', ';'], "");
        let mut params = Vec::new();
        let mut topics = Vec::new();
        let mut data = Vec::new();

        let params_str_list: Vec<&str> = body.split(',').collect();
        // let params_count: i32 = params_str_list.len().try_into().unwrap();
        for params_str in params_str_list {
            let triple: Vec<&str> = params_str.trim().split(' ').collect();
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

        
        Event {
            name,
            params,
            topics,
            data,
        }
    }

    pub fn to_signature(&self) -> String {
        let body: Vec<_> = self.params.iter().map(|x| x.to_signature()).collect();

        self.name.to_string() + "(" + &body.join(",") + ")"
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
    // pub fn to_string(&self) -> String {
    //     if self.indexed {
    //         return vec![
    //             self.evm_type.to_owned(),
    //             "indexed".to_string(),
    //             self.name.to_owned(),
    //         ]
    //         .join(" ");
    //     }

    //     return vec![self.evm_type.to_owned(), self.name.to_owned()].join(" ");
    // }

    pub fn to_signature(&self) -> String {
        self.evm_type.to_owned()
    }
}
