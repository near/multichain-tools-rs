// use near_sdk::{
//     env, ext_contract, near, near_bindgen, AccountId, Gas, NearToken, PanicOnDefault, Promise,
//     PromiseError,
// };

// use crate::utils::types::{SignRequest, SignatureResponse};

// const SIGN_GAS: Gas = Gas::from_tgas(300);
// const SIGN_DEPOSIT: NearToken = NearToken::from_yoctonear(1);

// #[ext_contract(ext_signature_contract)]
// pub trait SignatureContract {
//     fn sign(&mut self, request: SignRequest) -> Promise;
//     fn public_key(&self) -> String;
// }

// #[near(contract_state)]
// #[derive(PanicOnDefault)]
// pub struct CrossContractCaller {}

// #[near]
// impl CrossContractCaller {
// pub fn call_sign(&self, contract_id: AccountId, sign_request: SignRequest) -> Promise {
//     let promise = ext_signature_contract::ext(contract_id)
//         .with_static_gas(SIGN_GAS)
//         .with_attached_deposit(SIGN_DEPOSIT)
//         .sign(sign_request);

//     promise.then(
//         Self::ext(env::current_account_id())
//             .with_static_gas(Gas::from_tgas(5))
//             .callback_sign(),
//     )
// }

// #[private]
// pub fn callback_sign(
//     &self,
//     #[callback_result] call_result: Result<SignatureResponse, PromiseError>,
// ) -> SignatureResponse {
//     if let Ok(signature_response) = call_result {
//         signature_response
//     } else {
//         env::panic_str("Failed to call sign function")
//     }
// }

//     pub fn call_public_key(&self, contract_id: AccountId) -> Promise {
//         let promise = ext_signature_contract::ext(contract_id).public_key();

//         promise.then(Self::ext(env::current_account_id()).callback_public_key())
//     }

//     #[private]
//     pub fn callback_public_key(
//         &self,
//         #[callback_result] call_result: Result<String, PromiseError>,
//     ) -> String {
//         if let Ok(public_key) = call_result {
//             println!("Public key: {}", public_key);
//             public_key
//         } else {
//             env::panic_str("Failed to call public_key function")
//         }
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use near_workspaces::{Account, Contract, DevNetwork, Worker};
//     use serde_json::json;

//     async fn init(worker: &Worker<impl DevNetwork>) -> anyhow::Result<(Contract, Account)> {
//         // Deploy the contract
//         let contract = worker
//             .dev_deploy(include_bytes!(
//                 "../../target/wasm32-unknown-unknown/release/cross_contract_caller.wasm"
//             ))
//             .await?;

//         // Create a test account
//         let account = worker.dev_create_account().await?;

//         // Initialize the contract
//         contract.call("new").transact().await?;

//         Ok((contract, account))
//     }

//     #[tokio::test]
//     async fn test_call_public_key() -> anyhow::Result<()> {
//         let worker = near_workspaces::testnet().await?;
//         let (contract, account) = init(&worker).await?;

//         // Create a mock signature contract
//         let signature_contract = worker.dev_deploy(&[]).await?;

//         // Call the public_key method
//         let result = contract
//             .call("call_public_key")
//             .args_json(json!({
//                 "contract_id": signature_contract.id()
//             }))
//             .max_gas()
//             .transact()
//             .await?;

//         // Assert that the call was successful
//         assert!(result.is_success());

//         // You can add more specific assertions here based on the expected behavior
//         // For example, checking the returned public key or any logs
//         println!("Call result: {:?}", result);

//         Ok(())
//     }
// }
