use hex;
use k256::{
    ecdsa::{VerifyingKey, signature::Verifier, Signature, signature::Signature as _}, sha2::{Sha256, Digest}
};
use serde::{Serialize, Deserialize};

use crate::rpc;


#[derive(Serialize, Deserialize)]
pub struct Transaction {
    origin: Option<String>,
    destination: String,
    amount: u32,
    data: String,
    nonce: Option<u32>,
}

impl Transaction {
    pub fn new(origin: Option<String>, destination: String, amount: u32, data: String) -> Self {
        Self {
            origin,
            destination,
            amount,
            data,
            nonce: None,
        }
    }

    pub fn origin(&self) -> Option<&String> {
        self.origin.as_ref()
    }

    pub fn destination(&self) -> &str {
        &self.destination
    }

    pub fn amount(&self) -> u32 {
        self.amount
    }

    pub fn set_nonce(&mut self, nonce: u32) {
        self.nonce = Some(nonce);
    }

    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        if let Some(origin) = &self.origin {
            hasher.update(origin.as_bytes());
        }
        hasher.update(self.destination.as_bytes());
        hasher.update(self.amount.to_be_bytes());
        hasher.update(self.data.as_bytes());
        if let Some(nonce) = self.nonce {
            hasher.update(nonce.to_be_bytes());
        }
        hex::encode(hasher.finalize())
    }

    fn pb(&self) -> rpc::Transaction {
        rpc::Transaction {
            origin: self.origin.as_ref().unwrap_or(&String::new()).to_owned(),
            destination: self.destination.to_owned(),
            amount: self.amount,
            data: self.data.to_owned(),
            nonce: self.nonce.unwrap_or(0)
        }
    }
}

//     @classmethod
//     def from_pb(cls, pb):
//         inst = cls(
//             origin=pb.origin,
//             destination=pb.destination,
//             amount=pb.amount,
//             data=pb.data,
//         )
//         inst.set_nonce(pb.nonce)
//         return inst

//     def __repr__(self):
//         return f"From {self._origin[:4]} to {self._destination[:4]} {self._amount}"


#[derive(Serialize, Deserialize)]
pub struct SignedTransaction {
    transaction: Transaction,
    signature: String,
}

impl SignedTransaction {
    pub fn new(transaction: Transaction, signature: String) -> Self {
        Self {
            transaction,
            signature,
        }
    }

    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.transaction.hash());
        hasher.update(self.signature.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn is_valid(&self) -> bool
    {
        match &self.transaction.origin {
            Some(origin) => {
                let bytes = &hex::decode(origin).unwrap();
                let vk = VerifyingKey::from_sec1_bytes(&bytes).unwrap();
                let signature: Signature = Signature::from_bytes(&hex::decode(&self.signature).unwrap()).unwrap();
                match vk.verify(&hex::decode(self.transaction.hash()).unwrap(), &signature) {
                    Ok(_) => true,
                    Err(_) => false
                }
            },
            None => false
        }
    }

    pub fn pb(&self) -> rpc::SignedTransaction {
        rpc::SignedTransaction {
            transaction: Some(self.transaction.pb()),
            signature: self.signature.to_owned(),
        }
    }
}

//     @classmethod
//     def from_pb(cls, pb):
//         return cls(
//             transaction=Transaction.from_pb(pb.transaction),
//             signature=pb.signature
//         )
