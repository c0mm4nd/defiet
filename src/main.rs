mod config_parser;

use clap::Parser;
use csv::Writer;
use ethers::abi::{Hash, RawLog, Address, ethereum_types, Abi, Token};
use ethers::providers::{Ws, Middleware, StreamExt};
use ethers::types::{U256, I256, H256, Filter};
use ethers::{providers::Provider};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::{fs, fs::File, path::Path, str::FromStr};


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Ethereum node's WS provider string
    #[arg(short, long, default_value = "ws://127.0.0.1:8545")]
    ethereum: String,

    /// config path, can be file or folder
    #[arg(short, long, default_value = "./input.yml")]
    config: String,

    /// config path, can be file or folder
    #[arg(short, long, default_value = "./output")]
    output: String,

    /// run for each protocol in parallel
    #[arg(short, long, default_value_t = false)]
    parallel: bool,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init_timed();
    let args = Args::parse();

    // let db = DB::open_default("./db").unwrap();
    let provider = Provider::<Ws>::connect(args.ethereum).await.unwrap();
    let v = provider.client_version().await.unwrap();
    log::warn!("{v}");

    let tasks = config_parser::parse_config(provider.clone(), args.config).await;

    let mut handles: Vec<_> = vec![];
    for task in tasks {
        let provider = provider.clone();
        let output = args.output.clone();
        handles.push(tokio::spawn(async move {
            log::warn!("Start task {}", task.name);
            match task.abi {
                None => {
                    dump_logs_from_events(
                        provider,
                        task.name,
                        output,
                        task.contracts,
                        task.simple_events.to_vec(),
                    )
                    .await;
                },
                Some(abi) => {
                    dump_logs_from_abi(provider, task.name, output, task.contracts, abi).await;
                }
            }

        }));
    }

    for handle in handles {
        handle.await.unwrap()
    }
}

async fn dump_logs_from_events(
    provider: Provider<Ws>,
    task: String,
    output: String,
    addrs: Vec<Address>,
    events: Vec<config_parser::Event>,
) {
    let event_signatures: Vec<String> = events.iter().map(|e| e.to_signature()).collect();
    let event_hashes: Vec<Hash> = events.iter().map(|e| e.hash()).collect();
    let path = Path::new(&output);
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
        "transaction_from",  // tx from
        "transaction_to",    // tx to
        "transaction_value", // tx value
        "contract",
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
        .from_block(0_000_000)
        // .to_block(16_200_000)
        .events(event_signatures)
        .address(addrs);
    let mut stream = provider.get_logs_paginated(&filter, 100);
    while let Some(log) = stream.next().await {
        let log = log.unwrap();
        // log::debug!("{:?}", log);

        let tx = provider
            .get_transaction(log.transaction_hash.unwrap())
            .await
            .unwrap()
            .unwrap();

        let mut record: Vec<String> = Vec::new();
        record.push(log.block_number.unwrap().to_string());
        record.push(format!("{:#x}", log.transaction_hash.unwrap()));
        record.push(format!("{:#x}", tx.from));
        record.push(tx.value.to_string());
        record.push(match tx.to {
            None => "".to_owned(),
            Some(to) => format!("{:#x}", to),
        });
        record.push(format!("{:#x}", log.address));
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
        log::debug!(
            "{}: found event {} on tx: {:#x}",
            task,
            event.name,
            log.transaction_hash.unwrap()
        );

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

            record.push(value);
        }

        // read from data
        let mut raw_data = log.data;
        let mut pos: usize = 0;
        let mut suffix: usize = 0;
        for (index, param) in event.data.iter().enumerate() {
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
                "int256" | "int128" | "int96" | "int64" | "int32" | "int16" | "int8" | "int" => {
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

                    suffix += 32 + 32 * len_b32;
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

                    suffix += 32 + 32 * len_b32;
                    format!("{}", hex::encode(raw_bytes))
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

            record.push(value);
        }
        assert!(
            pos + suffix == raw_data.len(),
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

async fn dump_logs_from_abi(
    provider: Provider<Ws>,
    task: String,
    output: String,
    addrs: Vec<Address>,
    abi: Abi,
) {
    let event_signature_map: BTreeMap<_, _> = abi.events().map(|e| (e.signature(), e)).collect();

    let path = Path::new(&output);
    if !path.exists() {
        fs::create_dir_all(&path).unwrap();
    }
    let mut event_writers: BTreeMap<_, Writer<File>> = abi.events()
        .map(|e| {
            let path = path.join(format!("{}_{}.csv", task, e.name));
            (
                e.signature(),
                csv::Writer::from_writer(File::create(path).unwrap()),
            )
        })
        .collect();
    let fixed_fields = [
        "block_number",
        "transaction_hash",
        "transaction_from",  // tx from
        "transaction_to",    // tx to
        "transaction_value", // tx value
        "contract",
        // "transaction_log_index",
    ];

    for e in abi.events() {
        let mut fields = Vec::from(fixed_fields);
        for param in &e.inputs {
            let name = &param.name;
            fields.push(name);
        }
        event_writers
            .get_mut(&e.signature())
            .unwrap()
            .write_record(fields)
            .unwrap();
    }

    let event_signatures: Vec<_> = abi.events().map(|e| e.signature()).collect();
    log::warn!("{}: {:?}", task, event_signatures);
    let filter = Filter::new()
        .from_block(0_000_000)
        // .to_block(16_200_000)
        .events(event_signatures)
        .address(addrs);
    let mut stream = provider.get_logs_paginated(&filter, 100);
    while let Some(log) = stream.next().await {
        let log = log.unwrap();
        // log::debug!("{:?}", log);

        let tx = provider
            .get_transaction(log.transaction_hash.unwrap())
            .await
            .unwrap()
            .unwrap();

        let mut record: Vec<String> = Vec::new();
        record.push(log.block_number.unwrap().to_string());
        record.push(format!("{:#x}", log.transaction_hash.unwrap()));
        record.push(format!("{:#x}", tx.from));
        record.push(tx.value.to_string());
        record.push(match tx.to {
            None => "".to_owned(),
            Some(to) => format!("{:#x}", to),
        });
        record.push(format!("{:#x}", log.address));

        let sig = &log.topics[0];
        let event = event_signature_map.get(sig).unwrap();

        let raw_log = event.parse_log_whole(RawLog { topics: log.topics.to_vec(), data: log.data.to_vec() }).unwrap();
        for param in raw_log.params {
            match &param.value {
                Token::Address(addr) => record.push(addr.to_string()),
                Token::Uint(u) => record.push(u.to_string()),
                Token::Int(i) => record.push(i.to_string()),
                Token::Bool(ok) => record.push(ok.to_string()),
                Token::FixedBytes(bytes) => record.push(hex::encode(bytes)),
                Token::FixedArray(_) => todo!(),
                Token::String(s) => record.push(s.clone()),
                Token::Bytes(bytes) => record.push(hex::encode(bytes)),
                Token::Array(_) => todo!(),
                Token::Tuple(_) => todo!(),
            }
            
        }

        event_writers
            .get_mut(sig)
            .unwrap()
            .write_record(record)
            .unwrap();

        event_writers.get_mut(sig).unwrap().flush().unwrap();
    }
}
