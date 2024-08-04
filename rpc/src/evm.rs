use ethers_core::{
    k256::elliptic_curve::point::AffineCoordinates,
    types::{transaction::eip2930::AccessList, BlockNumber, Eip1559TransactionRequest, U256},
    utils::keccak256,
};
use ethers_providers::{JsonRpcClient, Middleware, Provider};
use near_jsonrpc_client::JsonRpcClient as NearJsonRpcClient;
use near_sdk::AccountId;
use utils::{
    kdf::{derive_child_public_key, derive_eth_address, naj_pk_to_verifying_key},
    types::{NearAuthentication, SignRequest},
};

use crate::{
    api::get_near_client,
    rpc::{call_public_key, call_sign},
};

pub struct EVM<P: JsonRpcClient> {
    evm_provider: Provider<P>,
    near_authentication: NearAuthentication,
    contract: AccountId,
    near_client: NearJsonRpcClient,
}

impl<P: JsonRpcClient> EVM<P> {
    pub fn new(
        evm_provider: Provider<P>,
        near_authentication: NearAuthentication,
        contract: AccountId,
    ) -> Self {
        Self {
            evm_provider,
            near_authentication: near_authentication.clone(),
            contract,
            near_client: get_near_client(near_authentication.network),
        }
    }

    pub fn prepare_transaction_for_signature(transaction: &Eip1559TransactionRequest) -> [u8; 32] {
        let serialized_transaction = transaction.rlp().to_vec();
        keccak256(serialized_transaction)
    }

    pub async fn send_signed_transaction(
        &self,
        transaction: Eip1559TransactionRequest,
        signature: ethers_core::types::Signature,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let signed_tx = transaction.rlp_signed(&signature);
        self.evm_provider
            .send_raw_transaction(signed_tx.into())
            .await?;
        Ok(())
    }

    pub async fn get_fee_properties(&self) -> Result<(U256, U256), Box<dyn std::error::Error>> {
        // Get the latest block
        let latest_block = self
            .evm_provider
            .get_block(BlockNumber::Latest)
            .await?
            .unwrap();

        // Calculate max_fee_per_gas and max_priority_fee_per_gas
        let base_fee_per_gas = latest_block.base_fee_per_gas.unwrap_or_default();
        let max_priority_fee_per_gas = U256::from(1_000_000_000);
        let max_fee_per_gas = base_fee_per_gas + max_priority_fee_per_gas;

        Ok((max_fee_per_gas, max_priority_fee_per_gas))
    }

    pub async fn attach_gas_and_nonce(
        &self,
        transaction: &Eip1559TransactionRequest,
        from: &str,
    ) -> Result<Eip1559TransactionRequest, Box<dyn std::error::Error>> {
        let (max_fee_per_gas, max_priority_fee_per_gas) = self.get_fee_properties().await?;
        let nonce = self.evm_provider.get_transaction_count(from, None).await?;

        Ok(Eip1559TransactionRequest {
            from: Some(from.parse()?),
            to: transaction.to.clone(),
            gas: None, // Let the provider estimate gas
            value: transaction.value,
            data: transaction.data.clone(),
            nonce: Some(nonce),
            access_list: AccessList::default(),
            max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
            max_fee_per_gas: Some(max_fee_per_gas),
            chain_id: Some(self.evm_provider.get_chainid().await?.as_u64().into()),
        })
    }

    pub async fn get_balance(&self, address: &str) -> Result<String, Box<dyn std::error::Error>> {
        let balance = self.evm_provider.get_balance(address, None).await?;
        Ok(ethers_core::utils::format_ether(balance))
    }

    pub async fn derive_address(&self, signer_id: &str, path: &str) -> String {
        let public_key = call_public_key(&self.near_client, self.contract.clone())
            .await
            .unwrap();
        println!("public_key: {:?}", public_key);
        let public_key = naj_pk_to_verifying_key(&public_key).unwrap();
        println!("naj_pk_to_verifying_key: {:?}", public_key);
        let child_public_key =
            derive_child_public_key(&public_key, signer_id.to_string(), path.to_string());
        println!("derive_child_public_key: {:?}", child_public_key);
        derive_eth_address(&child_public_key.unwrap())
    }

    pub async fn handle_transaction(
        &self,
        data: Eip1559TransactionRequest,
        path: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let from = self
            .derive_address(&self.near_authentication.account_id.to_string(), &path)
            .await;
        let transaction = self.attach_gas_and_nonce(&data, &from).await?;
        let transaction_hash = Self::prepare_transaction_for_signature(&transaction);

        let sign_request = SignRequest {
            payload: transaction_hash,
            path,
            key_version: 0,
        };

        let signature = call_sign(
            &self.near_client,
            self.contract.clone(),
            sign_request,
            self.near_authentication.key_pair.clone(),
        )
        .await?;

        let ethers_signature = ethers_core::types::Signature {
            r: U256::from_big_endian(signature.big_r.affine_point.x().as_slice()),
            s: U256::from_big_endian(signature.s.scalar.to_bytes().as_slice()),
            v: signature.recovery_id.into(),
        };

        let _ = self
            .send_signed_transaction(transaction, ethers_signature)
            .await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use ethers_core::types::{Address, NameOrAddress, U256, U64};
    use ethers_providers::Http;
    use near_crypto::{InMemorySigner, SecretKey};
    use near_primitives::types::AccountId;
    use std::str::FromStr;
    use utils::types::NearNetwork;

    #[tokio::test]
    async fn test_handle_transaction() {
        dotenv().ok();
        let account_id: AccountId = std::env::var("NEAR_ACCOUNT_ID").unwrap().parse().unwrap();
        let private_key: SecretKey = std::env::var("NEAR_PRIVATE_KEY").unwrap().parse().unwrap();
        let contract_id: AccountId = std::env::var("CHAIN_SIGNATURE_CONTRACT")
            .unwrap()
            .parse()
            .unwrap();

        // Create a mock EVM instance with Sepolia RPC URL
        let evm = EVM::new(
            Provider::<Http>::try_from(
                "https://sepolia.infura.io/v3/6df51ccaa17f4e078325b5050da5a2dd",
            )
            .unwrap(),
            NearAuthentication {
                network: NearNetwork::Testnet,
                account_id: account_id.clone(),
                key_pair: InMemorySigner::from_secret_key(account_id, private_key),
            },
            contract_id,
        );

        // Create a sample Eip1559TransactionRequest for Sepolia testnet
        let transaction_request = Eip1559TransactionRequest {
            to: Some(NameOrAddress::Address(
                Address::from_str("0x742d35Cc6634C0532925a3b844Bc454e4438f44e").unwrap(),
            )),
            value: Some(U256::from(1000000000000000u64)),
            data: None,
            nonce: None,
            access_list: AccessList::default(),
            max_priority_fee_per_gas: Some(U256::from(1500000000u64)),
            max_fee_per_gas: Some(U256::from(3000000000u64)),
            gas: Some(U256::from(21000u64)),
            chain_id: Some(U64::from(11155111u64)),
            ..Default::default()
        };

        let result = evm
            .handle_transaction(transaction_request, "eth".to_string())
            .await;

        assert!(result.is_ok());
    }
}
