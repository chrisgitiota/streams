//! Stream Errors

// Rust
use alloc::{
    boxed::Box,
    string::{FromUtf8Error, String},
};
use core::fmt::Debug;

// 3rd-party
use hex::FromHexError;
use thiserror_no_std::Error;

// IOTA
use spongos::error::Error as SpongosError;

use crate::address::Address;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
#[cfg(feature = "did")]
pub enum IdentityError {
    #[error("{0}")]
    Core(identity_iota::core::Error),
    #[error("{0}")]
    Error(identity_iota::did::Error),
    #[error("{0}")]
    DIDError(identity_iota::did::DIDError),
    #[error("{0}")]
    IotaCore(identity_iota::iota_core::Error),
    #[error("{0}")]
    IotaClient(identity_iota::client::Error),
    #[error("{0}")]
    Other(String),
}

#[cfg(feature = "did")]
impl From<identity_iota::core::Error> for IdentityError {
    fn from(error: identity_iota::core::Error) -> Self {
        Self::Core(error)
    }
}

#[cfg(feature = "did")]
impl From<identity_iota::did::Error> for IdentityError {
    fn from(error: identity_iota::did::Error) -> Self {
        Self::Error(error)
    }
}

#[cfg(feature = "did")]
impl From<identity_iota::did::DIDError> for IdentityError {
    fn from(error: identity_iota::did::DIDError) -> Self {
        Self::DIDError(error)
    }
}

#[cfg(feature = "did")]
impl From<identity_iota::iota_core::Error> for IdentityError {
    fn from(error: identity_iota::iota_core::Error) -> Self {
        Self::IotaCore(error)
    }
}

#[cfg(feature = "did")]
impl From<identity_iota::client::Error> for IdentityError {
    fn from(error: identity_iota::client::Error) -> Self {
        Self::IotaClient(error)
    }
}

#[cfg(feature = "did")]
impl From<String> for IdentityError {
    fn from(error: String) -> Self {
        Self::Other(error)
    }
}

#[derive(Debug, Error)]
/// Error type of the LETS crate.
#[allow(clippy::large_enum_variant)]
pub enum Error {
    #[error("Crypto error hile attempting to {0}: {1}")]
    Crypto(&'static str, crypto::Error),

    #[cfg(feature = "did")]
    #[error("Encountered DID error while trying to {0}; Error: {1}")]
    Did(&'static str, IdentityError),

    #[error("{0} is not encoded in {1} or the encoding is incorrect: {2:?}")]
    Encoding(&'static str, &'static str, Box<Error>),

    #[error("External error: {0:?}")]
    External(anyhow::Error),

    #[error("{0} must be {1} bytes long, but is {2} bytes long instead")]
    InvalidSize(&'static str, usize, u64),

    #[error("Malformed {0}: missing '{1}' for {2}")]
    Malformed(&'static str, &'static str, String),

    #[error("There was an issue with {0} the signature, cannot {1}")]
    Signature(&'static str, &'static str),

    #[error("Internal Spongos error: {0}")]
    Spongos(SpongosError),

    /// Transport

    #[error("Transport error for address {1}: {0}")]
    AddressError(&'static str, Address),

    #[cfg(any(feature = "tangle-client", feature = "tangle-client-wasm"))]
    #[error("Iota client error for {0}: {1}")]
    IotaClient(&'static str, iota_sdk::client::error::Error),

    #[error("message '{0}' not found in {1}")]
    MessageMissing(Address, &'static str),

    #[error("Nonce is not in the range 0..u32::MAX range for target score: {0}")]
    Nonce(f64),

    #[cfg(feature = "utangle-client")]
    #[error("Request HTTP error: {0}")]
    Request(reqwest::Error),
}

impl Error {
    #[cfg(feature = "did")]
    pub fn did<T: Into<IdentityError>>(did: &'static str, e: T) -> Self {
        Self::Did(did, e.into())
    }

    pub fn utf(m: &'static str, error: FromUtf8Error) -> Self {
        Self::Encoding(m, "utf8", Box::new(Self::External(error.into())))
    }
}

impl From<SpongosError> for Error {
    fn from(error: SpongosError) -> Self {
        Self::Spongos(error)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(error: FromUtf8Error) -> Self {
        Self::utf("string", error)
    }
}

impl From<FromHexError> for Error {
    fn from(error: FromHexError) -> Self {
        Self::Encoding("string", "hex", Box::new(Self::External(error.into())))
    }
}

#[cfg(feature = "utangle-client")]
impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::Request(error)
    }
}
