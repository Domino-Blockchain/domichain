//! Plain Old Data type for the Pedersen commitment scheme.

use {
    crate::zk_token_elgamal::pod::{Pod, Zeroable},
    std::fmt,
};
#[cfg(not(target_os = "wasi"))]
use {
    crate::{encryption::pedersen as decoded, errors::ProofError},
    curve25519_dalek::ristretto::CompressedRistretto,
};

/// The `PedersenCommitment` type as a `Pod`.
#[derive(Clone, Copy, Default, Pod, Zeroable, PartialEq, Eq)]
#[repr(transparent)]
pub struct PedersenCommitment(pub [u8; 32]);

impl fmt::Debug for PedersenCommitment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[cfg(not(target_os = "wasi"))]
impl From<decoded::PedersenCommitment> for PedersenCommitment {
    fn from(decoded_commitment: decoded::PedersenCommitment) -> Self {
        Self(decoded_commitment.to_bytes())
    }
}

// For proof verification, interpret pod::PedersenCommitment directly as CompressedRistretto
#[cfg(not(target_os = "wasi"))]
impl From<PedersenCommitment> for CompressedRistretto {
    fn from(pod_commitment: PedersenCommitment) -> Self {
        Self(pod_commitment.0)
    }
}

#[cfg(not(target_os = "wasi"))]
impl TryFrom<PedersenCommitment> for decoded::PedersenCommitment {
    type Error = ProofError;

    fn try_from(pod_commitment: PedersenCommitment) -> Result<Self, Self::Error> {
        Self::from_bytes(&pod_commitment.0).ok_or(ProofError::CiphertextDeserialization)
    }
}
