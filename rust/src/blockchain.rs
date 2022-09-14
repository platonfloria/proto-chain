use std::{collections::HashMap, sync::Arc};

use crate::{
    block::{Block, SignedBlock},
    transaction::SignedTransaction,
};


pub struct Blockchain {
    blocks: Vec<SignedBlock>,
    balances: HashMap<String, u32>,
    transaction_lookup: HashMap<String, Arc<SignedTransaction>>,
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            balances: HashMap::new(),
            transaction_lookup: HashMap::new(),
        }
    }

    pub fn blocks(&self) -> &[SignedBlock] {
        &self.blocks
    }

    pub fn get_block(&self, block_number: usize) -> Option<&SignedBlock> {
        self.blocks.get(block_number)
    }

    pub fn get_last_block(&self) -> Option<&SignedBlock> {
        self.blocks.last()
    }

    pub fn balances(&self) -> &HashMap<String, u32> {
        &self.balances
    }
    
    pub fn get_transaciton(&self, transaction_hash: &str) -> Option<&Arc<SignedTransaction>> {
        self.transaction_lookup.get(transaction_hash)
    }

    pub fn append_block(&mut self, signed_block: SignedBlock) {
        assert!(signed_block.is_valid());
        if !self.blocks.is_empty() {
            assert_eq!(*signed_block.block().previous_block_hash().as_ref().unwrap(), self.blocks.last().unwrap().hash());
        }
        *self.balances.entry(
            signed_block.block().reward().transaction().destination().to_owned()
        ).or_insert(0) += signed_block.block().reward().transaction().amount();
        for (address, delta) in signed_block.block().deltas() {
            if *delta >= 0 {
                *self.balances.entry(address.to_owned()).or_insert(0) += *delta as u32;
            } else {
                *self.balances.entry(address.to_owned()).or_insert(0) -= delta.abs() as u32;
            }
        }
        self.transaction_lookup.insert(signed_block.block().reward().hash(), signed_block.block().reward().clone());
        for tx in signed_block.block().transactions() {
            self.transaction_lookup.insert(tx.hash(), tx.clone());
        }
        self.blocks.push(signed_block);
    }

    pub fn is_transaction_valid(&self, signed_transaction: &SignedTransaction, next_block: &Block) -> bool {
        match signed_transaction.transaction().origin() {
            Some(origin) => {
                signed_transaction.is_valid() &&
                *self.balances.get(origin).unwrap_or(&0) as i32 + next_block.deltas().get(origin).unwrap_or(&0) >= signed_transaction.transaction().amount() as i32
            },
            None => false
        }
    }
}