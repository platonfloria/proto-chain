import fire
import grpc
import requests

import rpc_pb2, rpc_pb2_grpc

from account import Account
from transaction import Transaction


def main(method, data=None):
    account1 = Account("b129bdb55ff1dba59fbeb30a6aa774622bab3eedb1c71333262598af439006cd")
    account2 = Account("544d8df8088c017b02fbc6ea5a08b40df6b4da24ebb1a6f980ca3b14948dae57")
    account3 = Account("685e7572401dc2229533f8c3bdd3ed404407610df4d0f53ea71802094fd87b51")
    account4 = Account("447755f4a124f405e4908626346e97578ada078835eb49a6351073bba00dc6b4")

    if method == 'transact':
        txn = Transaction(account1.address, account2.address, 50, "")
        signed_txn = account1.sign_transaction(txn)
        requests.post('http://localhost:8000/transaction', json=signed_txn.dict)
        print(signed_txn.hash)

    elif method == 'sequence':
        txn = Transaction(account2.address, account3.address, 50, "")
        signed_txn = account2.sign_transaction(txn)
        requests.post('http://localhost:8000/transaction', json=signed_txn.dict)

        txn = Transaction(account3.address, account4.address, 50, "")
        signed_txn = account3.sign_transaction(txn)
        requests.post('http://localhost:8000/transaction', json=signed_txn.dict)

    elif method == 'double_spend':
        txn = Transaction(account2.address, account3.address, 50, "")
        signed_txn = account2.sign_transaction(txn)
        requests.post('http://localhost:8000/transaction', json=signed_txn.dict)

        txn = Transaction(account2.address, account4.address, 50, "")
        signed_txn = account2.sign_transaction(txn)
        requests.post('http://localhost:8000/transaction', json=signed_txn.dict)

    elif method == 'get':
        response = requests.get(f'http://localhost:8000/transaction/{data}')
        print(response.json())

    elif method == 'transaction_feed':
        with grpc.insecure_channel('localhost:50051') as channel:
            stub = rpc_pb2_grpc.RPCStub(channel)
            for transaction in stub.TransactionFeed(rpc_pb2.TransactionFeedRequest()):
                print(transaction)

    elif method == 'block_feed':
        with grpc.insecure_channel('localhost:50051') as channel:
            stub = rpc_pb2_grpc.RPCStub(channel)
            for block in stub.BlockFeed(rpc_pb2.BlockFeedRequest()):
                print(block)

    elif method == 'sync':
        with grpc.insecure_channel('localhost:50051') as channel:
            stub = rpc_pb2_grpc.RPCStub(channel)
            print(stub.Sync(rpc_pb2.SyncRequest()))


if __name__ == "__main__":
    fire.Fire(main)
