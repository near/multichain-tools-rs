// use near_sdk::{AccountId, NearToken, PublicKey};
// use near_workspaces::{InMemorySigner, Worker};

// use multichain_tools_rs::utils::types::SignatureResponse;

// pub async fn call_sign(
//     worker: &Worker<impl near_workspaces::Network + 'static>,
//     contract: &AccountId,
//     signer: InMemorySigner,
//     sign_request: serde_json::Value,
// ) -> Result<SignatureResponse, Box<dyn std::error::Error>> {
//     let sign_response: SignatureResponse = worker
//         .call(&signer, contract, "sign")
//         .args_json(sign_request)
//         .deposit(NearToken::from_yoctonear(1))
//         .max_gas()
//         .transact_async()
//         .await?
//         .await?
//         .json()?;
//     Ok(sign_response)
// }

// pub async fn call_public_key(
//     worker: &Worker<impl near_workspaces::Network + 'static>,
//     contract: AccountId,
//     signer: InMemorySigner,
// ) -> Result<PublicKey, Box<dyn std::error::Error>> {
//     let public_key_response: PublicKey = worker
//         .call(&signer, &contract, "public_key")
//         .max_gas()
//         .transact_async()
//         .await?
//         .await?
//         .json()?;
//     Ok(public_key_response)
// }

// #[cfg(test)]
// mod tests {
//     use dotenv::dotenv;
//     use std::env;
//     use multichain_tools_rs::kdf::{derive_child_public_key, naj_pk_to_verifying_key};
//     use near_workspaces::network::Testnet;

//     use super::*;
//     use near_workspaces::types::AccountId;
//     use near_workspaces::types::SecretKey;

//     struct TestEnv {
//         account_id: AccountId,
//         contract_id: AccountId,
//         worker: near_workspaces::Worker<Testnet>,
//         signer: near_workspaces::InMemorySigner,
//     }

//     async fn setup() -> TestEnv {
//         dotenv().ok();

//         let worker = near_workspaces::testnet().await.unwrap();

//         let account_id: AccountId = env::var("NEAR_ACCOUNT_ID").unwrap().parse().unwrap();
//         let contract_id: AccountId = env::var("CHAIN_SIGNATURE_CONTRACT")
//             .unwrap()
//             .parse()
//             .unwrap();
//         let secret_key: SecretKey = env::var("NEAR_PRIVATE_KEY").unwrap().parse().unwrap();
//         let signer =
//             near_workspaces::InMemorySigner::from_secret_key(account_id.clone(), secret_key);

//         TestEnv {
//             account_id,
//             contract_id,
//             worker,
//             signer,
//         }
//     }

//     #[tokio::test]
//     async fn test_contract_calls() -> Result<(), Box<dyn std::error::Error>> {
//         let test_env = setup().await;

//         // Test call_public_key
//         let public_key = call_public_key(
//             &test_env.worker,
//             test_env.contract_id.clone(),
//             test_env.signer.clone(),
//         )
//         .await?;

//         let verifying_key = naj_pk_to_verifying_key(String::from(&public_key).as_str())?;

//         let _ = derive_child_public_key(
//             &verifying_key,
//             test_env.account_id.to_string(),
//             "path".to_string(),
//         )?;

//         Ok(())
//     }
// }
