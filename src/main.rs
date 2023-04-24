mod config_parser;
mod csv_output;
mod log_parser;

use clap::Parser;
use ethers::providers::Provider;
use ethers::providers::{Middleware, Ws};
use std::fmt::Debug;

use crate::log_parser::LogParser;

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
        let addrs = task.contracts.clone();
        let task_name = task.name.clone();
        let output = args.output.clone();

        let log_parser = LogParser {
            provider,
            addrs,
            output,
            task_name,
        };

        handles.push(tokio::spawn(async move {
            let task_name = task.name.clone();
            log::warn!("Start task {}", task_name);
            match task.abi {
                None => {
                    log_parser
                        .dump_logs_from_events(task.simple_events.to_vec())
                        .await;
                }
                Some(abi) => {
                    if args.parallel {
                        log_parser.dump_logs_from_abi_mt(abi).await;
                    } else {
                        log_parser.dump_logs_from_abi(abi).await;
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap()
    }
}
