use near_crypto::InMemorySigner;
use near_jsonrpc_client::methods;
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_jsonrpc_primitives::types::transactions::{RpcTransactionError, TransactionInfo};
use near_primitives::transaction::{Action, FunctionCallAction, Transaction};
use near_primitives::types::{BlockReference, Finality, FunctionArgs};
use near_primitives::views::{
    FinalExecutionOutcomeViewEnum, FinalExecutionStatus, QueryRequest, TxExecutionStatus,
};

use serde_json::json;
use tokio::time;

use crate::utils::types::{SignRequest, SignatureResponse};
use near_sdk::AccountId;

const GAS: u64 = 300_000_000_000_000;
const DEPOSIT: u128 = 1;

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
    block_hash: near_primitives::hash::CryptoHash,
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
    tx_hash: near_primitives::hash::CryptoHash,
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

pub async fn call_sign(
    client: &near_jsonrpc_client::JsonRpcClient,
    contract_id: AccountId,
    sign_request: SignRequest,
    signer: InMemorySigner,
) -> Result<SignatureResponse, Box<dyn std::error::Error>> {
    let current_nonce = get_current_nonce(client, &signer).await?;
    let access_key_query_response = client
        .call(methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: near_primitives::views::QueryRequest::ViewAccessKey {
                account_id: signer.account_id.clone(),
                public_key: signer.public_key.clone(),
            },
        })
        .await?;

    let transaction = create_function_call_transaction(
        &signer,
        contract_id,
        access_key_query_response.block_hash,
        current_nonce + 1,
        "sign".to_string(),
        json!({"request": sign_request}).to_string().into_bytes(),
        GAS,
        DEPOSIT,
    );

    let request = methods::broadcast_tx_async::RpcBroadcastTxAsyncRequest {
        signed_transaction: transaction.sign(&signer),
    };

    let tx_hash = client.call(request).await?;

    let outcome =
        wait_for_transaction(client, tx_hash, &signer, time::Duration::from_secs(300)).await?;

    if let FinalExecutionOutcomeViewEnum::FinalExecutionOutcome(outcome) = outcome {
        if let FinalExecutionStatus::SuccessValue(value) = outcome.status {
            let signature_response: SignatureResponse = serde_json::from_slice(&value)
                .map_err(|e| format!("Failed to parse SignatureResponse: {}", e))?;
            Ok(signature_response)
        } else {
            Err("Execution did not result in a SuccessValue".into())
        }
    } else {
        Err("No final execution outcome available".into())
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

pub async fn call_public_key(
    client: &near_jsonrpc_client::JsonRpcClient,
    contract_id: AccountId,
) -> Result<String, Box<dyn std::error::Error>> {
    let result = call_view_function(
        client,
        contract_id,
        "public_key".to_string(),
        FunctionArgs::from(vec![]),
    )
    .await?;

    String::from_utf8(result).map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::sha2::{Digest, Sha256};
    use near_crypto::SecretKey;
    use near_jsonrpc_client::JsonRpcClient;
    use near_primitives::types::AccountId;

    #[tokio::test]
    async fn test_sign() -> Result<(), Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();

        let account_id: AccountId = std::env::var("NEAR_ACCOUNT_ID").unwrap().parse().unwrap();
        let private_key: SecretKey = std::env::var("NEAR_PRIVATE_KEY").unwrap().parse().unwrap();
        let contract_id: AccountId = std::env::var("CHAIN_SIGNATURE_CONTRACT")
            .unwrap()
            .parse()
            .unwrap();

        let signer = InMemorySigner::from_secret_key(account_id.clone(), private_key);
        let client = JsonRpcClient::connect("https://rpc.testnet.near.org");

        // Prepare the sign request
        let sign_request = SignRequest {
            payload: Sha256::digest("test".as_bytes()).into(),
            path: "test".to_string(),
            key_version: 0,
        };

        // Call the sign function
        let result = call_sign(&client, contract_id, sign_request, signer).await?;

        println!("Sign result: {:?}", result);

        Ok(())
    }

    #[tokio::test]
    async fn test_public_key() -> Result<(), Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();

        let contract_id: AccountId = std::env::var("CHAIN_SIGNATURE_CONTRACT")
            .unwrap()
            .parse()
            .unwrap();

        let client = JsonRpcClient::connect("https://rpc.testnet.near.org");

        let result = call_public_key(&client, contract_id).await;

        println!("{:?}", result);

        Ok(())
    }
}
