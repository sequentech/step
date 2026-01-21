// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

quick_error! {
    #[derive(Debug, PartialEq, Eq)]
    pub enum BallotError {
        ParseBigUint(uint_str: String, message: String) {}
        CryptographicCheck(message: String) {}
        ConsistencyCheck(message: String) {}
        Serialization(message: String) {}
    }
}
