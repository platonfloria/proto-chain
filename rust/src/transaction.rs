use hex;
use k256::{
    ecdsa::{VerifyingKey, signature::Verifier, Signature, signature::Signature as _}
};
use serde::{Serialize, Deserialize};

use crate::rpc::{self, PB};


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
        self.destination.as_ref()
    }

    pub fn amount(&self) -> u32 {
        self.amount
    }

    pub fn set_nonce(&mut self, nonce: u32) {
        self.nonce = Some(nonce);
    }
}

impl PB<rpc::Transaction> for Transaction {
    fn pb(&self) -> rpc::Transaction {
        rpc::Transaction {
            origin: match self.origin.clone() {
                Some(origin) => origin,
                None => String::new(),
            },
            destination: self.destination.clone(),
            amount: self.amount,
            data: self.data.clone(),
            nonce: self.nonce.unwrap_or(0)
        }
    }
}



//     @property
//     def dict(self):
//         return {
//             "nonce": self._nonce,
//             "origin": self._origin,
//             "destination": self._destination,
//             "amount": self._amount,
//             "data": self._data
//         }

//     @classmethod
//     def from_dict(cls, dict):
//         inst = cls(
//             origin=dict["origin"],
//             destination=dict["destination"],
//             amount=dict["amount"],
//             data=dict["data"],
//         )
//         inst.set_nonce(dict["nonce"])
//         return inst

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

//     @property
//     def origin(self):
//         return self._origin

//     @property
//     def destination(self):
//         return self._destination

//     @property
//     def amount(self):
//         return self._amount

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



    // pub fn is_valid(&self) -> bool {
    //     let destination = self.block.reward().transaction().destination();
    //     let bytes = &hex::decode(destination).unwrap();
    //     let vk = VerifyingKey::from_sec1_bytes(&bytes).unwrap();
    //     let signature: Signature = Signature::from_bytes(&hex::decode(&self.signature).unwrap()).unwrap();
    //     match vk.verify(&hex::decode(self.block.hash()).unwrap(), &signature) {
    //         Ok(_) => true,
    //         Err(_) => false
    //     }
    // }

    pub fn is_valid(&self) -> bool
    where
        Self: rpc::PB<rpc::SignedTransaction>
    {
        match &self.transaction.origin {
            Some(origin) => {
                let bytes = &hex::decode(origin).unwrap();
                let vk = VerifyingKey::from_sec1_bytes(&bytes).unwrap();
                let signature: Signature = Signature::from_bytes(&hex::decode(&self.signature).unwrap()).unwrap();
                match vk.verify(&hex::decode(self.transaction.hash()).unwrap(), &signature) {
                    Ok(_) => true,
                    Err(a) => false
                }
            },
            None => false
        }
    }
//     @property
//     def is_valid(self):
//         vk = VerifyingKey.from_string(bytes.fromhex(self._transaction.origin), NIST256p)
//         return vk.verify(bytes.fromhex(self._signature), bytes.fromhex(self._transaction.hash))

}

impl PB<rpc::SignedTransaction> for SignedTransaction {
    fn pb(&self) -> rpc::SignedTransaction {
        rpc::SignedTransaction {
            transaction: Some(self.transaction.pb()),
            signature: self.signature.clone(),
        }
    }
}


// class SignedTransaction:
//     def __init__(self, transaction, signature):
//         self._transaction = transaction
//         self._signature = signature

//     @property
//     def transaction(self):
//         return self._transaction

//     @property
//     def hash(self):
//         m = sha256()
//         m.update(self.pb.SerializeToString())
//         return m.hexdigest()

//     @property
//     def dict(self):
//         return {
//             "transaction": self._transaction.dict,
//             "signature": self._signature
//         }

//     @classmethod
//     def from_dict(cls, dict):
//         return cls(
//             transaction=Transaction.from_dict(dict["transaction"]),
//             signature=dict["signature"]
//         )

//     @classmethod
//     def from_pb(cls, pb):
//         return cls(
//             transaction=Transaction.from_pb(pb.transaction),
//             signature=pb.signature
//         )
