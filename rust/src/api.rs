use std::sync::Arc;

use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router, extract::Path,
};
use tokio::sync::mpsc::Sender;
use triggered::Listener;

use crate::{transaction::SignedTransaction, runtime::Runtime};


pub struct API {
    port: String,
    runtime: Arc<Runtime>,
    txn_sender: Sender<SignedTransaction>,
    stop: Option<Listener>,
}

impl API {
    pub fn new(
        port: &str,
        runtime: Arc<Runtime>,
        txn_sender: Sender<SignedTransaction>,
        stop: Listener,
    ) -> API {
        Self {
            port: port.to_string(),
            runtime,
            txn_sender,
            stop: Some(stop),
        }
    }

    pub async fn start(mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting");
        let app = Router::new()
        .route("/block_count", get({
            let runtime = self.runtime.clone();
            || block_count(runtime)
        }))
        .route("/block/:block_number", get({
            let runtime = self.runtime.clone();
            |block_number| get_block_transactions(block_number, runtime)
        }))
        .route("/transaction/:transaction_hash", get(
            |txn_hash| get_transaciton(txn_hash, self.runtime)
        ))
        .route("/transaction", post(
            move |txn| post_transaciton(txn, self.txn_sender)
        ));

        let addr = format!("[::1]:{}", self.port).parse()?;

        {
            let stop = self.stop.take().unwrap();
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .with_graceful_shutdown(stop)
                .await
                .unwrap();
        }

        Ok(())
    }
}


async fn block_count(runtime: Arc<Runtime>) -> (StatusCode, Json<Option<u32>>) {
    match runtime.blockchain().lock().unwrap().get_last_block() {
        Some(signed_block) => (StatusCode::FOUND, Json(Some(signed_block.block().number()))),
        None => (StatusCode::NOT_FOUND, Json(None))
    }
}

async fn get_block_transactions(Path(block_number): Path<usize>, runtime: Arc<Runtime>) -> (StatusCode, Json<Option<Vec<Arc<SignedTransaction>>>>) {
    match runtime.blockchain().lock().unwrap().get_block(block_number){
        Some(signed_block) => {
            (StatusCode::FOUND, Json(Some(signed_block.block().transactions().iter().map(|t| t.clone()).collect::<Vec<Arc<SignedTransaction>>>())))
        },
        None => (StatusCode::NOT_FOUND, Json(None))
    }
}

async fn get_transaciton(Path(transaction_hash): Path<String>, runtime: Arc<Runtime>) -> (StatusCode, Json<Option<Arc<SignedTransaction>>>) {
    match runtime.get_transaction(transaction_hash) {
        Some(txn) => (StatusCode::FOUND, Json(Some(txn))),
        None => (StatusCode::NOT_FOUND, Json(None))
    }
}

async fn post_transaciton(Json(payload): Json<SignedTransaction>, txn_sender: Sender<SignedTransaction>) -> (StatusCode, Json<String>) {
    let hash = payload.hash().clone();
    txn_sender.send(payload).await;
    (StatusCode::ACCEPTED, Json(hash))
}