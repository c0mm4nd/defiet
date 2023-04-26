use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::Path,
    sync::{Arc, Mutex},
};

use csv::Writer;
use ethers::{
    abi::{Abi, Event, RawLog, Token},
    providers::{Middleware, Provider, StreamExt, Ws},
    types::{Address, Filter, Log, H256, I256, U256, U64},
};
use tokio::join;

use crate::config_parser;

const FIXED_FIELDS: &[&str] = &[
    "block_number",
    "transaction_hash",
    "transaction_from",  // tx from
    "transaction_to",    // tx to
    "transaction_value", // tx value
    "status",
    "contract",
];

#[derive(Debug, Clone)]
pub struct LogParser {
    pub provider: Provider<Ws>,
    pub addrs: Vec<Address>,
    pub task_name: String,
    pub output: String,

    pub start: u64,
    pub end: u64,

    pub with_gas: bool,
}

impl LogParser {
    pub async fn dump_logs_from_events(&self, events: Vec<config_parser::Event>) {
        let event_signatures: Vec<String> = events.iter().map(|e| e.to_signature()).collect();
        let event_hashes: Vec<H256> = events.iter().map(|e| e.hash()).collect();
        let path = Path::new(&self.output);
        if !path.exists() {
            fs::create_dir_all(path).unwrap();
        }
        let mut event_writers: Vec<Writer<File>> = events
            .iter()
            .map(|e| {
                let path = path.join(format!("{}_{}.csv", self.task_name, e.name));
                csv::Writer::from_writer(File::create(path).unwrap())
            })
            .collect();

        for (i, e) in events.iter().enumerate() {
            let mut fields = Vec::from(FIXED_FIELDS);
            if self.with_gas {
                // total = gas used * (base fee + priority fee)
                // base: for concensus, burnt
                // priority: fee to validator
                fields.push("gas_used");
                fields.push("priority_fee_per_gas");
                fields.push("effective_gas_price");
            }

            for p in e.topics.iter() {
                fields.push(&p.name)
            }
            for p in e.data.iter() {
                fields.push(&p.name)
            }
            event_writers[i].write_record(fields).unwrap(); // write headers
        }

        log::warn!("{}: {:?}", self.task_name, event_signatures);
        let mut filter = Filter::new()
            .events(event_signatures)
            .address(self.addrs.to_vec())
            .from_block(self.start);

        if self.end != 0 {
            filter = filter.to_block(self.end);
        }

        let mut stream = self.provider.get_logs_paginated(&filter, 100);
        while let Some(log) = stream.next().await {
            let log = log.unwrap();
            // log::debug!("{:?}", log);

            let fn_hash = log.topics[0];

            let event_index = match event_hashes.iter().position(|&h| fn_hash == h) {
                Some(event_index) => event_index,
                None => {
                    let task_name = self.task_name.clone();
                    log::error!("{}: {:#x} not in {:?}", task_name, fn_hash, event_hashes);
                    continue;
                }
            };
            let event = &events[event_index];
            let record = self.parse_log_from_events(&log, event).await;

            event_writers[event_index].write_record(record).unwrap();

            event_writers[event_index].flush().unwrap();
        }
    }

