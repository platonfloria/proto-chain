from functools import cached_property
from hashlib import sha256

from ecdsa import SigningKey, SECP256k1

from proto_chain.transaction import SignedTransaction
from proto_chain.block import SignedBlock


class Account:
    def __init__(self, private_key=None):
        if private_key is None:
            self._sk = SigningKey.generate(SECP256k1)
        else:
            self._sk = SigningKey.from_string(bytes.fromhex(private_key), SECP256k1)
        self._nonce = 0

    @cached_property
    def address(self):
        return self._sk.verifying_key.to_string("compressed").hex()

    def sign_transaction(self, transaction):
        transaction.set_nonce(self._nonce)
        self._nonce += 1
        return SignedTransaction(transaction, self._sk.sign_deterministic(bytes.fromhex(transaction.hash), hashfunc=sha256).hex())

    def sign_block(self, block):
        return SignedBlock(block, self._sk.sign_deterministic(bytes.fromhex(block.hash), hashfunc=sha256).hex())
