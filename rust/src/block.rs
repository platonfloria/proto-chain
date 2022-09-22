use std::{collections::HashMap, sync::{Arc, atomic::{AtomicBool, Ordering}}};

use hex;
use k256::{
    sha2::{Sha256, Digest},
    ecdsa::{VerifyingKey, signature::Verifier, Signature, signature::Signature as _}
};

use crate::{
    transaction::SignedTransaction,
    rpc
};


pub struct Block {
    number: u32,
    previous_block_hash: Option<String>,
    transactions: Vec<Arc<SignedTransaction>>,
    difficulty: u32,
    reward: Arc<SignedTransaction>,
    deltas: HashMap<String, i32>,
}

impl Block {
    pub fn new(
        number: u32,
        previous_block_hash: Option<String>,
        difficulty: u32,
        reward: SignedTransaction
    ) -> Self {
        Self {
            number,
            previous_block_hash,
            transactions: Vec::new(),
            difficulty,
            reward: Arc::new(reward),
            deltas: HashMap::new(),
        }
    }

    pub fn number(&self) -> u32 {
        self.number
    }

    pub fn previous_block_hash(&self) -> &Option<String> {
        &self.previous_block_hash
    }

    pub fn transactions(&self) -> &[Arc<SignedTransaction>] {
        &self.transactions
    }

    pub fn reward(&self) -> &Arc<SignedTransaction> {
        &self.reward
    }

    pub fn deltas(&self) -> &HashMap<String, i32> {
        &self.deltas
    }

    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.number.to_be_bytes());
        if let Some(previous_block_hash) = &self.previous_block_hash {
            hasher.update(previous_block_hash.as_bytes());
        }
        for txn in &self.transactions {
            hasher.update(txn.hash());
        }
        hasher.update(self.difficulty.to_be_bytes());
        hasher.update(self.reward.hash());
        hex::encode(hasher.finalize())
    }

    pub fn find_solution(&mut self, interrupt_event: &AtomicBool) -> u32 {
        let mut candidate = 0;
        while !self.check_solution(&candidate) && !interrupt_event.load(Ordering::Relaxed) {
            candidate += 1;
        }
        candidate
    }

    fn check_solution(&self, solution: &u32) -> bool {
        if self.previous_block_hash == None && self.number == 0 {
            true
        } else {
            let mut hasher = Sha256::new();
            hasher.update(hex::decode(self.hash()).unwrap());
            hasher.update(solution.to_be_bytes());
            
            let mut zeros = 0;
            for byte in hasher.finalize().to_vec() {
                zeros += byte.leading_zeros();
                if byte != 0 {
                    break;
                }
            }
            zeros > self.difficulty
        }
    }

    pub fn append_transaction(&mut self, signed_transaction: Arc<SignedTransaction>) {
        let origin = signed_transaction.transaction().origin().unwrap();
        let destination = signed_transaction.transaction().destination();
        let amount = signed_transaction.transaction().amount() as i32;
        match self.deltas.get_mut(origin) {
            Some(delta) => *delta -= amount,
            None => {self.deltas.insert(origin.to_owned(), -amount);}
        }
        match self.deltas.get_mut(destination) {
            Some(delta) => *delta += amount,
            None => {self.deltas.insert(destination.to_string(), amount);}
        }
        self.transactions.push(signed_transaction);
    }

    fn pb(&self) -> rpc::Block {
        rpc::Block {
            number: self.number,
            previous_block_hash: self.previous_block_hash.as_ref().unwrap_or(&String::new()).to_owned(),
            transactions: self.transactions.iter().map(|t| t.pb()).collect(),
            difficulty: self.difficulty,
            reward: Some(self.reward.pb())
        }
    }

    pub fn from_pb(pb: &rpc::Block) -> Self {
        Self {
            number: pb.number,
            previous_block_hash: Some(pb.previous_block_hash.to_owned()),
            transactions: pb.transactions.iter().map(|t| Arc::new(SignedTransaction::from_pb(t))).collect(),
            difficulty: pb.difficulty,
            reward: Arc::new(SignedTransaction::from_pb(pb.reward.as_ref().unwrap())),
            deltas: HashMap::new(),
        }
    }
}

pub struct SignedBlock {
    block: Block,
    solution: Option<u32>,
    signature: String,
}

impl SignedBlock {
    pub fn new(block: Block, signature: String) -> Self {
        Self {
            block,
            solution: None,
            signature,
        }
    }

    pub fn block(&self) -> &Block {
        &self.block
    }

    pub fn set_solution(&mut self, solution: u32) {
        self.solution = Some(solution);
    }

    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.block.hash());
        if let Some(solution) = self.solution {
            hasher.update(solution.to_be_bytes());
        }
        hasher.update(self.signature.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn is_valid(&self) -> bool {
        let destination = self.block.reward().transaction().destination();
        let bytes = &hex::decode(destination).unwrap();
        let vk = VerifyingKey::from_sec1_bytes(&bytes).unwrap();
        let signature: Signature = Signature::from_bytes(&hex::decode(&self.signature).unwrap()).unwrap();
        match (vk.verify(&hex::decode(self.block.hash()).unwrap(), &signature), self.solution) {
            (Ok(_), Some(solution)) => self.block.check_solution(&solution),
            (Ok(_), None) => true,
            (Err(_), _) => false
        }
    }

    pub fn pb(&self) -> rpc::SignedBlock {
        rpc::SignedBlock {
            block: Some(self.block.pb()),
            solution: self.solution.unwrap_or(0),
            signature: self.signature.to_owned()
        }
    }

    pub fn from_pb(pb: &rpc::SignedBlock) -> Self {
        Self {
            block: Block::from_pb(pb.block.as_ref().unwrap()),
            solution: if pb.solution == 0 { None } else { Some(pb.solution) },
            signature: pb.signature.to_owned(),
        }
    }
}
