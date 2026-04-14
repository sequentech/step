// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

quick_error! {
    #[derive(Debug, PartialEq, Eq)]
    /// Errors related to ballot processing.
    pub enum BallotError {
        /// Error parsing a big unsigned integer from a string.
        ParseBigUint(uint_str: String, message: String) {}
        /// Error while cryptographic checks.
        CryptographicCheck(message: String) {}
        /// Error during consistency checks.
        ConsistencyCheck(message: String) {}
        /// Error during serialization or deserialization.
        Serialization(message: String) {}
    }
}
