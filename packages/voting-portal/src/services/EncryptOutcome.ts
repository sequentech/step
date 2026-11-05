// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** What came of encrypting a ballot for review. */
export enum EEncryptOutcome {
    ENCRYPTED = "encrypted",
    FAILED = "failed",
    /** The Vitrine check refused the configuration; `boothRefusal` says why. */
    REFUSED = "refused",
    /** Another encryption is still running. */
    BUSY = "busy",
}
