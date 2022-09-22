use std::{thread::{self, JoinHandle}, time, sync::{Arc, atomic::{AtomicBool, Ordering}, Mutex}, future::Future};

use futures::{future, FutureExt};
use indexmap::map::IndexMap;
use tokio::{runtime::Runtime as AsyncRuntime, sync::{Mutex as AsyncMutex, mpsc::{Receiver, Sender}}};
use tonic::Status;
use triggered::Listener;

use crate::{
    block::{Block, SignedBlock},
    blockchain::Blockchain,
    transaction::{Transaction, SignedTransaction},
    rpc,
    Account
};


const DIFFICULTY: u32 = 16;


pub struct Runtime {
    stop: Listener,
    account: Mutex<Account>,
    async_runtime: Arc<AsyncRuntime>,
    transaction_queues: rpc::TransactionQueues,
    block_queues: rpc::BlockQueues,
    blockchain: Mutex<Blockchain>,
    transaction_pool: Mutex<IndexMap<String, Arc<SignedTransaction>>>,
    interrupt_mining_event: AtomicBool,
    block_sender: Arc<AsyncMutex<Option<rpc::BlockChannel>>>,
    transaction_sender: Arc<AsyncMutex<Option<rpc::TransactionChannel>>>,
}

impl Runtime {
    pub fn new(
        account: Account,
        async_runtime: Arc<AsyncRuntime>,
        transaction_queues: rpc::TransactionQueues,
        block_queues: rpc::BlockQueues,
        transaction_sender: Sender<Result<rpc::SignedTransaction, Status>>,
        block_sender: Sender<Result<rpc::SignedBlock, Status>>,
        stop: Listener,
    ) -> Self {
        Self {
            stop,
            account: Mutex::new(account),
            async_runtime,
            transaction_queues,
            block_queues,
            blockchain: Mutex::new(Blockchain::new()),
            transaction_pool: Mutex::new(IndexMap::new()),
            interrupt_mining_event: AtomicBool::new(false),
            transaction_sender: Arc::new(AsyncMutex::new(Some(transaction_sender))),
            block_sender: Arc::new(AsyncMutex::new(Some(block_sender))),
        }
    }

    pub fn blockchain(&self) -> &Mutex<Blockchain> {
        &self.blockchain
    }

    pub fn sync(&self, address: &str, peers: &[String]) {
        let mut signed_blocks = Vec::new();
        if peers.is_empty() {
            signed_blocks.push(self.create_genesis_block())
        } else {
            let peers = peers.clone();
            signed_blocks.extend(self.async_runtime.block_on(async move {
                rpc::get_blocks_from_peers(peers, address).await.unwrap()
            }));
        }
        for signed_block in signed_blocks.into_iter() {
            self.append_block(signed_block)
        }
        for peer in peers.iter() {
            self.add_peer(peer)
        }
    }

    pub fn get_task(
        self: Arc<Self>,
        mut txn_receiver: Receiver<SignedTransaction>,
        mut transaction_receiver: Receiver<Result<rpc::SignedTransaction, Status>>,
        mut block_receiver: Receiver<Result<rpc::SignedBlock, Status>>
    ) -> impl Future<Output = Vec<()>> {
        future::join_all(vec![
            {
                let this = self.clone();
                async move {
                    while !this.stop.is_triggered() {
                        if let Some(txn) = txn_receiver.recv().await {
                            this.add_transaction(txn).await;
                        }
                    }
                }
            }.boxed(),
            {
                let this = self.clone();
                async move {
                    while !this.stop.is_triggered() {
                        if let Some(txn) = transaction_receiver.recv().await {
                            this.add_to_transaction_pool(SignedTransaction::from_pb(&txn.unwrap()));
                        }
                    }
                }
            }.boxed(),
            async move {
                while !self.stop.is_triggered() {
                    if let Some(block) = block_receiver.recv().await {
                        self.append_block(SignedBlock::from_pb(&block.unwrap()));
                        self.interrupt_mining_event.store(true, Ordering::Relaxed)
                    }
                }
            }.boxed()
        ])
    }
    
    pub fn run(self: Arc<Self>) -> JoinHandle<()> {
        std::thread::spawn(move || {
            while !self.stop.is_triggered() {
                self.mine();
                thread::sleep(time::Duration::from_millis(100));
            }
        })
    }

    pub async fn stop(&self) {
        self.transaction_queues.lock().await.clear();
        self.block_queues.lock().await.clear();
        self.interrupt_mining_event.store(true, Ordering::Relaxed);
        self.transaction_sender.lock().await.take();
        self.block_sender.lock().await.take();
    }

