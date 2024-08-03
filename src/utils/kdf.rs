use k256::ecdsa::{Error, VerifyingKey};
use near_sdk::bs58;

use ethers_core::{
    k256::{
        elliptic_curve::scalar::FromUintUnchecked,
        sha2::{Digest, Sha256},
        AffinePoint, Scalar, U256,
    },
    utils::{hex, keccak256},
};

// Ex: secp256k1:54hU5wcCmVUPFWLDALXMh1fFToZsVXrx9BbTbHzSfQq1Kd1rJZi52iPa4QQxo6s5TgjWqgpY8HamYuUDzG6fAaUq
pub fn naj_pk_to_verifying_key(root_pk: &str) -> Result<VerifyingKey, Error> {
    let root_pk = root_pk.split(":").nth(1).expect("Invalid root public key");
    let root_pk = bs58::decode(root_pk).into_vec().unwrap();
    let mut sec1_key = vec![0x04];
    sec1_key.extend_from_slice(&root_pk);
    VerifyingKey::from_sec1_bytes(&sec1_key)
}

pub fn derive_epsilon(predecessor: String, path: String) -> Scalar {
    let mut hasher = Sha256::new();
    hasher.update(format!(
        "near-mpc-recovery v0.1.0 epsilon derivation:{predecessor},{path}"
    ));

    Scalar::from_uint_unchecked(U256::from_le_slice(&hasher.finalize()))
}

pub fn derive_child_public_key(
    public_key: &VerifyingKey,
    predecessor: String,
    path: String,
) -> Result<VerifyingKey, Error> {
    let epsilon = derive_epsilon(predecessor, path);

    let new_public_key = (AffinePoint::GENERATOR * epsilon + public_key.as_affine()).to_affine();
    VerifyingKey::from_affine(new_public_key)
}

pub fn derive_eth_address(public_key: &VerifyingKey) -> String {
    let encoded_point = public_key.to_encoded_point(false);
    let address = &keccak256(&encoded_point.as_bytes()[1..])[12..];
    hex::encode(address)
}

#[cfg(test)]
mod tests {
    use near_workspaces::{network::Testnet, types::SecretKey};

    use dotenv::dotenv;
    use near_workspaces::types::AccountId;
    use std::env;

    use super::*;

    async fn setup() -> TestEnv {
        dotenv().ok();

        let account_id: AccountId = env::var("NEAR_ACCOUNT_ID").unwrap().parse().unwrap();
        let secret_key: SecretKey = env::var("NEAR_PRIVATE_KEY").unwrap().parse().unwrap();
        let contract_id: AccountId = env::var("CHAIN_SIGNATURE_CONTRACT")
            .unwrap()
            .parse()
            .unwrap();

        let worker = near_workspaces::testnet().await.unwrap();
        let signer =
            near_workspaces::InMemorySigner::from_secret_key(account_id.clone(), secret_key);

        TestEnv {
            account_id,
            contract_id,
            worker,
            signer,
        }
    }

    struct TestEnv {
        account_id: AccountId,
        contract_id: AccountId,
        worker: near_workspaces::Worker<Testnet>,
        signer: near_workspaces::InMemorySigner,
    }

    #[tokio::test]
    async fn test_naj_pk_to_uncompressed_hex_point() {
        let test_env = setup().await;

        let result: String = test_env
            .worker
            .call(&test_env.signer, &test_env.contract_id, "public_key")
            .max_gas()
            .transact()
            .await
            .unwrap()
            .json()
            .unwrap();

        let verifying_key = naj_pk_to_verifying_key(&result);
        assert!(verifying_key.is_ok());
        let verifying_key = verifying_key.unwrap();

        let child_pk = derive_child_public_key(
            &verifying_key,
            test_env.account_id.to_string(),
            "path".to_string(),
        );

        assert!(child_pk.is_ok());
    }
}
