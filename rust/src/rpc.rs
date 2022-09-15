use std::sync::Arc;

use tonic::{transport::Server, Request, Response, Status};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use triggered::Listener;


mod rpc_pb2 {
    tonic::include_proto!("rpc");
}
pub use rpc_pb2::*;

use crate::{
    runtime::Runtime,
    block::SignedBlock as SignedBlockInternal
};

type TransactionChannel = mpsc::Sender<Result<SignedTransaction, Status>>;
pub type TransactionQueues = Arc<Mutex<Vec<TransactionChannel>>>;
type BlockChannel = mpsc::Sender<Result<SignedBlock, Status>>;
pub type BlockQueues = Arc<Mutex<Vec<BlockChannel>>>;


pub struct RPC {
    port: String,
    runtime: Arc<Runtime>,
    transaction_queues: TransactionQueues,
    block_queues: BlockQueues,
    stop: Option<Listener>,
}

impl RPC {
    pub fn new(
        port: &u32,
        runtime: Arc<Runtime>,
        transaction_queues: TransactionQueues,
        block_queues: BlockQueues,
        stop: Listener,
    ) -> RPC {
        Self {
            port: port.to_string(),
            runtime,
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
        _: Request<TransactionFeedRequest>,
    ) -> Result<Response<Self::TransactionFeedStream>, Status> {
        let (tx, rx) = mpsc::channel(1);
        self.transaction_queues.lock().await.push(tx);

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn block_feed(
        &self,
        _: Request<BlockFeedRequest>,
    ) -> Result<Response<Self::BlockFeedStream>, Status> {
        let (tx, rx) = mpsc::channel(1);
        self.block_queues.lock().await.push(tx);
        println!("PUSHED {:?}", self.block_queues.lock().await.len());

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn sync(
        &self,
        request: Request<SyncRequest>,
    ) -> Result<Response<BlockChain>, Status> {
        self.runtime.add_peer(&request.into_inner().address);        
        let reply = BlockChain {
            blocks: self.runtime.blockchain().lock().unwrap().blocks().iter().map(|b| b.pb()).collect(),
        };
        Ok(Response::new(reply))
    }
}

pub async fn get_blocks_from_peers(peers: &[String], address: &str) -> Result<Vec<SignedBlockInternal>, Box<dyn std::error::Error>> {
    let mut blocks = Vec::new();
    for peer in peers {
        blocks.clear();
        let mut client = rpc_client::RpcClient::connect(format!["http://{}", peer]).await?;
        let response = client.sync(Request::new(SyncRequest {address: address.to_owned()})).await?;
        blocks.extend(response.into_inner().blocks.iter().map(|b| SignedBlockInternal::from_pb(b)));
    }
    Ok(blocks)
}
