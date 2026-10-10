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
}

// MiruDocument interface
export interface IMiruDocument {
    document_ids: IMiruDocumentIds
    servers_sent_to: Array<IMiruServersSentTo>
    transaction_id: string
    created_at: string
    signatures: Array<IMiruSignature>
}

// MiruCcsServer interface
export interface IMiruCcsServer {
    name: string
    tag: string
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
