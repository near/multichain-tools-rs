use k256::ecdsa::{Error, VerifyingKey};
use near_sdk::bs58;

use ethers_core::k256::{
    elliptic_curve::scalar::FromUintUnchecked,
    sha2::{Digest, Sha256},
    AffinePoint, Scalar, U256,
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

pub fn derive_public_key(
    public_key: &VerifyingKey,
    predecessor: String,
    path: String,
) -> Result<VerifyingKey, Error> {
    let epsilon = derive_epsilon(predecessor, path);

    let new_public_key = (AffinePoint::GENERATOR * epsilon + public_key.as_affine()).to_affine();
    VerifyingKey::from_affine(new_public_key)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use near_workspaces::types::SecretKey;

    use dotenv::dotenv;
    use near_workspaces::types::AccountId;
    use std::env;

    use super::*;
    #[tokio::test]
    async fn test_naj_pk_to_uncompressed_hex_point() {
        dotenv().ok();

        let worker = near_workspaces::testnet().await.unwrap();
        let contract_id: AccountId = "v1.signer-dev.testnet".parse().unwrap();

        let account_id =
            AccountId::from_str(env::var("NEAR_ACCOUNT_ID").unwrap().as_str()).unwrap();
        let secret_key =
            SecretKey::from_str(env::var("NEAR_PRIVATE_KEY").unwrap().as_str()).unwrap();
        let signer =
            near_workspaces::InMemorySigner::from_secret_key(account_id.clone(), secret_key);

        let result: String = worker
            .call(&signer, &contract_id, "public_key")
            .max_gas()
            .transact()
            .await
            .unwrap()
            .json()
            .unwrap();

        let verifying_key = naj_pk_to_verifying_key(&result);
        assert!(verifying_key.is_ok());
        let verifying_key = verifying_key.unwrap();

        let derived_pk =
            derive_public_key(&verifying_key, account_id.to_string(), "path".to_string());

        assert!(derived_pk.is_ok());
    }
}
