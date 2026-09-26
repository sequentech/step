// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A Miru transmission package of the tally: the destination servers, the SBEI
// members who sign it and the documents each generation produced.
import {
    MIRU_TALLY_SESSION_ANNOTATION_KEY,
    type IMiruCcsServer,
    type IMiruDocument,
    type IMiruTransmissionPackageData,
} from "@/types/miru"
import {STORY_IDS} from "@/__stories__/fixtures"
import {documentId} from "@/resources/Tally/__stories__/TallyFixture"

export const MIRU_SERVERS: IMiruCcsServer[] = ["ccs-north", "ccs-south", "ccs-east"].map(
    (name, index) => ({
        name,
        tag: `tag-${index + 1}`,
        address: `https://${name}.admin-story.invalid`,
        public_key_pem: `synthetic-pem-${index + 1}`,
        send_logs: true,
    })
)

/** SBEI members of the north district; the signed-in "admin" user is sbei-1. */
export const SBEI_IDS = ["sbei-1", "sbei-2", "sbei-3"]

/** Annotations an event and an area store as JSON strings. */
export const SBEI_USERS_ANNOTATION = {
    "miru:sbei-users": JSON.stringify([
        {username: "admin", miru_id: "sbei-1"},
        {username: "trustee2", miru_id: "sbei-2"},
    ]),
}
export const AREA_TRUSTEES_ANNOTATION = {"miru:area-trustee-users": JSON.stringify(SBEI_IDS)}

export const MIRU_DOCUMENT_IDS = {
    eml: documentId(51),
    xz: documentId(52),
    allServers: documentId(53),
    previousAllServers: documentId(54),
}

/** The package documents: an older generation and the current one, signed by `signed`. */
export function miruDocuments(signed: string[] = ["sbei-1"], sentTo: string[] = []) {
    const previous: IMiruDocument = {
        document_ids: {
            eml: documentId(55),
            xz: documentId(56),
            all_servers: MIRU_DOCUMENT_IDS.previousAllServers,
        },
        servers_sent_to: [],
        transaction_id: "tx-0001",
        created_at: "2026-01-15T09:00:00Z",
        signatures: [],
    }
    const current: IMiruDocument = {
        document_ids: {
            eml: MIRU_DOCUMENT_IDS.eml,
            xz: MIRU_DOCUMENT_IDS.xz,
            all_servers: MIRU_DOCUMENT_IDS.allServers,
        },
        servers_sent_to: sentTo.map((name) => ({
            name,
            sent_at: "2026-01-15T13:00:00Z",
            status: "SUCCESS",
        })),
        transaction_id: "tx-0002",
        created_at: "2026-01-15T12:45:00Z",
        signatures: signed.map((id) => ({
            sbei_miru_id: id,
            pub_key: `synthetic-key-${id}`,
            signature: `synthetic-signature-${id}`,
        })),
    }
    return [previous, current]
}

export function miruPackage(
    overrides: Partial<IMiruTransmissionPackageData> = {}
): IMiruTransmissionPackageData {
    return {
        election_id: STORY_IDS.election,
        area_id: STORY_IDS.area,
        servers: MIRU_SERVERS,
        documents: miruDocuments(),
        logs: [{created_date: "2026-01-15T12:45:00Z", log_text: "Transmission package created"}],
        threshold: 2,
        ...overrides,
    }
}

/** The tally session annotation that lists its transmission packages. */
export const miruTallyAnnotations = (packages: IMiruTransmissionPackageData[]) => ({
    [MIRU_TALLY_SESSION_ANNOTATION_KEY]: JSON.stringify(packages),
})
