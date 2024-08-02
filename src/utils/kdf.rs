use near_sdk::bs58;

// Ex: secp256k1:54hU5wcCmVUPFWLDALXMh1fFToZsVXrx9BbTbHzSfQq1Kd1rJZi52iPa4QQxo6s5TgjWqgpY8HamYuUDzG6fAaUq
pub fn naj_pk_to_uncompressed_hex_point(root_pk: &str) -> Vec<u8> {
    let root_pk = root_pk.split(":").nth(1).unwrap();
    let root_pk = bs58::decode(root_pk).into_vec().unwrap();
    let mut sec1_key = vec![0x04];
    sec1_key.extend_from_slice(&root_pk);
    sec1_key
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
        let signer = near_workspaces::InMemorySigner::from_secret_key(account_id, secret_key);

        let result: String = worker
            .call(&signer, &contract_id, "public_key")
            .max_gas()
            .transact()
            .await
            .unwrap()
            .json()
            .unwrap();

        let uncompressed_hex_point = naj_pk_to_uncompressed_hex_point(&result);
        assert_eq!(uncompressed_hex_point.len(), 65);
        assert_eq!(uncompressed_hex_point[0], 0x04);
    }
}
