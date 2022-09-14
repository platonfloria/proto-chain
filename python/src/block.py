from functools import cached_property
from hashlib import sha256

from ecdsa import VerifyingKey, SECP256k1

import rpc_pb2

from transaction import SignedTransaction


class Block:
    def __init__(self, number, previous_block_hash, difficulty, reward):
        self._number = number
        self._previous_block_hash = previous_block_hash
        self._transactions = []
        self._difficulty = difficulty
        self._solution = None
        self._reward = reward
        self._deltas = {reward.transaction.destination: reward.transaction.amount}

    def append_transaction(self, signed_transactions):
        origin = signed_transactions.transaction.origin
        destination = signed_transactions.transaction.destination
        amount = signed_transactions.transaction.amount
        self._deltas[origin] = self._deltas.get(origin, 0) - amount
        self._deltas[destination] = self._deltas.get(destination, 0) + amount
        self._transactions.append(signed_transactions)

    def set_solution(self, solution):
        self._solution = solution

    @property
    def number(self):
        return self._number

    @property
    def previous_block_hash(self):
        return self._previous_block_hash

    @property
    def reward(self):
        return self._reward

    @property
    def transactions(self):
        return self._transactions

    @property
    def hash(self):
        m = sha256()
        m.update(self.pb.SerializeToString())
        return m.hexdigest()

    @property
    def deltas(self):
        return self._deltas

    @property
    def pb(self):
        return rpc_pb2.Block(
            number=self._number,
            previous_block_hash=self._previous_block_hash,
            transactions=[t.pb for t in self._transactions],
            difficulty=self._difficulty,
            solution=self._solution,
            reward=self._reward.pb
        )

    @classmethod
    def from_pb(cls, pb):
        inst = cls(
            number=pb.number,
            previous_block_hash=None if pb.number == 0 else pb.previous_block_hash,
            difficulty=pb.difficulty,
            reward=SignedTransaction.from_pb(pb.reward)
        )
        for transaction in pb.transactions:
            inst.append_transaction(SignedTransaction.from_pb(transaction))
        inst.set_solution(pb.solution)
        return inst

    def find_solution(self, interrupt_event):
        if self._previous_block_hash is None:
            self._solution = 0
            return
        else:
            candidate = 0
            while not self._check_solution(candidate) and not interrupt_event.is_set():
                candidate += 1

    @property
    def is_valid(self):
        deltas = {}
        deltas[self._reward.transaction.destination] = self._reward.transaction.amount
        for signed_transactions in self._transactions:
            origin = signed_transactions.transaction.origin
            destination = signed_transactions.transaction.destination
            amount = signed_transactions.transaction.amount
            deltas[origin] = deltas.get(origin, 0) - amount
            deltas[destination] = deltas.get(destination, 0) + amount
        if deltas != self._deltas:
            return False
        if self._previous_block_hash is None and self._number == 0:
            return True
        return self._check_solution(self._solution)

    @cached_property
    def _partial_hash(self):
        m = sha256()
        m.update(bytes(self._number))
        m.update(bytes.fromhex(self._previous_block_hash))
        for t in self._transactions:
            m.update(t.pb.SerializeToString())
        m.update(self._reward.pb.SerializeToString())
        return m

    def _check_solution(self, candidate):
        m = self._partial_hash.copy()
        m.update(bytes(candidate))
        digest = m.digest()
        if 256 - int.from_bytes(digest, 'big').bit_length() > self._difficulty:
            self._solution = candidate
            return True
        return False


class SignedBlock:
    def __init__(self, block, signature):
        self._block = block
        self._signature = signature

    @property
    def block(self):
        return self._block

    @property
    def is_valid(self):
        vk = VerifyingKey.from_string(bytes.fromhex(self._block.reward.transaction.destination), SECP256k1, hashfunc=sha256)
        return vk.verify(bytes.fromhex(self._signature), bytes.fromhex(self._block.hash)) and self._block.is_valid

    @property
    def pb(self):
        return rpc_pb2.SignedBlock(
            block=self._block.pb,
            signature=self._signature
        )

    @classmethod
    def from_pb(cls, pb):
        return cls(
            block=Block.from_pb(pb.block),
            signature=pb.signature
        )
