use std::{collections::HashMap, sync::Arc};

use clap::Parser;
use serde::Deserialize;
use serde_yaml;
use tokio::{runtime::Builder, sync::{mpsc, Mutex}};

mod account;
mod block;
mod blockchain;
mod transaction;
mod runtime;
mod api;
mod rpc;

use account::Account;
use runtime::Runtime;


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
   #[clap(short, long)]
   node: usize,

   #[clap(short, long, use_value_delimiter = true, value_delimiter = ',')]
   peers: Vec<usize>,
}


#[derive(Deserialize)]
struct NodeConfig {
    pub ipaddress: String,
    pub api_port: u32,
    pub rpc_port: u32,
    pub account: String,
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("{:?}", args);

    let f = std::fs::File::open("../config.yaml")?;
    let config: HashMap<String, Vec<NodeConfig>>  = serde_yaml::from_reader(f)?;
    let node_ipaddress = config["nodes"][args.node].ipaddress.to_owned();
    let node_api_port = config["nodes"][args.node].api_port;
    let node_rpc_port = config["nodes"][args.node].rpc_port;
    let node_account = config["nodes"][args.node].account.to_owned();
    let account = Account::new(&node_account);
    let mut peers: Vec<String> = vec![];
    if !args.peers.is_empty() {
        peers.extend(args.peers.iter().map(|&p| format!(
            "{}:{}", config["nodes"][p].ipaddress, config["nodes"][p].rpc_port
        )).collect::<Vec<String>>());
    }
    println!("{:?}", peers);


    let async_runtime = Arc::new(Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap());
    let transaction_queues = Arc::new(Mutex::new(Vec::new()));
    let block_queues = Arc::new(Mutex::new(Vec::new()));
    let (stop_trigger, stop_listener) = triggered::trigger();
    let (txn_sender, txn_receiver) = mpsc::channel(1);

    let runtime = Runtime::new(account, async_runtime.clone(), transaction_queues.clone(), block_queues.clone(), stop_listener.clone());
    let runtime = Arc::new(runtime);
    {
        let runtime = runtime.clone();
        async_runtime.block_on(async move {
            runtime.sync(format!("{}:{}", node_ipaddress, node_rpc_port), peers).await;
        });
    }

    let (runtime_task, runtime_thread) = Runtime::run(
        runtime.clone(),
        txn_receiver,
    );

    let rpc_server = rpc::RPC::new(&node_rpc_port, runtime.clone(), transaction_queues.clone(), block_queues.clone(), stop_listener.clone());
    let api_server = api::API::new(&node_api_port, runtime.clone(), txn_sender, stop_listener.clone());

    let async_thread = {
        std::thread::spawn(move || {
            async_runtime.block_on(async move {
                tokio::join!(
                    rpc_server.start(),
                    api_server.start(),
                    runtime_task,
                    tokio::task::spawn(async move {
                        tokio::signal::ctrl_c()
                            .await
                            .expect("failed to install CTRL+C signal handler");
                        println!("received Ctrl+C!");
                        stop_trigger.trigger();
                        runtime.stop().await;
                    }),
                );
            });
        })
    };

    println!("Hello, world!");
    async_thread.join();
    runtime_thread.join();
    Ok(())
}
