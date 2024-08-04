use borsh::{BorshDeserialize, BorshSerialize};
use ethers_core::k256::{
    ecdsa::RecoveryId,
    elliptic_curve::{point::DecompressPoint, scalar::FromUintUnchecked},
    AffinePoint, Scalar, U256,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, JsonSchema, Debug)]
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

impl BorshSerialize for SerializableScalar {
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let to_ser: [u8; 32] = self.scalar.to_bytes().into();
        BorshSerialize::serialize(&to_ser, writer)
    }
}

impl BorshDeserialize for SerializableScalar {
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
        let from_ser: [u8; 32] = BorshDeserialize::deserialize_reader(reader)?;
        let scalar = Scalar::from_bytes(&from_ser[..]);
        Ok(SerializableScalar { scalar })
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, JsonSchema)]
pub struct SerializableAffinePoint {
    #[schemars(with = "String")]
    pub affine_point: AffinePoint,
}

impl BorshSerialize for SerializableAffinePoint {
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let to_ser: Vec<u8> = serde_json::to_vec(&self.affine_point)?;
        BorshSerialize::serialize(&to_ser, writer)
    }
}

impl BorshDeserialize for SerializableAffinePoint {
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
        let from_ser: Vec<u8> = BorshDeserialize::deserialize_reader(reader)?;
        let affine_point = serde_json::from_slice(&from_ser)?;
        Ok(SerializableAffinePoint { affine_point })
    }
}

#[derive(
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    JsonSchema,
)]
pub struct SignatureResponse {
    pub big_r: SerializableAffinePoint,
    pub s: SerializableScalar,
    pub recovery_id: u8,
}

impl SignatureResponse {
    #[must_use]
    pub fn new(r: [u8; 32], s: [u8; 32], v: RecoveryId) -> Option<Self> {
        let big_r = AffinePoint::decompress(&r.into(), u8::from(v.is_y_odd()).into()).unwrap();

        Some(Self {
            big_r: SerializableAffinePoint {
                affine_point: big_r,
            },
            s: SerializableScalar {
                scalar: Scalar::from_bytes(&s),
            },
            recovery_id: v.into(),
        })
    }

    #[must_use]
    pub fn from_ecdsa_signature(
        signature: ethers_core::k256::ecdsa::Signature,
        recovery_id: RecoveryId,
    ) -> Option<Self> {
        SignatureResponse::new(
            signature.r().to_bytes().into(),
            signature.s().to_bytes().into(),
            recovery_id,
        )
    }
}
