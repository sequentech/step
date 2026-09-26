// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Error type for this library

use thiserror::Error;

/**
 * Error type for the cryptography module.
 *
 * This error type is used to represent all possible errors that can occur
 * within the cryptography module.
 */
#[derive(Error, Debug)]
pub enum Error {
    /// Deserialization error for [`crate::utils::serialization`] functionality
    #[error("{0}")]
    DeserializationError(String),

    /// Serialization error for [`crate::utils::serialization`] functionality
    #[error("{0}")]
    SerializationError(String),

    /// Occurs when deserializing with invalid length prefixes in [`crate::utils::serialization`] functionality
    #[error("Try from slice error: {0}")]
    DeserializationLengthError(#[from] std::array::TryFromSliceError),

    /// Occurs when deserializing with invalid length prefixes (`LengthU` conversion) in [`crate::utils::serialization`] functionality
    #[error("Try from int error: {0}")]
    DeserializationLengthIntError(#[from] std::num::TryFromIntError),

    /// Occurs when deserializing `ed2219_dalek` signatures or keys
    #[error("Signature error: {0}")]
    SignatureDeserError(#[from] ed25519_dalek::SignatureError),

    /// Occurs when [encoding][`crate::traits::groups::CryptographicGroup::encode_bytes`] to the curve fails
    #[error("{0}")]
    EncodingError(String),

    /// Occurs when [Naor-Yung][`crate::cryptosystem::naoryung::Ciphertext`] well-formedness proofs fail to verify.
    #[error("{0}")]
    NaorYungStripError(String),

    /// Occurs when Joint-Feldman DKG [share verification][`crate::dkgd::dealer::VerifiableShare`] fails.
    #[error("{0}")]
    ShareVerificationFailed(String),

    /// Occurs when [decryption proofs][`crate::dkgd::recipient::PartialDecryption`] fail to verify.
    #[error("{0}")]
    DecryptProofFailed(String),

    /// Occurs when a permutation is applied to a slice of mismatched length
    #[error("Mismatched permutation length")]
    MismatchedPermutationLength,

    /// Occurs when [`multi_exp`][`crate::traits::groups::GroupElement::multi_exp`]
    /// is given base and exponent lists of different lengths.
    #[error("Mismatched multi-exponentiation length: {0} bases, {1} exponents")]
    MismatchedMultiExpLength(usize, usize),

    /// Occurs when shuffling zero ciphertexts
    #[error("Empty shuffle")]
    EmptyShuffle,

    /// Occurs when there is a length mismatch in shuffle data
    #[error("Mismatched shuffle length")]
    MismatchedShuffleLength,

    /// Occurs when a hash to curve or hash to scalar error occurs in `p256`
    #[error("{0}")]
    HashToScalarError(#[from] p256::hash2curve::ExpandMsgXmdError),

    /// Occurs when a hash to curve or hash to scalar error occurs in `p256`
    #[error("{0}")]
    HashToElementError(String),

    /// Occurs when a scalar cannot be decoded from two elements
    #[error("{0}")]
    ScalarDecodeError(String),

    /// Occurs when randomness used to decrypt is invalid
    #[error("{0}")]
    DecryptionError(String),

    /// Occurs when symmetric encryption fails
    #[error("{0}")]
    EncryptionError(String),

    /// Wraps another error with additional context
    #[error("{0}: {1}")]
    WrappedError(String, Box<Error>),
}

/// Attaches a contextual string to an Error.
pub trait ErrorContext<T> {
    /// Attaches a contextual string to an Error.
    ///
    /// # Errors
    ///
    /// Returns the wrapped error with context if the result is an error.
    fn with_context(self, context: &str) -> Result<T, Error>;
}
impl<T> ErrorContext<T> for Result<T, Error> {
    /// Attaches a contextual string to an Error.
    fn with_context(self, context: &str) -> Result<T, Error> {
        if let Err(e) = self {
            Err(Error::WrappedError(context.to_string(), Box::new(e)))
        } else {
            self
        }
    }
}
