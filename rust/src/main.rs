use core::time;
use std::{collections::HashMap, sync::{Arc, Mutex}, thread};

use clap::Parser;
use k256::{ecdsa::{VerifyingKey, signature::Verifier, Signature, signature::Signer, signature::Signature as _, SigningKey}, Secp256k1};
use serde::Deserialize;
use serde_yaml;
use tokio::{runtime::Builder, sync::mpsc};

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
    let node = &config["nodes"][0];
    let account = Account::new(&node.account);
    let mut peers: Vec<String> = vec![];
    if !args.peers.is_empty() {
        peers.extend(args.peers.iter().map(|&p| format!(
            "{}:{}", config["nodes"][p].ipaddress, config["nodes"][p].rpc_port
        )).collect::<Vec<String>>());
    }
    println!("{:?}", peers);

    let transaction_queues = Arc::new(Mutex::new(Vec::new()));
    let block_queues = Arc::new(Mutex::new(Vec::new()));

    let (stop_trigger, stop_listener) = triggered::trigger();

    let (txn_sender, mut txn_receiver) = mpsc::channel(1);
    let mut runtime = Runtime::new(account, transaction_queues.clone(), block_queues.clone(), stop_listener.clone());
    runtime.sync(format!("{}:{}", node.ipaddress, node.rpc_port), peers);

    let runtime = Arc::new(runtime);
    let (runtime_task, runtime_thread) = Runtime::run(
        runtime.clone(),
        txn_receiver,
        // stop_listener.clone()
    );

    let rpc_server = rpc::RPC::new(&node.rpc_port.to_string(), transaction_queues.clone(), block_queues.clone(), stop_listener.clone());
    let api_server = api::API::new(&node.api_port.to_string(), runtime.clone(), txn_sender, stop_listener.clone());

    let async_runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();


    // let runtime = Arc::new(Mutex::new(runtime));
    let async_thread = {
        // let runtime = runtime.clone();
        // let stop_listener = stop_listener.clone();
        std::thread::spawn(move || {
            async_runtime.block_on(async move {
                tokio::join!(
                    rpc_server.start(),
                    api_server.start(),
                    runtime_task,
                    // {
                    //     let stop_trigger = stop_trigger.clone();
                    //     tokio::task::spawn(async move {
                    //         while !stop_trigger.is_triggered() {
                    //             if let Some(txn) = txn_receiver.recv().await {
                    //                 runtime.lock().unwrap().add_transaction(txn);
                    //             }
                    //         }
                    //     })
                    // },
                    tokio::task::spawn(async move {
                        tokio::signal::ctrl_c()
                            .await
                            .expect("failed to install CTRL+C signal handler");
                        println!("received Ctrl+C!");
                        stop_trigger.trigger();
                        runtime.stop();
                    }),
                );
            });
        })
    };

    // let runtime_thread = std::thread::spawn(move || {
    //     while !stop_listener.is_triggered() {
    //         runtime.lock().unwrap().mine();
    //         thread::sleep(time::Duration::from_millis(100));
    //     }
    // });

    println!("Hello, world!");
    async_thread.join();
    runtime_thread.join();
    Ok(())
}
