use std::{thread::{self, JoinHandle}, time, sync::{Arc, atomic::{AtomicBool, Ordering}, Mutex}, future::Future};

use indexmap::map::IndexMap;
use tokio::{runtime::Runtime as AsyncRuntime, sync::mpsc::Receiver};
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
    transaction_queues: rpc::TransactionQueues,
    block_queues: rpc::BlockQueues,
    blockchain: Mutex<Blockchain>,
    transaction_pool: Mutex<IndexMap<String, Arc<SignedTransaction>>>,
    interrupt_mining_event: Arc<AtomicBool>,
    //         self._peer_threads = {}
}

impl Runtime {
    pub fn new(
        account: Account,
        transaction_queues: rpc::TransactionQueues,
        block_queues: rpc::BlockQueues,
        stop: Listener,
    ) -> Self {
        Self {
            stop,
            account: Mutex::new(account),
            transaction_queues,
            block_queues,
            blockchain: Mutex::new(Blockchain::new()),
            transaction_pool: Mutex::new(IndexMap::new()),
            interrupt_mining_event: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn blockchain(&self) -> &Mutex<Blockchain> {
        &self.blockchain
    }

    pub fn sync(&mut self, address: String, peers: Vec<String>) {
        let mut signed_blocks = Vec::new();
        if peers.is_empty() {
            signed_blocks.push(self.create_genesis_block())
        } else {
            signed_blocks.extend(self.get_blocks_from_peers(&peers, &address))
        }
        for signed_block in signed_blocks.into_iter() {
            self.append_block(signed_block)
        }
        for peer in peers.iter() {
            self.add_peer(peer)
        }
    }

    pub fn run(self: Arc<Self>, async_runtime: Arc<AsyncRuntime>, mut txn_receiver: Receiver<SignedTransaction>) -> (impl Future<Output = ()>, JoinHandle<()>) {
        let task = {
            let this = self.clone();
            let stop = self.stop.clone();
            async move {
                while !stop.is_triggered() {
                    if let Some(txn) = txn_receiver.recv().await {
                        this.add_transaction(txn).await;
                    }
                }
            }
        };
        let thread_handle = {
            let stop = self.stop.clone();
            std::thread::spawn(move || {
                while !stop.is_triggered() {
                    self.mine(async_runtime.clone());
                    thread::sleep(time::Duration::from_millis(100));
                }
            })
        };
        (task, thread_handle)
    }

    pub async fn stop(&self) {
        self.transaction_queues.lock().await.clear();
        self.block_queues.lock().await.clear();
        self.interrupt_mining_event.store(true, Ordering::Relaxed);
    }
//     def stop(self):
//         self._stop = True
//         self._interrupt_mining_event.set()
//         for peer_threads in self._peer_threads.values():
//             for future in peer_threads['futures']:
//                 future.cancel()
//             for thread in peer_threads['threads']:
//                 thread.join()

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
        for (address, balance) in self.blockchain.lock().unwrap().balances() {
            println!("{}: {}", &address[..4], balance);
        }
        let mut transaction_pool = self.transaction_pool.lock().unwrap();
        for tx in signed_block.block().transactions() {
            transaction_pool.remove(&tx.hash());
        }
        self.blockchain.lock().unwrap().append_block(signed_block);
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

    pub fn mine(&self, async_runtime: Arc<AsyncRuntime>) {
        self.interrupt_mining_event.store(false, Ordering::Relaxed);
        let mut next_block = self.form_next_block();
        let solution = next_block.find_solution(self.interrupt_mining_event.clone());
        if !self.interrupt_mining_event.load(Ordering::Relaxed) {
            println!("BLOCK FOUND {}", next_block.number());
            let mut signed_block = self.account.lock().unwrap().sign_block(next_block);
            signed_block.set_solution(solution);

            {
                let block_queues = self.block_queues.clone();
                let signed_block = signed_block.pb();
                async_runtime.spawn(async move {
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

    }
//     def add_peer(self, peer):
//         print("ADD PEER", peer)
//         if peer not in self._peer_threads:
//             self._peer_threads[peer] = {
//                 'threads': [
//                     threading.Thread(target=self._listen_to_new_blocks_from_peer, args=(peer,)),
//                     threading.Thread(target=self._listen_to_new_transactions_from_peer, args=(peer,))
//                 ],
//                 'futures': []
//             }
//             for thread in self._peer_threads[peer]['threads']:
//                 thread.start()

    fn get_blocks_from_peers(&self, peers: &Vec<String>, address: &str) -> Vec<SignedBlock> {
        Vec::new()
    }
//     def _get_blocks_from_peers(self, peers, address):
//         for peer in peers:
//             with grpc.insecure_channel(peer) as channel:
//                 stub = rpc_pb2_grpc.RPCStub(channel)
//                 blocks = [
//                     SignedBlock.from_pb(block) for block in stub.Sync(rpc_pb2.SyncRequest(address=address)).blocks
//                 ]
//         return blocks

}

//     def _listen_to_new_blocks_from_peer(self, address):
//         with grpc.insecure_channel(address) as channel:
//             stub = rpc_pb2_grpc.RPCStub(channel)
//             future = stub.BlockFeed(rpc_pb2.BlockFeedRequest())
//             self._peer_threads[address]['futures'].append(future)
//             for block in future:
//                 self._append_block(SignedBlock.from_pb(block))
//                 self._interrupt_mining_event.set()

//     def _listen_to_new_transactions_from_peer(self, address):
//         with grpc.insecure_channel(address) as channel:
//             stub = rpc_pb2_grpc.RPCStub(channel)
//             future = stub.TransactionFeed(rpc_pb2.TransactionFeedRequest())
//             self._peer_threads[address]['futures'].append(future)
//             for transaction in future:
//                 self._add_to_transaction_pool(SignedTransaction.from_pb(transaction))