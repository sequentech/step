// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

quick_error! {
    /// Errors raised while encoding, encrypting, or validating ballots.
    #[derive(Debug, PartialEq, Eq)]
    pub enum BallotError {
        /// A numeric ballot value could not be parsed from its string form.
        ParseBigUint(uint_str: String, message: String) {}
        /// A cryptographic proof or ciphertext check failed.
        CryptographicCheck(message: String) {}
        /// Ballot contents violate structural or policy constraints.
        ConsistencyCheck(message: String) {}
        /// JSON, base64, or borsh serialization/deserialization failed.
        Serialization(message: String) {}
    }
}
