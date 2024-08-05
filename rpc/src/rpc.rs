use near_crypto::InMemorySigner;
use near_jsonrpc_client::methods;
use near_primitives::types::FunctionArgs;
use near_primitives::views::{FinalExecutionOutcomeViewEnum, FinalExecutionStatus};
use near_sdk::AccountId;
use serde_json::{json, Value};
use tokio::time;
use utils::types::{SignRequest, SignatureResponse};

use crate::api::{
    call_view_function, create_function_call_transaction, get_current_nonce, get_latest_block_hash,
    wait_for_transaction,
};

const GAS: u64 = 300_000_000_000_000;
const DEPOSIT: u128 = 1;

pub async fn call_sign(
    client: &near_jsonrpc_client::JsonRpcClient,
    contract_id: AccountId,
    sign_request: SignRequest,
    signer: InMemorySigner,
) -> Result<SignatureResponse, Box<dyn std::error::Error>> {
    let current_nonce = get_current_nonce(client, &signer).await?;
    let block_hash = get_latest_block_hash(client).await?;

    let transaction = create_function_call_transaction(
        &signer,
        contract_id,
        block_hash,
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

    let json_str = String::from_utf8(result.to_vec())?;
    let json_value: Value = serde_json::from_str(&json_str)?;

    if let Value::String(public_key) = json_value {
        Ok(public_key)
    } else {
        Err("Unexpected format for public key".into())
    }
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
