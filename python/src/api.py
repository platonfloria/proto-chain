from fastapi import FastAPI
from pydantic import BaseModel

import transaction


class Api(FastAPI):
    def set_runtime(self, runtime):
        self._runtime = runtime

    @property
    def runtime(self):
        return self._runtime


api = Api()


class Transaction(BaseModel):
    nonce: int
    origin: str
    destination: str
    amount: int
    data: str


class SignedTransaction(BaseModel):
    transaction: Transaction
    signature: str


@api.get("/block_count")
def get_block_count():
    return api.runtime.blockchain.last_block.number


@api.get("/block/{block_number}")
def get_block_transactions(block_number: int):
    signed_block = api.runtime.blockchain.get_block(block_number)
    return [t.dict for t in signed_block.block.transactions]


@api.get("/transaction/{transaction_hash}")
def get_transaciton(transaction_hash: str):
    transaction = api.runtime.get_transaction(transaction_hash)
    if transaction is not None:
        return transaction.dict


@api.post("/transaction")
def post_transaciton(signed_transaction: SignedTransaction):
    txn = transaction.SignedTransaction.from_dict(signed_transaction.dict())
    return api.runtime.add_transaction(txn), txn.hash
