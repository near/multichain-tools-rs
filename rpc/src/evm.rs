use ethers_core::{
    k256::elliptic_curve::point::AffineCoordinates,
    types::{
        transaction::{eip2718::TypedTransaction, eip2930::AccessList},
        BlockNumber, Eip1559TransactionRequest, H160, H256, U256,
    },
};
use ethers_providers::{JsonRpcClient, Middleware, Provider};
use near_jsonrpc_client::JsonRpcClient as NearJsonRpcClient;
use near_sdk::AccountId;
use utils::{
    kdf::derive_eth_address,
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

    pub async fn send_signed_transaction(
        &self,
        transaction: TypedTransaction,
        signature: ethers_core::types::Signature,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        let signed_tx = transaction.rlp_signed(&signature);

        match self.evm_provider.send_raw_transaction(signed_tx).await {
            Ok(tx_hash) => Ok(tx_hash.tx_hash()),
            Err(e) => {
                eprintln!("Error sending transaction: {:?}", e);
                Err(Box::new(e))
            }
        }
    }

    pub async fn get_fee_properties(&self) -> Result<(U256, U256), Box<dyn std::error::Error>> {
        let latest_block = self
            .evm_provider
            .get_block(BlockNumber::Latest)
            .await?
            .ok_or("Latest block not found")?;

        let base_fee_per_gas = latest_block.base_fee_per_gas.unwrap_or_default();
        let max_priority_fee_per_gas = U256::from(1_000_000_000);
        let max_fee_per_gas = base_fee_per_gas + max_priority_fee_per_gas;

        Ok((max_fee_per_gas, max_priority_fee_per_gas))
    }

    pub async fn attach_gas_and_nonce(
        &self,
        transaction: &TypedTransaction,
        from: &str,
    ) -> Result<TypedTransaction, Box<dyn std::error::Error>> {
        let (max_fee_per_gas, max_priority_fee_per_gas) = self.get_fee_properties().await?;
        let nonce = self.evm_provider.get_transaction_count(from, None).await?;
        let gas_estimate = self
            .evm_provider
            .estimate_gas(&transaction.clone(), None)
            .await?;

        Ok(TypedTransaction::Eip1559(
            Eip1559TransactionRequest::new()
                .from(from.parse::<H160>().unwrap())
                .to(transaction.to().cloned().unwrap())
                .gas(gas_estimate)
                .value(transaction.value().cloned().unwrap_or_default())
                .data(transaction.data().cloned().unwrap_or_default())
                .nonce(nonce)
                .access_list(AccessList::default())
                .max_priority_fee_per_gas(max_priority_fee_per_gas)
                .max_fee_per_gas(max_fee_per_gas)
                .chain_id(self.evm_provider.get_chainid().await?.as_u64()),
        ))
    }

    pub async fn get_balance(&self, address: &str) -> Result<String, Box<dyn std::error::Error>> {
        let balance = self.evm_provider.get_balance(address, None).await?;
        Ok(ethers_core::utils::format_ether(balance))
    }

    pub async fn derive_address(
        &self,
        signer_id: &str,
        path: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let naj_public_key = call_public_key(&self.near_client, self.contract.clone()).await?;
        let eth_address =
            derive_eth_address(&naj_public_key, signer_id.to_string(), path.to_string()).await?;
        Ok(eth_address)
    }

    pub async fn handle_transaction(
        &self,
        data: TypedTransaction,
        path: String,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        let from = self
            .derive_address(&self.near_authentication.account_id.to_string(), &path)
            .await?;
        let transaction = self.attach_gas_and_nonce(&data, &from).await?;

        let sign_request = SignRequest {
            payload: transaction.sighash().into(),
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
            r: U256::from_big_endian(&signature.big_r.affine_point.x() as &[u8]),
            s: U256::from_big_endian(&signature.s.scalar.to_bytes() as &[u8]),
            v: signature.recovery_id.into(),
        };

        self.send_signed_transaction(transaction, ethers_signature)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use ethers_core::types::U256;
    use ethers_providers::Http;
    use near_crypto::{InMemorySigner, SecretKey};
    use near_primitives::types::AccountId;
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
        let eth_rpc_url = std::env::var("ETH_SEPOLIA_RPC_URL").unwrap();

        let evm = EVM::new(
            Provider::<Http>::try_from(eth_rpc_url).unwrap(),
            NearAuthentication {
                network: NearNetwork::Testnet,
                account_id: account_id.clone(),
                key_pair: InMemorySigner::from_secret_key(account_id, private_key),
            },
            contract_id,
        );

        let transaction_request = TypedTransaction::Eip1559(
            Eip1559TransactionRequest::new()
                .to("0x4174678c78fEaFd778c1ff319D5D326701449b25"
                    .parse::<ethers_core::types::NameOrAddress>()
                    .unwrap())
                .value(U256::from(3500000000000000u64)),
        );

        let result = evm
            .handle_transaction(transaction_request, "eth".to_string())
            .await;

        assert!(result.is_ok());

        println!("Tx hash: {:?}", result.unwrap());
    }
}
