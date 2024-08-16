// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ILog} from "./ceremonies"

// MiruSignature interface
export interface IMiruSignature {
    trustee_name: string
    pub_key: string
    signature: string
}

export interface IMiruServersSentTo {
    name: string
    document_id: string
}

// MiruDocument interface
export interface IMiruDocument {
    document_id: string
    servers_sent_to: Array<IMiruServersSentTo>
    transaction_id: string
    created_at: string
    signatures: Array<IMiruSignature>
}

// MiruCcsServer interface
export interface IMiruCcsServer {
    name: string
    address: string
    publick_key_pem: string
}

// MiruTransmissionPackageData interface
export interface IMiruTransmissionPackageData {
    election_id: string
    area_id: string
    servers: Array<IMiruCcsServer>
    documents: Array<IMiruDocument>
    logs: Array<ILog>
}

// MiruTallySessionData type alias
export type IMiruTallySessionData = Array<IMiruTransmissionPackageData>

export const MIRU_TALLY_SESSION_ANNOTATION_KEY = "miru:tally-session-data"
