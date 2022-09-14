use std::{thread::{self, JoinHandle}, time, sync::{Arc, atomic::{AtomicBool, Ordering}, Mutex}, future::Future};

use indexmap::map::IndexMap;
use tokio::sync::mpsc::Receiver;
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
    // stop: Arc<AtomicBool>,
    account: Mutex<Account>,
    transaction_queues: rpc::TransactionQueues,
    block_queues: rpc::BlockQueues,
    blockchain: Mutex<Blockchain>,
    transaction_pool: Mutex<IndexMap<String, Arc<SignedTransaction>>>,
    //         self._transaction_pool = OrderedDict()
    //         self._transaction_pool_lock = threading.Lock()
    interrupt_mining_event: Arc<AtomicBool>,
    mine_thread: Option<JoinHandle<()>>,
    receive_new_transactions_thread: Option<JoinHandle<()>>,

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
            mine_thread: None,
            receive_new_transactions_thread: None,
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
//     def sync(self, address, peers):
//         if peers == []:
//             signed_blocks = [self._create_genesis_block()]
//         else:
//             signed_blocks = self._get_blocks_from_peers(peers, address)
//         for signed_block in signed_blocks:
//             self._append_block(signed_block)
//         for peer in peers:
//             self.add_peer(peer)

    pub fn run(this: Arc<Self>, mut txn_receiver: Receiver<SignedTransaction>) -> (impl Future<Output = ()>, JoinHandle<()>) {
        let task = {
            let this = this.clone();
            let stop = this.stop.clone();
            async move {
                while !stop.is_triggered() {
                    if let Some(txn) = txn_receiver.recv().await {
                        this.add_transaction(txn);
                    }
                }
            }
        };
        let thread_handle = {
            let stop = this.stop.clone();
            std::thread::spawn(move || {
                while !stop.is_triggered() {
                    this.mine();
                    thread::sleep(time::Duration::from_millis(100));
                }
            })
        };
        (task, thread_handle)
    }

    pub fn stop(&self) {
        self.transaction_queues.lock().unwrap().clear();
        self.block_queues.lock().unwrap().clear();
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
//     def _append_block(self, signed_block):
//         self._blockchain.append_block(signed_block)
//         from pprint import pprint
//         print("NEW BLOCK", signed_block.block.number)
//         pprint({address[:4]: balance for address, balance in self._blockchain._balances.items()})
//         with self._transaction_pool_lock:
//             for transaciton in signed_block.block.transactions:
//                 self._transaction_pool.pop(transaciton.hash, None)

    pub fn add_transaction(&self, txn: SignedTransaction) {
        self.transaction_queues.lock().unwrap().retain(|tx| {
            match tx.blocking_send(Ok(txn.pb())) {
                Ok(_) => true,
                Err(_) => false,
            }
        });
        self.add_to_transaction_pool(txn)
    }
//     def add_transaction(self, transaction):
//         for queue in self._transaction_queues:
//             queue.put(transaction)
//         self._add_to_transaction_pool(transaction)

    fn add_to_transaction_pool(&self, txn: SignedTransaction) {
        println!("NEW TRANSACTION {}", txn.hash());
        if txn.is_valid() {
            self.transaction_pool.lock().unwrap().insert(txn.hash(), Arc::new(txn));
        }
    }
// def _add_to_transaction_pool(self, signed_transaction):
//     print("NEW TRANSACTION", signed_transaction.transaction)
//     if signed_transaction.is_valid:
//         with self._transaction_pool_lock:
//             self._transaction_pool[signed_transaction.hash] = signed_transaction
//         return True
//     return False

    pub fn get_transaction(&self, transaction_hash: String) -> Option<Arc<SignedTransaction>> {
        match self.blockchain.lock().unwrap().get_transaciton(&transaction_hash) {
            Some(txn) => Some(txn),
            None => {
                match self.transaction_pool.lock().unwrap().get(&transaction_hash) {
                    Some(txn) => Some(txn.clone()),
                    None => None
                }
            }
        }

    }
//     def get_transaction(self, transaction_hash):
//         transaction = self._blockchain.get_transaction(transaction_hash)
//         if transaction is None:
//             return self._transaction_pool.get(transaction_hash)
//         else:
//             return transaction

    pub fn mine(&self) {
        self.interrupt_mining_event.store(false, Ordering::Relaxed);
        let mut next_block = self.form_next_block();
        let solution = next_block.find_solution(self.interrupt_mining_event.clone());
        if !self.interrupt_mining_event.load(Ordering::Relaxed) {
            println!("BLOCK FOUND {}", next_block.number());
            let mut signed_block = self.account.lock().unwrap().sign_block(next_block);
            signed_block.set_solution(solution);
            self.block_queues.lock().unwrap().retain(|tx| {
                match tx.blocking_send(Ok(signed_block.pb())) {
                    Ok(_) => true,
                    Err(_) => false,
                }
            });
            self.append_block(signed_block);
        }
    }
//     def mine(self):
//         while not self._stop:
//             self._interrupt_mining_event.clear()
//             next_block = self._form_next_block()
//             next_block.find_solution(self._interrupt_mining_event)
//             if not self._interrupt_mining_event.is_set():
//                 print("BLOCK_FOUND", next_block.number)
//                 signed_block = SignedBlock(next_block, self._account.sign_block(next_block))
//                 for queue in self._block_queues:
//                     queue.put(signed_block)
//                 self._append_block(signed_block)

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
        // for (_, tx) in transaction_pool.iter() {
        //     if blockchain.is_transaction_valid(&tx, &next_block) {
        //         next_block.append_transaction(tx.clone());
        //     } else {
        //         println!("Invalid transaction {}", tx.hash());
        //         // transaction_pool.remove(&tx.hash());
        //     }
        // }
        next_block
    }
//     def _form_next_block(self):
//         reward = Transaction(None, self._account.address, 100, "")
//         signed_reward = SignedTransaction(reward, self._account.sign_transaction(reward))
//         next_block = Block(
//             self._blockchain.last_block.number + 1,
//             self._blockchain.last_block.hash,
//             DIFFICULTY,
//             signed_reward
//         )
//         with self._transaction_pool_lock:
//             for transaction in list(self._transaction_pool.values()):
//                 if self._blockchain.is_transaction_valid(transaction, next_block):
//                     next_block.append_transaction(transaction)
//                 else:
//                     print("Invalid transaction", transaction.hash)
//                     self._transaction_pool.pop(transaction.hash)
//         return next_block

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

// class Runtime:
//     def __init__(self, account, transaction_queues, block_queues):
//         self._stop = False
//         self._account = account
//         self._transaction_queues = transaction_queues
//         self._block_queues = block_queues
//         self._blockchain = BlockChain()
//         self._transaction_pool = OrderedDict()
//         self._transaction_pool_lock = threading.Lock()
//         self._interrupt_mining_event = threading.Event()
//         self._peer_threads = {}


//     @property
//     def blockchain(self):
//         return self._blockchain

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