use std::{collections::HashMap, sync::{Arc, atomic::{AtomicBool, Ordering}}};

use hex;
use k256::{
    sha2::{Sha256, Digest},
    ecdsa::{VerifyingKey, signature::Verifier, Signature, signature::Signature as _}
};
use prost::Message;

use crate::{
    transaction::SignedTransaction,
    rpc::{self, PB}
};


pub struct Block {
    number: u32,
    previous_block_hash: Option<String>,
    transactions: Vec<Arc<SignedTransaction>>,
    difficulty: u32,
    solution: Option<u32>,
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
            solution: None,
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

    pub fn find_solution(&mut self, interrupt_event: Arc<AtomicBool>) {
        if let None = self.previous_block_hash {
            self.solution = Some(0);
        } else {
            let mut candidate = 0;
            while !self.check_solution(candidate) && !interrupt_event.load(Ordering::Relaxed) {
                candidate += 1;
            }
            println!("{}", candidate);
        }
    }

    fn check_solution(&self, candidate: u32) -> bool {
        let hasher = self.partial_hash();
        {
            let mut hasher = hasher.clone();
            hasher.update(candidate.to_be_bytes());
            
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
//     def _check_solution(self, candidate):
//         m = self._partial_hash.copy()
//         m.update(bytes(candidate))
//         digest = m.digest()
//         if 256 - int.from_bytes(digest, 'big').bit_length() > self._difficulty:
//             self._solution = candidate
//             return True
//         return False

    fn partial_hash(&self) -> Sha256 {
        let mut hasher = Sha256::new();
        hasher.update(self.number.to_be_bytes());
        hasher.update(hex::decode(self.previous_block_hash.clone().unwrap()).unwrap());
        for t in &self.transactions {
            let mut buf = vec![];
            t.pb().encode(&mut buf).unwrap();
            hasher.update(&mut buf);
        }
        return hasher;
    }
//     @cached_property
//     def _partial_hash(self):
//         m = sha256()
//         m.update(bytes(self._number))
//         m.update(bytes.fromhex(self._previous_block_hash))
//         for t in self._transactions:
//             m.update(t.pb.SerializeToString())
//         m.update(self._reward.pb.SerializeToString())
//         return m

    pub fn append_transaction(&mut self, signed_transaction: Arc<SignedTransaction>) {
        let origin = signed_transaction.transaction().origin().unwrap();
        let destination = signed_transaction.transaction().destination();
        let amount = signed_transaction.transaction().amount() as i32;
        match self.deltas.get_mut(origin) {
            Some(delta) => *delta -= amount,
            None => {self.deltas.insert(origin.clone(), -amount);}
        }
        match self.deltas.get_mut(destination) {
            Some(delta) => *delta += amount,
            None => {self.deltas.insert(destination.to_string(), amount);}
        }
        self.transactions.push(signed_transaction);
    }
}

impl PB<rpc::Block> for Block {
    fn pb(&self) -> rpc::Block {
        rpc::Block {
            number: self.number,
            previous_block_hash: match self.previous_block_hash.clone() {
                Some(previous_block_hash) => previous_block_hash,
                None => String::new(),
            },
            transactions: self.transactions.iter().map(|t| t.pb()).collect(),
            difficulty: self.difficulty,
            solution: self.solution.unwrap_or(0),
            reward: Some(self.reward.pb())
        }
    }
}

//     def append_transaction(self, signed_transactions):
//         origin = signed_transactions.transaction.origin
//         destination = signed_transactions.transaction.destination
//         amount = signed_transactions.transaction.amount
//         self._deltas[origin] = self._deltas.get(origin, 0) - amount
//         self._deltas[destination] = self._deltas.get(destination, 0) + amount
//         self._transactions.append(signed_transactions)

//     def set_solution(self, solution):
//         self._solution = solution

//     @property
//     def dict(self):
//         return {
//             "number": self._number,
//             "previous_block_hash": self._previous_block_hash,
//             "transactions": [t.dict for t in self._transactions],
//             "difficulty": self._difficulty,
//             "solution": self._solution,
//             "reward": self._reward.dict
//         }

//     @property
//     def number(self):
//         return self._number

//     @property
//     def previous_block_hash(self):
//         return self._previous_block_hash

//     @property
//     def reward(self):
//         return self._reward

//     @property
//     def transactions(self):
//         return self._transactions

//     @property
//     def hash(self):
//         m = sha256()
//         m.update(self.pb.SerializeToString())
//         return m.hexdigest()

//     @property
//     def deltas(self):
//         return self._deltas

//     @classmethod
//     def from_pb(cls, pb):
//         inst = cls(
//             number=pb.number,
//             previous_block_hash=None if pb.number == 0 else pb.previous_block_hash,
//             difficulty=pb.difficulty,
//             reward=SignedTransaction.from_pb(pb.reward)
//         )
//         for transaction in pb.transactions:
//             inst.append_transaction(SignedTransaction.from_pb(transaction))
//         inst.set_solution(pb.solution)
//         return inst

//     def find_solution(self, interrupt_event):
//         if self._previous_block_hash is None:
//             self._solution = 0
//             return
//         else:
//             candidate = 0
//             while not self._check_solution(candidate) and not interrupt_event.is_set():
//                 candidate += 1

//     @property
//     def is_valid(self):
//         deltas = {}
//         deltas[self._reward.transaction.destination] = self._reward.transaction.amount
//         for signed_transactions in self._transactions:
//             origin = signed_transactions.transaction.origin
//             destination = signed_transactions.transaction.destination
//             amount = signed_transactions.transaction.amount
//             deltas[origin] = deltas.get(origin, 0) - amount
//             deltas[destination] = deltas.get(destination, 0) + amount
//         if deltas != self._deltas:
//             return False
//         if self._previous_block_hash is None and self._number == 0:
//             return True
//         return self._check_solution(self._solution)

//     @cached_property
//     def _partial_hash(self):
//         m = sha256()
//         m.update(bytes(self._number))
//         m.update(bytes.fromhex(self._previous_block_hash))
//         for t in self._transactions:
//             m.update(t.pb.SerializeToString())
//         m.update(self._reward.pb.SerializeToString())
//         return m

//     def _check_solution(self, candidate):
//         m = self._partial_hash.copy()
//         m.update(bytes(candidate))
//         digest = m.digest()
//         if 256 - int.from_bytes(digest, 'big').bit_length() > self._difficulty:
//             self._solution = candidate
//             return True
//         return False



pub struct SignedBlock {
    block: Block,
    signature: String,
}

impl SignedBlock {
    pub fn new(block: Block, signature: String) -> Self {
        Self {
            block,
            signature,
        }
    }

    pub fn block(&self) -> &Block {
        &self.block
    }

    pub fn is_valid(&self) -> bool {
        let destination = self.block.reward().transaction().destination();
        let bytes = &hex::decode(destination).unwrap();
        let vk = VerifyingKey::from_sec1_bytes(&bytes).unwrap();
        let signature: Signature = Signature::from_bytes(&hex::decode(&self.signature).unwrap()).unwrap();
        match vk.verify(&hex::decode(self.block.hash()).unwrap(), &signature) {
            Ok(_) => true,
            Err(_) => false
        }
    }
//     @property
//     def is_valid(self):
//         vk = VerifyingKey.from_string(bytes.fromhex(self._block.reward.transaction.destination), NIST256p)
//         return vk.verify(bytes.fromhex(self._signature), bytes.fromhex(self._block.hash)) and self._block.is_valid
}

impl PB<rpc::SignedBlock> for SignedBlock {
    fn pb(&self) -> rpc::SignedBlock {
        rpc::SignedBlock {
            block: Some(self.block.pb()),
            signature: self.signature.clone()
        }
    }
}


// class SignedBlock:
//     def __init__(self, block, signature):
//         self._block = block
//         self._signature = signature

//     @property
//     def block(self):
//         return self._block

//     @property
//     def is_valid(self):
//         vk = VerifyingKey.from_string(bytes.fromhex(self._block.reward.transaction.destination), NIST256p)
//         return vk.verify(bytes.fromhex(self._signature), bytes.fromhex(self._block.hash)) and self._block.is_valid

//     @property
//     def pb(self):
//         return rpc_pb2.SignedBlock(
//             block=self._block.pb,
//             signature=self._signature
//         )

//     @classmethod
//     def from_pb(cls, pb):
//         return cls(
//             block=Block.from_pb(pb.block),
//             signature=pb.signature
//         )
