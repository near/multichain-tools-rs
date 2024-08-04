use ethers_core::k256::{elliptic_curve::scalar::FromUintUnchecked, AffinePoint, Scalar, U256};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct SignRequest {
    pub payload: [u8; 32],
    pub path: String,
    pub key_version: u32,
}

pub trait ScalarExt {
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl ScalarExt for Scalar {
    fn from_bytes(bytes: &[u8]) -> Self {
        Scalar::from_uint_unchecked(U256::from_be_slice(bytes))
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, JsonSchema)]
pub struct SerializableScalar {
    #[schemars(with = "String")]
    pub scalar: Scalar,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, JsonSchema)]
pub struct SerializableAffinePoint {
    #[schemars(with = "String")]
    pub affine_point: AffinePoint,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema)]
pub struct SignatureResponse {
    pub big_r: SerializableAffinePoint,
    pub s: SerializableScalar,
    pub recovery_id: u8,
}
