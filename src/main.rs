use anyhow::{Context, Result};
use axum::{body::Bytes, response::IntoResponse, routing::post, Router};
use clap::Parser;
use hyper::StatusCode;
use reqwest::{
    multipart::{Form, Part},
    Client, Url,
};
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    main_().await;
}

#[derive(Parser, Debug)]
struct Args {
    /// URL to the control API of the IPFS node to which blocks are sent.
    #[arg(long, env, default_value = "http://localhost:5001")]
    ipfs_node_rpc: Url,

    /// Value of the `Authorization` header sent to the Pinata API.
    #[arg(long, env)]
    pinata_authorization: String,

    /// Address this program should serve its HTTP API on.
    #[arg(long, env, default_value = "0.0.0.0:8000")]
    bind: SocketAddr,
}

async fn main_() {
    let args = Args::parse();
    let client = reqwest::Client::default();
    let handler = |body: Bytes| async move {
        let response = match put_block(
            &client,
            args.ipfs_node_rpc.clone(),
            body.to_vec(),
            PutBlockHash::Keccak256,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => {
                eprintln!("error sending block to ipfs node: {err:?}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        match pin_cid_pinata(&client, &args.pinata_authorization, &response.key).await {
            Ok(()) => (),
            Err(err) => {
                eprintln!("error pinning cid in pinata: {err:?}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        (StatusCode::OK, response.key).into_response()
    };
    let app = Router::new().route("/pin_block", post(handler));
    axum::Server::bind(&args.bind)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn put_block(
    client: &Client,
    ipfs_node: Url,
    data: Vec<u8>,
    hash: PutBlockHash,
) -> Result<PutBlockResponse> {
    let mut url = ipfs_node;
    url.path_segments_mut()
        .unwrap()
        .pop_if_empty()
        .extend(&["api", "v0", "block", "put"]);
    url.query_pairs_mut()
        .append_pair("mhtype", hash.ipfs_name())
        .append_pair("cid-codec ", "raw");
    client
        .post(url)
        .multipart(Form::new().part("", Part::bytes(data)))
        .send()
        .await
        .context("send")?
        .error_for_status()
        .context("status")?
        .json()
        .await
        .context("json")
}

enum PutBlockHash {
    _Sha256,
    Keccak256,
}

impl PutBlockHash {
    fn ipfs_name(&self) -> &'static str {
        match self {
            PutBlockHash::_Sha256 => "sha2-256",
            PutBlockHash::Keccak256 => "keccak-256",
        }
    }
}

#[derive(Deserialize)]
struct PutBlockResponse {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Size")]
    _size: u64,
}

async fn pin_cid_pinata(client: &Client, authorization: &str, cid: &str) -> Result<()> {
    let body = json!({ "hashToPin": cid });
    client
        .post("https://api.pinata.cloud/pinning/pinByHash")
        .header("Authorization", authorization)
        .json(&body)
        .send()
        .await
        .context("send")?
        .error_for_status()
        .context("status")
        .map(|_| ())
}
