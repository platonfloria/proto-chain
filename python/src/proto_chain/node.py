import threading
from collections import OrderedDict

import fire
import grpc
import uvicorn
import yaml

import rpc_pb2, rpc_pb2_grpc

from proto_chain import rpc
from proto_chain.api import api
from proto_chain.account import Account
from proto_chain.block import Block, SignedBlock
from proto_chain.blockchain import BlockChain
from proto_chain.transaction import Transaction, SignedTransaction


DIFFICULTY = 21


class Runtime:
    def __init__(self, account, transaction_queues, block_queues):
        self._stop = False
        self._account = account
        self._transaction_queues = transaction_queues
        self._block_queues = block_queues
        self._blockchain = BlockChain()
        self._transaction_pool = OrderedDict()
        self._transaction_pool_lock = threading.Lock()
        self._interrupt_mining_event = threading.Event()
        self._peer_threads = {}

    def sync(self, address, peers):
        if peers == []:
            signed_blocks = [self._create_genesis_block()]
        else:
            signed_blocks = self._get_blocks_from_peers(peers, address)
        for signed_block in signed_blocks:
            self._append_block(signed_block)
        for peer in peers:
            self.add_peer(peer)

    @property
    def blockchain(self):
        return self._blockchain

    def get_transaction(self, transaction_hash):
        transaction = self._blockchain.get_transaction(transaction_hash)
        if transaction is None:
            return self._transaction_pool.get(transaction_hash)
        else:
            return transaction

    def add_transaction(self, transaction):
        for queue in self._transaction_queues:
            queue.put(transaction)
        self._add_to_transaction_pool(transaction)

    def add_peer(self, peer):
        print("ADD PEER", peer)
        if peer not in self._peer_threads:
            self._peer_threads[peer] = {
                'threads': [
                    threading.Thread(target=self._listen_to_new_blocks_from_peer, args=(peer,)),
                    threading.Thread(target=self._listen_to_new_transactions_from_peer, args=(peer,))
                ],
                'futures': []
            }
            for thread in self._peer_threads[peer]['threads']:
                thread.start()

    def mine(self):
        while not self._stop:
            self._interrupt_mining_event.clear()
            next_block = self._form_next_block()
            solution = next_block.find_solution(self._interrupt_mining_event)
            if not self._interrupt_mining_event.is_set():
                print("BLOCK_FOUND", next_block.number)
                signed_block = self._account.sign_block(next_block)
                signed_block.set_solution(solution)
                for queue in self._block_queues:
                    queue.put(signed_block)
                self._append_block(signed_block)

    def stop(self):
        self._stop = True
        self._interrupt_mining_event.set()
        for peer_threads in self._peer_threads.values():
            for future in peer_threads['futures']:
                future.cancel()
            for thread in peer_threads['threads']:
                thread.join()

    def _add_to_transaction_pool(self, signed_transaction):
        print("NEW TRANSACTION", signed_transaction.transaction)
        if signed_transaction.is_valid:
            with self._transaction_pool_lock:
                self._transaction_pool[signed_transaction.hash] = signed_transaction
            return True
        return False

    def _create_genesis_block(self):
        reward = Transaction(None, self._account.address, 100, "")
        signed_reward = self._account.sign_transaction(reward)
        block = Block(0, None, DIFFICULTY, signed_reward)
        signed_block = self._account.sign_block(block)
        return signed_block

    def _form_next_block(self):
        reward = Transaction(None, self._account.address, 100, "")
        signed_reward = self._account.sign_transaction(reward)
        next_block = Block(
            self._blockchain.last_block.block.number + 1,
            self._blockchain.last_block.hash,
            DIFFICULTY,
            signed_reward
        )
        with self._transaction_pool_lock:
            for transaction in list(self._transaction_pool.values()):
                if self._blockchain.is_transaction_valid(transaction, next_block):
                    next_block.append_transaction(transaction)
                else:
                    print("Invalid transaction", transaction.hash)
                    self._transaction_pool.pop(transaction.hash)
        return next_block

    def _append_block(self, signed_block):
        self._blockchain.append_block(signed_block)
        from pprint import pprint
        print("NEW BLOCK", signed_block.block.number)
        pprint({address[:4]: balance for address, balance in self._blockchain._balances.items()})
        with self._transaction_pool_lock:
            for transaciton in signed_block.block.transactions:
                self._transaction_pool.pop(transaciton.hash, None)

    def _get_blocks_from_peers(self, peers, address):
        for peer in peers:
            with grpc.insecure_channel(peer) as channel:
                stub = rpc_pb2_grpc.RPCStub(channel)
                blocks = [
                    SignedBlock.from_pb(block) for block in stub.Sync(rpc_pb2.SyncRequest(address=address)).blocks
                ]
        return blocks

    def _listen_to_new_blocks_from_peer(self, address):
        with grpc.insecure_channel(address) as channel:
            stub = rpc_pb2_grpc.RPCStub(channel)
            future = stub.BlockFeed(rpc_pb2.BlockFeedRequest())
            self._peer_threads[address]['futures'].append(future)
            for block in future:
                self._append_block(SignedBlock.from_pb(block))
                self._interrupt_mining_event.set()

    def _listen_to_new_transactions_from_peer(self, address):
        with grpc.insecure_channel(address) as channel:
            stub = rpc_pb2_grpc.RPCStub(channel)
            future = stub.TransactionFeed(rpc_pb2.TransactionFeedRequest())
            self._peer_threads[address]['futures'].append(future)
            for transaction in future:
                self._add_to_transaction_pool(SignedTransaction.from_pb(transaction))


def main(node, peers: list=None):
    with open('../config.yaml', 'r') as f:
        config = yaml.safe_load(f)
    node = config['nodes'][node]
    account = Account(node['account'])

    transaction_queues = []
    block_queues = []
    if peers is None:
        peers = []
    else:
        peers = [f"localhost:{config['nodes'][node]['rpc_port']}" for node in peers]
    runtime = Runtime(account, transaction_queues, block_queues)
    api.set_runtime(runtime)
    stop_event = threading.Event()
    rpc_server = rpc.get_server(node['rpc_port'], runtime, transaction_queues, block_queues, stop_event)
    rpc_thread = threading.Thread(target=rpc_server.start)
    rpc_thread.start()

    runtime.sync(f"{node['ipaddress']}:{node['rpc_port']}", peers)
    runtime_thread = threading.Thread(target=runtime.mine)
    runtime_thread.start()

    uvicorn.run(api, host="0.0.0.0", port=node['api_port'])

    runtime.stop()
    stop_event.set()
    rpc_server.stop(None)
    rpc_server.wait_for_termination()
    runtime_thread.join()
    rpc_thread.join()


if __name__ == "__main__":
    fire.Fire(main)
