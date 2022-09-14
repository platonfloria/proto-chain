use hex;
use k256::{ecdsa::{self, signature::Signer}};

use crate::{
    transaction::{Transaction, SignedTransaction},
    block::{Block, SignedBlock},
    rpc::PB
};


pub struct Account {
    sk: ecdsa::SigningKey,
    nonce: u32,
}

impl Account {
    pub fn new(private_key: &str) -> Self {
        let priv_key_bytes = &hex::decode(private_key).unwrap();
        let sk = ecdsa::SigningKey::from_bytes(&priv_key_bytes).unwrap();
        Self {
            sk,
            nonce: 0
        }
    }

    pub fn get_address(&self) -> String {
        let vk = ecdsa::VerifyingKey::from(&self.sk);
        hex::encode(vk.to_bytes())
    }

    pub fn sign_transaction(&mut self, mut transaction: Transaction) -> SignedTransaction {
        transaction.set_nonce(self.nonce);
        self.nonce += 1;
        let signature: ecdsa::Signature = self.sk.sign(&hex::decode(transaction.hash()).unwrap());
        SignedTransaction::new(transaction, hex::encode(signature))
    }

    pub fn sign_block(&self, block: Block) -> SignedBlock {
        let signature: ecdsa::Signature = self.sk.sign(&hex::decode(block.hash()).unwrap());
        SignedBlock::new(block, hex::encode(signature))
    }
}