    fn create_genesis_block(&self) -> SignedBlock {
        let mut account = self.account.lock().unwrap();
        let reward = Transaction::new(
            None,
            account.get_address(),
            100,
            String::new()
        );
        let signed_reward = account.sign_transaction(reward);
        let block = Block::new(0, None, DIFFICULTY, signed_reward);
        let signed_block = account.sign_block(block);
        signed_block
    }

    fn append_block(&self, signed_block: SignedBlock) {
        println!("NEW BLOCK {}", signed_block.block().number());
        let mut transaction_pool = self.transaction_pool.lock().unwrap();
        for tx in signed_block.block().transactions() {
            transaction_pool.remove(&tx.hash());
        }
        self.blockchain.lock().unwrap().append_block(signed_block);
        for (address, balance) in self.blockchain.lock().unwrap().balances() {
            println!("{}: {}", &address[..4], balance);
        }
    }

    pub async fn add_transaction(&self, txn: SignedTransaction) {
        let mut remaining = Vec::new();
        let mut transaction_queues = self.transaction_queues.lock().await;
        for tx in transaction_queues.drain(..) {
            println!("{}", tx.is_closed());
            if let Ok(_) = tx.send(Ok(txn.pb())).await {
                remaining.push(tx);
            }
        }
        transaction_queues.extend(remaining);
        self.add_to_transaction_pool(txn)
    }

    fn add_to_transaction_pool(&self, txn: SignedTransaction) {
        println!("NEW TRANSACTION {}", txn.hash());
        if txn.is_valid() {
            self.transaction_pool.lock().unwrap().insert(txn.hash(), Arc::new(txn));
        }
    }

    pub fn get_transaction(&self, transaction_hash: String) -> Option<Arc<SignedTransaction>> {
        match self.blockchain.lock().unwrap().get_transaciton(&transaction_hash) {
            Some(txn) => Some(txn.clone()),
            None => {
                match self.transaction_pool.lock().unwrap().get(&transaction_hash) {
                    Some(txn) => Some(txn.clone()),
                    None => None
                }
            }
        }
    }

    pub fn mine(&self) {
        self.interrupt_mining_event.store(false, Ordering::Relaxed);
        let mut next_block = self.form_next_block();
        let solution = next_block.find_solution(&self.interrupt_mining_event);
        if !self.interrupt_mining_event.load(Ordering::Relaxed) {
            println!("BLOCK FOUND {}", next_block.number());
            let mut signed_block = self.account.lock().unwrap().sign_block(next_block);
            signed_block.set_solution(solution);

            {
                let block_queues = self.block_queues.clone();
                let signed_block = signed_block.pb();
                self.async_runtime.spawn(async move {
                    let mut remaining = Vec::new();
                    let mut block_queues = block_queues.lock().await;
                    for tx in block_queues.drain(..) {
                        println!("{}", tx.is_closed());
                        if let Ok(_) = tx.send(Ok(signed_block.clone())).await {
                            remaining.push(tx);
                        }
                    }
                    block_queues.extend(remaining);
                });
            }
            self.append_block(signed_block);
        }
    }

    fn form_next_block(&self) -> Block {
        let mut account = self.account.lock().unwrap();
        let reward = Transaction::new(
            None,
            account.get_address(),
            100,
            String::new()
        );
        let signed_reward = account.sign_transaction(reward);
        let blockchain = self.blockchain.lock().unwrap();
        let last_block = blockchain.get_last_block().unwrap();
        let mut next_block = Block::new(
            last_block.block().number() + 1,
            Some(last_block.hash()),
            DIFFICULTY,
            signed_reward
        );
        self.transaction_pool.lock().unwrap().retain(|_, tx| {
            if blockchain.is_transaction_valid(&tx, &next_block) {
                next_block.append_transaction(tx.clone());
                true
            } else {
                println!("Invalid transaction {}", tx.hash());
                false
            }
        });
        next_block
    }

    pub fn add_peer(&self, peer: &str) {
        println!("ADD PEER {}", peer);
        let peer = peer.to_owned();
        let transaction_sender = self.transaction_sender.clone();
        let block_sender = self.block_sender.clone();
        {
            let peer = peer.clone();
            self.async_runtime.spawn(async move {
                rpc::listen_to_new_transactions_from_peer(&peer, &transaction_sender).await;
            });
        }
        self.async_runtime.spawn(async move {
            rpc::listen_to_new_blocks_from_peer(&peer, &block_sender).await;
        });
    }
}