    pub async fn parse_log_from_events(
        &self,
        log: &Log,
        event: &config_parser::Event,
    ) -> Vec<String> {
        let tx_result = self.provider.get_transaction(log.transaction_hash.unwrap());
        let receipt_result = self
            .provider
            .get_transaction_receipt(log.transaction_hash.unwrap());
        let (tx, receipt) = join!(tx_result, receipt_result);
        let tx = tx.unwrap().unwrap();
        let receipt = receipt.unwrap().unwrap();

        let mut record: Vec<String> = Vec::new();
        record.push(log.block_number.unwrap().to_string());
        record.push(format!("{:#x}", log.transaction_hash.unwrap()));
        record.push(format!("{:#x}", tx.from));
        record.push(match tx.to {
            None => "".to_owned(),
            Some(to) => format!("{:#x}", to),
        });
        record.push(tx.value.to_string());
        record.push(receipt.status.unwrap_or(U64::from(1)).as_u64().to_string());
        record.push(format!("{:#x}", log.address));

        if self.with_gas {
            record.push(receipt.gas_used.unwrap().to_string());
            record.push(
                tx.max_priority_fee_per_gas
                    .unwrap_or(tx.gas_price.unwrap())
                    .to_string(),
            );
            record.push(receipt.effective_gas_price.unwrap().to_string());
        }

        log::debug!(
            "{}: found event {} on tx: {:#x}",
            self.task_name,
            event.name,
            log.transaction_hash.unwrap()
        );

        assert!(
            event.topics.len() == log.topics.len() - 1,
            "{}.{}: config {} != log no fn {}",
            self.task_name,
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
        let raw_data = &log.data;
        let mut pos: usize = 0;
        let mut suffix: usize = 0;
        for param in &event.data {
            let evm_type = param.evm_type.as_str();
            let value = match evm_type {
                "address" => {
                    let raw = &raw_data[pos..pos + 32];
                    pos += 32;
                    format!("{:#x}", Address::from(H256::from_slice(raw)))
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
                    hex::encode(raw_bytes).to_string()
                }
                "bytes32" => {
                    let raw = &raw_data[pos..pos + 32];
                    pos += 32;
                    format!("{:#x}", H256::from_slice(raw))
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
            self.task_name,
            event.name,
            pos,
            raw_data.len()
        );

        record
    }

    pub async fn dump_logs_from_abi(&self, abi: Abi) {
        let event_signature_map: BTreeMap<_, _> =
            abi.events().map(|e| (e.signature(), e)).collect();

        let path = Path::new(&self.output);
        if !path.exists() {
            fs::create_dir_all(path).unwrap();
        }
        let mut event_writers: BTreeMap<_, Writer<File>> = abi
            .events()
            .map(|e| {
                let path = path.join(format!("{}_{}.csv", self.task_name, e.name));
                (
                    e.signature(),
                    csv::Writer::from_writer(File::create(path).unwrap()),
                )
            })
            .collect();

        for e in abi.events() {
            let mut fields = Vec::from(FIXED_FIELDS);
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
        log::warn!("{}: {:?}", self.task_name, event_signatures);
        let mut filter = Filter::new()
            .topic0(event_signatures)
            .address(self.addrs.to_vec())
            .from_block(self.start);

        if self.end != 0 {
            filter = filter.to_block(self.end);
        }

        let mut stream = self.provider.get_logs_paginated(&filter, 100);
        while let Some(log) = stream.next().await {
            let log = log.unwrap();
            // log::debug!("{:?}", log);
            let sig = &log.topics[0];
            let event = event_signature_map.get(sig).unwrap();

            let record = self.parse_log_from_abi(&log, event).await;

            event_writers
                .get_mut(sig)
                .unwrap()
                .write_record(record)
                .unwrap();

            event_writers.get_mut(sig).unwrap().flush().unwrap();
        }
    }

    pub async fn dump_logs_from_abi_mt(&self, abi: Abi) {
        let event_signature_map: BTreeMap<_, _> =
            abi.events().map(|e| (e.signature(), e.clone())).collect();
        let event_signature_map = Arc::new(event_signature_map);
        let path = Path::new(&self.output);
        if !path.exists() {
            fs::create_dir_all(path).unwrap();
        }
        let event_writers: BTreeMap<_, Writer<File>> = abi
            .events()
            .map(|e| {
                let path = path.join(format!("{}_{}.csv", self.task_name, e.name));
                (
                    e.signature(),
                    csv::Writer::from_writer(File::create(path).unwrap()),
                )
            })
            .collect();
        let event_writers = Arc::new(Mutex::new(event_writers));

        for e in abi.events() {
            let mut fields = Vec::from(FIXED_FIELDS);
            for param in &e.inputs {
                let name = &param.name;
                fields.push(name);
            }
            event_writers
                .lock()
                .unwrap()
                .get_mut(&e.signature())
                .unwrap()
                .write_record(fields)
                .unwrap();
        }

        let event_signatures: Vec<_> = abi.events().map(|e| e.signature()).collect();
        log::warn!("{}: {:?}", self.task_name, event_signatures);

        let mut handles: Vec<_> = vec![];
        for addr in self.addrs.clone() {
            let event_signatures = event_signatures.clone();
            let parser = self.clone();
            let event_writers = event_writers.clone();
            let event_signature_map = event_signature_map.clone();

            let start = self.start;
            let end = self.end;

            handles.push(tokio::spawn(async move {
                let mut filter = Filter::new()
                    .topic0(event_signatures)
                    .address(addr)
                    .from_block(start);

                if end != 0 {
                    filter = filter.to_block(end);
                }

                let mut stream = parser.provider.get_logs_paginated(&filter, 100);
                while let Some(log) = stream.next().await {
                    let log = log.unwrap();
                    // log::debug!("{:?}", log);
                    let sig = &log.topics[0];
                    let event = event_signature_map.get(sig).unwrap();

                    let record = parser.parse_log_from_abi(&log, event).await;

                    event_writers
                        .lock()
                        .unwrap()
                        .get_mut(sig)
                        .unwrap()
                        .write_record(record)
                        .unwrap();

                    event_writers
                        .lock()
                        .unwrap()
                        .get_mut(sig)
                        .unwrap()
                        .flush()
                        .unwrap();
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap()
        }
    }

    pub async fn parse_log_from_abi(&self, log: &Log, event: &Event) -> Vec<String> {
        let tx_result = self.provider.get_transaction(log.transaction_hash.unwrap());
        let receipt_result = self
            .provider
            .get_transaction_receipt(log.transaction_hash.unwrap());
        let (tx, receipt) = join!(tx_result, receipt_result);
        let tx = tx.unwrap().unwrap();
        let receipt = receipt.unwrap().unwrap();

        let mut record: Vec<String> = Vec::new();
        record.push(log.block_number.unwrap().to_string());
        record.push(format!("{:#x}", log.transaction_hash.unwrap()));
        record.push(format!("{:#x}", tx.from));
        record.push(match tx.to {
            None => "".to_owned(),
            Some(to) => format!("{:#x}", to),
        });
        record.push(tx.value.to_string());
        record.push(receipt.status.unwrap_or(U64::from(1)).as_u64().to_string());
        record.push(format!("{:#x}", log.address));

        if self.with_gas {
            record.push(receipt.gas_used.unwrap().to_string());
            record.push(
                tx.max_priority_fee_per_gas
                    .unwrap_or(tx.gas_price.unwrap())
                    .to_string(),
            );
            record.push(receipt.effective_gas_price.unwrap().to_string());
        }

        let raw_log = event
            .parse_log_whole(RawLog {
                topics: log.topics.to_vec(),
                data: log.data.to_vec(),
            })
            .unwrap();
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
        record
    }
}
