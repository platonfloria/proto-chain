class BlockChain:
    def __init__(self):
        self._blocks = []
        self._balances = {}
        self._transaction_lookup = {}

    def append_block(self, signed_block):
        assert signed_block.is_valid
        if self._blocks != []:
            assert signed_block.block.previous_block_hash == self._blocks[-1].block.hash
        self._blocks.append(signed_block)
        for address, delta in signed_block.block.deltas.items():
            self._balances[address] = self._balances.get(address, 0) + delta
        self._transaction_lookup[signed_block.block.reward.hash] = signed_block.block.reward
        for transaction in signed_block.block.transactions:
            self._transaction_lookup[transaction.hash] = transaction

    def get_block(self, block_number):
        return self._blocks[block_number]
    
    def get_transaction(self, transaction_hash):
        return self._transaction_lookup.get(transaction_hash)

    @property
    def blocks(self):
        return self._blocks

    @property
    def last_block(self):
        return self._blocks[-1].block

    def is_transaction_valid(self, signed_transaction, next_block):
        if signed_transaction.is_valid:
            origin = signed_transaction.transaction.origin
            amount = signed_transaction.transaction.amount
            return self._balances.get(origin, 0) + next_block.deltas.get(origin, 0) >= amount
        return False
