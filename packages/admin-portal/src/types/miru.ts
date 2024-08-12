// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ILog} from "./ceremonies"

// MiruSignature interface
export interface IMiruSignature {
    trusteeName: string
    pubKey: string
    signature: string
}

// MiruDocument interface
export interface IMiruDocument {
    documentId: string
    serversSentTo: Array<string>
    createdAt: string
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
    electionId: string
    areaId: string
    servers: Array<IMiruCcsServer>
    documents: Array<IMiruDocument>
    logs: Array<ILog>
}

// MiruTallySessionData type alias
export type IMiruTallySessionData = Array<IMiruTransmissionPackageData>

export const MIRU_TALLY_SESSION_ANNOTATION_KEY = "miru:tally-session-data"
