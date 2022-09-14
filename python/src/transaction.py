from google.protobuf import json_format
from hashlib import sha256

from ecdsa import VerifyingKey, SECP256k1

import rpc_pb2


class Transaction:
    def __init__(self, origin, destination, amount, data):
        self._origin = origin
        self._destination = destination
        self._amount = amount
        self._data = data
        self._nonce = None

    def set_nonce(self, nonce):
        self._nonce = nonce

    @property
    def hash(self):
        m = sha256()
        if self._origin is not None:
            m.update(self._origin.encode())
        m.update(self._destination.encode())
        m.update(self._amount.to_bytes(length=4, byteorder='big'))
        m.update(self._data.encode())
        if self._nonce is not None:
            m.update(self._nonce.to_bytes(length=4, byteorder='big'))
        return m.hexdigest()

    @property
    def pb(self):
        return rpc_pb2.Transaction(
            origin = self._origin,
            destination = self._destination,
            amount = self._amount,
            data = self._data,
            nonce = self._nonce
        )

    @classmethod
    def from_pb(cls, pb):
        inst = cls(
            origin=pb.origin,
            destination=pb.destination,
            amount=pb.amount,
            data=pb.data,
        )
        inst.set_nonce(pb.nonce)
        return inst

    @property
    def origin(self):
        return self._origin

    @property
    def destination(self):
        return self._destination

    @property
    def amount(self):
        return self._amount

    def __repr__(self):
        return f"From {self._origin[:4]} to {self._destination[:4]} {self._amount}"


class SignedTransaction:
    def __init__(self, transaction, signature):
        self._transaction = transaction
        self._signature = signature

    @property
    def transaction(self):
        return self._transaction

    @property
    def hash(self):
        m = sha256()
        m.update(self._transaction.hash.encode())
        m.update(self._signature.encode())
        return m.hexdigest()

    @property
    def dict(self):
        txn_pb = self.pb
        txn_dict = json_format.MessageToDict(txn_pb)
        if txn_pb.transaction.nonce == 0:
            txn_dict['transaction']['nonce'] = 0
        if txn_pb.transaction.data == '':
            txn_dict['transaction']['data'] = ''
        return txn_dict

    @classmethod
    def from_dict(cls, dict):
        txn_pb = rpc_pb2.SignedTransaction()
        json_format.ParseDict(dict, txn_pb)
        return cls.from_pb(txn_pb)

    @property
    def pb(self):
        return rpc_pb2.SignedTransaction(
            transaction=self._transaction.pb,
            signature=self._signature
        )

    @classmethod
    def from_pb(cls, pb):
        return cls(
            transaction=Transaction.from_pb(pb.transaction),
            signature=pb.signature
        )

    @property
    def is_valid(self):
        vk = VerifyingKey.from_string(bytes.fromhex(self._transaction.origin), SECP256k1, hashfunc=sha256)
        return vk.verify(bytes.fromhex(self._signature), bytes.fromhex(self._transaction.hash))
