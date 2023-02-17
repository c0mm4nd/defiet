mod event_parser;

use clap::Parser;
use csv::Writer;
use ethers::{prelude::*, providers::Provider, utils::keccak256};
use std::fmt::Debug;
use std::{fs, fs::File, path::Path, str::FromStr};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// config path
    #[arg(short, long, default_value = "./input.yml")]
    config: String,

    #[arg(short, long, default_value_t = false)]
    parallel: bool,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init_timed();
    let args = Args::parse();

    // let db = DB::open_default("./db").unwrap();
    let provider = Provider::<Ws>::connect("ws://172.24.1.2:8545")
        .await
        .unwrap(); // // Provider::<Ws>::connect("wss://mainnet.infura.io/ws/v3/dc6980e1063b421bbcfef8d7f58ccd43")
    let v = provider.client_version().await.unwrap();
    log::warn!("{v}");

    let kanban = parse_config(args.config);
    let tasks = kanban.tasks;
    let mut handles: Vec<_> = vec![];
    for task in tasks {
        let provider = provider.clone();
        if args.parallel {
            handles.push(tokio::spawn(async move {
                log::warn!("Start task {}", task.name);
                dump_event_logs_from_contract(
                    provider,
                    task.name,
                    task.contracts,
                    task.events.to_vec(),
                )
                .await;
            }));
        } else {
            dump_event_logs_from_contract(
                provider,
                task.name,
                task.contracts,
                task.events.to_vec(),
            )
            .await;
        }
    }

    for handle in handles {
        handle.await.unwrap()
    }
}

#[derive(Debug, Clone)]
struct Kanban {
    tasks: Vec<Task>,
}

#[derive(Debug, Clone)]
struct Task {
    pub name: String,
    contracts: Vec<Address>,
    events: Vec<event_parser::Event>,
}

fn parse_config(path: String) -> Kanban {
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
    for (task_name, task_detail) in input {
        log::debug!("{:?}", task_name);
        let contracts = task_detail["contracts"].as_sequence().unwrap();
        if contracts.len() == 0 {
            continue;
        }

        let input_events = task_detail["events"].as_sequence().unwrap();
        let events: Vec<event_parser::Event> = input_events
            .to_owned()
            .iter()
            .map(|c| event_parser::Event::new(String::from(c.as_str().unwrap())))
            .collect();
        let task = Task {
            name: task_name.as_str().unwrap().to_owned(),
            contracts: contracts
                .to_owned()
                .iter()
                .map(|c| Address::from_str(c.as_str().unwrap()).unwrap())
                .collect(),
            events,
        };

        tasks.push(task);
    }

    return Kanban { tasks };
}

async fn dump_event_logs_from_contract(
    provider: Provider<Ws>,
    task: String,
    addrs: Vec<Address>,
    events: Vec<event_parser::Event>,
) {
    let event_signatures: Vec<String> = events.iter().map(|e| e.to_signature()).collect();
    let event_hashes: Vec<H256> = events.iter().map(|e| e.hash()).collect();
    let path = Path::new(".").join("csv_output");
    if !path.exists() {
        fs::create_dir_all(&path).unwrap();
    }
    let mut event_writers: Vec<Writer<File>> = events
        .iter()
        .map(|e| {
            let path = path.join(format!("{}_{}.csv", task, e.name));
            csv::Writer::from_writer(File::create(path).unwrap())
        })
        .collect();
    let fixed_fields = [
        "block_number",
        "transaction_hash",
        "caller", // tx from
        "contract", // tx to
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

    log::warn!("{}: {:?}", task, event_signatures);
    let filter = Filter::new()
        // .from_block(16_000_000)
        .from_block(0_000_000)
        .to_block(16_200_000)
        .events(event_signatures)
        .address(addrs);
    let mut stream = provider.get_logs_paginated(&filter, 100);
    while let Some(log) = stream.next().await {
        let log = log.unwrap();
        // log::debug!("{:?}", log);

        let tx = provider.get_transaction(log.transaction_hash.unwrap()).await.unwrap().unwrap();

        let mut record: Vec<String> = Vec::new();
        record.push(log.block_number.unwrap().to_string());
        record.push(format!("{:#x}", log.transaction_hash.unwrap()));
        record.push(format!("{:#x}", tx.from));
        record.push(match tx.to {
            None => "".to_owned(),
            Some(to) => format!("{:#x}", to),
        });
        // record.push(log.transaction_log_index.unwrap().to_string());

        let fn_hash = log.topics[0];
        let event_index = match event_hashes.iter().position(|&h| fn_hash == h) {
            Some(event_index) => event_index,
            None => {
                log::error!("{}: {:#x} not in {:?}", task, fn_hash, event_hashes);
                continue;
            }
        };
        let event = &events[event_index];
        log::debug!("{}: found event {:?}", task, event);

        assert!(
            event.topics.len() == log.topics.len() - 1,
            "{}.{}: config {} != log no fn {}",
            task,
            event.name,
            event.topics.len(),
            log.topics.len() - 1
        );
        for (index, param) in event.topics.iter().enumerate() {
            let raw = log.topics[index + 1]; // step over fn name
            let value = match param.evm_type.as_str() {
                "address" => format!("{:#x}", Address::from(raw)),
                "uint256" | "uint128" | "uint64" | "uint32" | "uint16" | "uint8" => {
                    U256::from(raw.as_bytes()).to_string()
                }
                "bool" => (!raw.is_zero()).to_string(),
                "string" => {
                    log::error!("{}.{}: {:#x} is a hash of string", task, event.name, raw);
                    // panic!("string in index?")
                    format!("{:#x}", raw) // = keccak(the_string)
                }
                "bytes32" => {
                    log::error!("{}.{}: {:#x} is a byte32", task, event.name, raw);
                    // panic!("string in index?")
                    format!("{:#x}", raw)
                }
                _ => todo!(), // _ => format!("{:#x}", Address::from(raw)), // as address
            };

            record.push(value);
        }

        // read from data
        let raw_data = log.data;
        let mut pos: usize = 0;
        for (index, param) in event.data.iter().enumerate() {
            let value = match param.evm_type.as_str() {
                "address" => {
                    let raw = &raw_data[pos..pos + 32];
                    pos += 32;
                    format!("{:#x}", Address::from(H256::from_slice(raw)))
                }
                "uint256" | "uint128" | "uint64" | "uint32" | "uint16" | "uint8" => {
                    let raw = &raw_data[pos..pos + 32];
                    pos += 32;
                    U256::from(raw).to_string()
                }
                "int256" | "int128" | "int64" | "int32" | "int16" | "int8" => {
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
                    // read_length
                    let raw = &raw_data[pos..pos + 32];
                    pos += 32;
                    let len_u256 = U256::from(raw).as_usize();
                    let raw = &raw_data[pos..pos + 32 * len_u256];
                    String::from_utf8(raw.to_vec()).unwrap()
                }
                _ => panic!("unknown type {} in data", param.evm_type),
            };

            record.push(value);
        }
        assert!(
            pos == raw_data.len(),
            "{}.{}: data parsed {} != {} in actual",
            task,
            event.name,
            pos,
            raw_data.len()
        );

        event_writers[event_index].write_record(record).unwrap();

        event_writers[event_index].flush().unwrap();
    }

    return;
}
