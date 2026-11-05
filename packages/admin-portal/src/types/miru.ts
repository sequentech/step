// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ILog} from "./ceremonies"

// MiruSignature interface
export interface IMiruSignature {
    sbei_miru_id: string
    pub_key: string
    signature: string
}

export interface IMiruServersSentTo {
    name: string
    sent_at: string
    status: string
}

export interface IMiruDocumentIds {
    eml: string
    xz: string
    all_servers: string
    /** The results CSV, generated with the EML. */
    csv?: string | null
}

/** Uppercase hex SHA-256 of each stored file; the EML's is the one the package carries. */
export interface IMiruDocumentHashes {
    eml: string
    csv?: string | null
    all_servers: string
}

// MiruDocument interface
export interface IMiruDocument {
    document_ids: IMiruDocumentIds
    servers_sent_to: Array<IMiruServersSentTo>
    transaction_id: string
    created_at: string
    signatures: Array<IMiruSignature>
    sha256?: IMiruDocumentHashes | null
}

// MiruCcsServer interface
export interface IMiruCcsServer {
    name: string
    tag: string
    /** Empty for a destination that receives the package by hand. */
    address: string
    public_key_pem: string
    send_logs?: boolean
}

// MiruTransmissionPackageData interface
export interface IMiruTransmissionPackageData {
    election_id: string
    area_id: string
    servers: Array<IMiruCcsServer>
    documents: Array<IMiruDocument>
    logs: Array<ILog>
    threshold: number
    /** The signing request the package waits for, when the transmit-results rule needs signatures. */
    signing_request?: IMiruSigningRequest | null
    /** The OVCS public key PEM the package's signature verifies with. */
    ovcs_public_key?: string | null
}

/** A transmission package's signing request. */
export interface IMiruSigningRequest {
    id: string
    code: string
    required: number
}

// MiruTallySessionData type alias
export type IMiruTallySessionData = Array<IMiruTransmissionPackageData>

export const MIRU_TALLY_SESSION_ANNOTATION_KEY = "miru:tally-session-data"
