use near_crypto::InMemorySigner;
use near_jsonrpc_client::{methods, JsonRpcClient};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_jsonrpc_primitives::types::transactions::{RpcTransactionError, TransactionInfo};
use near_primitives::hash::CryptoHash;
use near_primitives::transaction::{Action, FunctionCallAction, Transaction};
use near_primitives::types::{BlockReference, Finality, FunctionArgs};
use near_primitives::views::{FinalExecutionOutcomeViewEnum, QueryRequest, TxExecutionStatus};
use near_sdk::AccountId;

use tokio::time;
use utils::types::NearNetwork;

pub async fn get_current_nonce(
    client: &near_jsonrpc_client::JsonRpcClient,
    signer: &InMemorySigner,
) -> Result<u64, Box<dyn std::error::Error>> {
    let access_key_query_response = client
        .call(methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: near_primitives::views::QueryRequest::ViewAccessKey {
                account_id: signer.account_id.clone(),
                public_key: signer.public_key.clone(),
            },
        })
        .await?;

    match access_key_query_response.kind {
        QueryResponseKind::AccessKey(access_key) => Ok(access_key.nonce),
        _ => Err("failed to extract current nonce".into()),
    }
}

pub fn create_function_call_transaction(
    signer: &InMemorySigner,
    receiver_id: AccountId,
    block_hash: CryptoHash,
    nonce: u64,
    method_name: String,
    args: Vec<u8>,
    gas: u64,
    deposit: u128,
) -> Transaction {
    Transaction {
        signer_id: signer.account_id.clone(),
        public_key: signer.public_key.clone(),
        nonce,
        receiver_id,
        block_hash,
        actions: vec![Action::FunctionCall(Box::new(FunctionCallAction {
            method_name,
            args,
            gas,
            deposit,
        }))],
    }
}

pub async fn wait_for_transaction(
    client: &near_jsonrpc_client::JsonRpcClient,
    tx_hash: CryptoHash,
    signer: &InMemorySigner,
    timeout: time::Duration,
) -> Result<FinalExecutionOutcomeViewEnum, Box<dyn std::error::Error>> {
    let sent_at = time::Instant::now();

    loop {
        let response = client
            .call(methods::tx::RpcTransactionStatusRequest {
                transaction_info: TransactionInfo::TransactionId {
                    tx_hash,
                    sender_account_id: signer.account_id.clone(),
                },
                wait_until: TxExecutionStatus::Executed,
            })
            .await;
        let received_at = time::Instant::now();
        let delta = received_at - sent_at;

        if delta > timeout {
            return Err("time limit exceeded for the transaction to be recognized".into());
        }

        match response {
            Err(err) => match err.handler_error() {
                Some(
                    RpcTransactionError::TimeoutError
                    | RpcTransactionError::UnknownTransaction { .. },
                ) => {
                    time::sleep(time::Duration::from_secs(10)).await;
                    continue;
                }
                _ => return Err(err.into()),
            },
            Ok(response) => {
                if let Some(outcome) = response.final_execution_outcome {
                    return Ok(outcome);
                }
            }
        }
    }
}

pub async fn call_view_function(
    client: &near_jsonrpc_client::JsonRpcClient,
    contract_id: AccountId,
    method_name: String,
    args: FunctionArgs,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let request = methods::query::RpcQueryRequest {
        block_reference: BlockReference::Finality(Finality::Final),
        request: QueryRequest::CallFunction {
            account_id: contract_id,
            method_name,
            args,
        },
    };

    let response = client.call(request).await?;

    if let QueryResponseKind::CallResult(result) = response.kind {
        Ok(result.result)
    } else {
        Err("Unexpected response kind".into())
    }
}

pub async fn get_latest_block_hash(
    client: &near_jsonrpc_client::JsonRpcClient,
) -> Result<CryptoHash, Box<dyn std::error::Error>> {
    let request = methods::block::RpcBlockRequest {
        block_reference: BlockReference::Finality(Finality::Final),
    };

    let response = client.call(request).await?;
    Ok(response.header.hash)
}

pub fn get_near_client(network: NearNetwork) -> JsonRpcClient {
    let rpc_url = match network {
        NearNetwork::Mainnet => "https://rpc.mainnet.near.org",
        NearNetwork::Testnet => "https://rpc.testnet.near.org",
    };

    JsonRpcClient::connect(rpc_url)
}
