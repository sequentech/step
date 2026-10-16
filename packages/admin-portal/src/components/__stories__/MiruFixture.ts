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

/** A synthetic P-256 destination key; its DER SubjectPublicKeyInfo hashes to MANUAL_KEY_SHA256. */
export const MANUAL_KEY_PEM = [
    "-----BEGIN PUBLIC KEY-----",
    "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEMP8nkUSnx3UqYsrG0S7ESrQDj3Te",
    "Xq2LhhZnUwg2l6GrOl5aOwY/FYw7Lqfnjkdhx4OMX39f48eSH7JA7tFtEQ==",
    "-----END PUBLIC KEY-----",
    "",
].join("\n")
export const MANUAL_KEY_SHA256 = "66BA07DD9888E2E7DECB84E8DBA009918FEA3247E8C2FAF77DB0532AAD4353E2"

/** A destination without an address: the package is downloaded and handed over. */
export const MANUAL_SERVER: IMiruCcsServer = {
    name: "CCS Manila (SBOC)",
    tag: "sboc",
    address: "",
    public_key_pem: MANUAL_KEY_PEM,
}

/** A synthetic OVCS public key the package's signature verifies with. */
export const OVCS_PUBLIC_KEY = [
    "-----BEGIN PUBLIC KEY-----",
    "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEIncvQYyXXWk+rde84ASofRlw47zR",
    "qqA3pKbPu6zS7PfY92n5ZDaXFWC5/oGqKsCQVvgHgy8KkoeH3fr9awfDIg==",
    "-----END PUBLIC KEY-----",
    "",
].join("\n")

/** The SHA-256 of the current generation's ER file, as the package stores it. */
export const ER_FILE_SHA256 = "E8D67AC41B2C3D4E5F60718293A4B5C6D7E8F9011223344556677889073CD925"

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
    csv: documentId(57),
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
            csv: MIRU_DOCUMENT_IDS.csv,
        },
        sha256: {
            eml: ER_FILE_SHA256,
            csv: "0A82EABE".repeat(8),
            all_servers: "4884BAC3".repeat(8),
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
