use std::sync::{Arc, Mutex};

use k256::sha2::{Sha256, Digest};
use tonic::{transport::Server, Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use triggered::Listener;


mod rpc_pb2 {
    tonic::include_proto!("rpc");
}
pub use rpc_pb2::*;

type TransactionChannel = mpsc::Sender<Result<SignedTransaction, Status>>;
pub type TransactionQueues = Arc<Mutex<Vec<TransactionChannel>>>;
type BlockChannel = mpsc::Sender<Result<SignedBlock, Status>>;
pub type BlockQueues = Arc<Mutex<Vec<BlockChannel>>>;


pub trait PB<M: prost::Message> {
    fn hash(&self) -> String
    where
        Self: Sized
    {
        let mut hasher = Sha256::new();
        let buf = self.pb().encode_to_vec();
        hasher.update(&buf);
        hex::encode(hasher.finalize())
    }

    fn pb(&self) -> M;
}

#[derive(Debug)]
pub struct RPC {
    port: String,
    transaction_queues: TransactionQueues,
    block_queues: BlockQueues,
    stop: Option<Listener>,
}

impl RPC {
    pub fn new(
        port: &str,
        transaction_queues: TransactionQueues,
        block_queues: BlockQueues,
        stop: Listener,
    ) -> RPC {
        Self {
            port: port.to_string(),
            transaction_queues,
            block_queues,
            stop: Some(stop),
        }
    }

    pub async fn start(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("[::1]:{}", self.port).parse()?;
    
        {
            let stop = self.stop.take().unwrap();
            Server::builder()
                .add_service(rpc_server::RpcServer::new(self))
                .serve_with_shutdown(addr, stop)
                .await?;
        }
    
        Ok(())
    }
}

#[tonic::async_trait]
impl rpc_server::Rpc for RPC {
    type TransactionFeedStream = ReceiverStream<Result<SignedTransaction, Status>>;
    type BlockFeedStream = ReceiverStream<Result<SignedBlock, Status>>;

    async fn transaction_feed(
        &self,
        request: Request<TransactionFeedRequest>,
    ) -> Result<Response<Self::TransactionFeedStream>, Status> {
        let (tx, rx) = mpsc::channel(1);
        self.transaction_queues.lock().unwrap().push(tx);

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn block_feed(
        &self,
        request: Request<BlockFeedRequest>,
    ) -> Result<Response<Self::BlockFeedStream>, Status> {
        let (tx, rx) = mpsc::channel(1);
        self.block_queues.lock().unwrap().push(tx);
        println!("PUSHED {:?}", self.block_queues.lock().unwrap().len());

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn sync(
        &self,
        request: Request<SyncRequest>,
    ) -> Result<Response<BlockChain>, Status> {
        let reply = BlockChain {
            blocks: vec![],
        };

        Ok(Response::new(reply))
    }
}