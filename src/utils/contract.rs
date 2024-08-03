// use super::types::{SignRequest, SignatureResponse};
// use near_sdk::{AccountId, NearToken, PublicKey};

// pub async fn call_sign(
//     contract: &AccountId,
//     sign_request: serde_json::Value,
// ) -> Result<SignatureResponse, Box<dyn std::error::Error>> {
//     let sign_response: SignatureResponse = contract
//         .call("sign")
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
//     contract: &AccountId,
// ) -> Result<PublicKey, Box<dyn std::error::Error>> {
//     let public_key_response: PublicKey = contract
//         .call("public_key")
//         .max_gas()
//         .transact_async()
//         .await?
//         .await?
//         .json()?;
//     Ok(public_key_response)
// }
