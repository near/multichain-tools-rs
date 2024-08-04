pub mod utils;

// pub use crate::utils::kdf;

use near_sdk::{
    env, ext_contract, near, AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseError,
};

use crate::utils::types::{SignRequest, SignatureResponse};

const ONE_YOCTO_NEAR: NearToken = NearToken::from_yoctonear(1);

#[ext_contract(ext_signature_contract)]
pub trait SignatureContract {
    fn sign(&mut self, request: SignRequest) -> Promise;
    fn public_key(&self) -> String;
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct CrossContractCaller {}

#[near]
impl CrossContractCaller {
    #[init]
    #[private]
    pub fn init() -> Self {
        Self {}
    }

    #[private]
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        Self {}
    }

    pub fn call_sign(&self, contract_id: AccountId, sign_request: SignRequest) -> Promise {
        let promise = ext_signature_contract::ext(contract_id)
            .with_static_gas(Gas::from_tgas(250))
            .with_attached_deposit(ONE_YOCTO_NEAR)
            .sign(sign_request);

        promise.then(
            Self::ext(env::current_account_id())
                .with_static_gas(Gas::from_tgas(5))
                .callback_sign(),
        )
    }

    #[private]
    pub fn callback_sign(
        &self,
        #[callback_result] call_result: Result<SignatureResponse, PromiseError>,
    ) -> SignatureResponse {
        if let Ok(signature_response) = call_result {
            signature_response
        } else {
            env::panic_str("Failed to call sign function")
        }
    }

    pub fn call_public_key(&self, contract_id: AccountId) -> Promise {
        let promise = ext_signature_contract::ext(contract_id).public_key();

        promise.then(Self::ext(env::current_account_id()).callback_public_key())
    }

    #[private]
    pub fn callback_public_key(
        &self,
        #[callback_result] call_result: Result<String, PromiseError>,
    ) -> String {
        match call_result {
            Ok(public_key) => public_key,
            Err(_) => env::panic_str("Failed to call public_key function"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::sha2::{Digest, Sha256};
    use near_workspaces::{types::SecretKey, Contract};

    use dotenv::dotenv;
    use serde_json::json;
    use std::env;

    async fn init() -> anyhow::Result<(Contract, AccountId, AccountId)> {
        dotenv().ok();

        let worker = near_workspaces::testnet().await?;
        let account_id: AccountId = env::var("NEAR_ACCOUNT_ID").unwrap().parse().unwrap();
        let contract_id: AccountId = env::var("CHAIN_SIGNATURE_CONTRACT")
            .unwrap()
            .parse()
            .unwrap();
        let secret_key: SecretKey = env::var("NEAR_PRIVATE_KEY").unwrap().parse().unwrap();

        let contract = worker
            .create_tla_and_deploy(
                account_id.clone(),
                secret_key,
                include_bytes!("../target/wasm32-unknown-unknown/release/multichain_tools_rs.wasm"),
            )
            .await?
            .unwrap();

        Ok((contract, account_id, contract_id))
    }

    #[tokio::test]
    async fn test_sign() -> anyhow::Result<()> {
        let (contract, _, contract_id) = init().await?;

        // Ensure the contract is migrated
        let result = contract.call("migrate").max_gas().transact().await?;
        assert!(result.is_success());

        // Prepare test data
        let args = SignRequest {
            payload: Sha256::digest("Hello, World!".as_bytes()).into(),
            path: "m/44'/60'/0'/0/0".to_string(),
            key_version: 0,
        };

        let result = contract
            .call("call_sign")
            .args_json(json!({
                "contract_id": contract_id,
                "sign_request": args
            }))
            .max_gas()
            .transact()
            .await?;

        assert!(result.is_success());

        let result: SignatureResponse = result.json().unwrap();

        println!("Sign result: {:?}", result);

        Ok(())
    }

    #[tokio::test]
    async fn test_call_public_key() -> anyhow::Result<()> {
        let (contract, _, contract_id) = init().await?;

        let result = contract.call("migrate").max_gas().transact().await?;
        assert!(result.is_success());

        let result = contract
            .call("call_public_key")
            .args_json(json!({
                "contract_id":contract_id
            }))
            .max_gas()
            .transact()
            .await?;

        assert!(result.is_success());

        let result: String = result.json().unwrap();

        assert!(result.contains("secp256k1:"));

        Ok(())
    }
}
