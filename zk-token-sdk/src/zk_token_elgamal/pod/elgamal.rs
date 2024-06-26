//! Plain Old Data types for the ElGamal encryption scheme.

use {
    crate::zk_token_elgamal::pod::{Pod, Zeroable},
    base64::{prelude::BASE64_STANDARD, Engine},
    std::fmt,
};
#[cfg(not(target_os = "wasi"))]
use {
    crate::{encryption::elgamal as decoded, errors::ProofError},
    curve25519_dalek::ristretto::CompressedRistretto,
};

/// The `ElGamalCiphertext` type as a `Pod`.
#[derive(Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
#[repr(transparent)]
pub struct ElGamalCiphertext(pub [u8; 64]);

impl fmt::Debug for ElGamalCiphertext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl fmt::Display for ElGamalCiphertext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", BASE64_STANDARD.encode(self.0))
    }
}

impl Default for ElGamalCiphertext {
    fn default() -> Self {
        Self::zeroed()
    }
}

#[cfg(not(target_os = "wasi"))]
impl From<decoded::ElGamalCiphertext> for ElGamalCiphertext {
    fn from(decoded_ciphertext: decoded::ElGamalCiphertext) -> Self {
        Self(decoded_ciphertext.to_bytes())
    }
}

#[cfg(not(target_os = "wasi"))]
impl TryFrom<ElGamalCiphertext> for decoded::ElGamalCiphertext {
    type Error = ProofError;

    fn try_from(pod_ciphertext: ElGamalCiphertext) -> Result<Self, Self::Error> {
        Self::from_bytes(&pod_ciphertext.0).ok_or(ProofError::CiphertextDeserialization)
    }
}

/// The `ElGamalPubkey` type as a `Pod`.
#[derive(Clone, Copy, Default, Pod, Zeroable, PartialEq, Eq)]
#[repr(transparent)]
pub struct ElGamalPubkey(pub [u8; 32]);

impl fmt::Debug for ElGamalPubkey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl fmt::Display for ElGamalPubkey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", BASE64_STANDARD.encode(self.0))
    }
}

#[cfg(not(target_os = "wasi"))]
impl From<decoded::ElGamalPubkey> for ElGamalPubkey {
    fn from(decoded_pubkey: decoded::ElGamalPubkey) -> Self {
        Self(decoded_pubkey.to_bytes())
    }
}

#[cfg(not(target_os = "wasi"))]
impl TryFrom<ElGamalPubkey> for decoded::ElGamalPubkey {
    type Error = ProofError;

    fn try_from(pod_pubkey: ElGamalPubkey) -> Result<Self, Self::Error> {
        Self::from_bytes(&pod_pubkey.0).ok_or(ProofError::CiphertextDeserialization)
    }
}

/// The `DecryptHandle` type as a `Pod`.
#[derive(Clone, Copy, Default, Pod, Zeroable, PartialEq, Eq)]
#[repr(transparent)]
pub struct DecryptHandle(pub [u8; 32]);

impl fmt::Debug for DecryptHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[cfg(not(target_os = "wasi"))]
impl From<decoded::DecryptHandle> for DecryptHandle {
    fn from(decoded_handle: decoded::DecryptHandle) -> Self {
        Self(decoded_handle.to_bytes())
    }
}

// For proof verification, interpret pod::DecryptHandle as CompressedRistretto
#[cfg(not(target_os = "wasi"))]
impl From<DecryptHandle> for CompressedRistretto {
    fn from(pod_handle: DecryptHandle) -> Self {
        Self(pod_handle.0)
    }
}

#[cfg(not(target_os = "wasi"))]
impl TryFrom<DecryptHandle> for decoded::DecryptHandle {
    type Error = ProofError;

    fn try_from(pod_handle: DecryptHandle) -> Result<Self, Self::Error> {
        Self::from_bytes(&pod_handle.0).ok_or(ProofError::CiphertextDeserialization)
    }
}
