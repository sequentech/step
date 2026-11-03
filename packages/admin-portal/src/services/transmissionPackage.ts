// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {IMiruCcsServer, IMiruDocument, IMiruSigningRequest} from "@/types/miru"

const HASH_GROUP = 8
const KEY_FINGERPRINT_CHARS = 16

/** Splits a hex digest into uppercase groups of eight characters, to read it out. */
export const groupHash = (hex: string): string =>
    (
        hex
            .replace(/\s+/g, "")
            .toUpperCase()
            .match(new RegExp(`.{1,${HASH_GROUP}}`, "g")) ?? []
    ).join(" ")

/** A destination without an address receives the package by hand. */
export const isManualReceipt = (server: Pick<IMiruCcsServer, "address">): boolean =>
    !(server.address ?? "").trim()

/** The destinations the package is sent to over the network. */
export const networkServers = <T extends Pick<IMiruCcsServer, "address">>(servers: T[]): T[] =>
    servers.filter((server) => !isManualReceipt(server))

/** Every destination receives the package by hand, so there is nothing to send. */
export const onlyManualReceipt = (servers: Array<Pick<IMiruCcsServer, "address">>): boolean =>
    servers.length > 0 && servers.every(isManualReceipt)

/** The DER bytes of a PEM block. */
export const pemToDer = (pem: string): Uint8Array<ArrayBuffer> => {
    const block = pem.match(/-----BEGIN ([^-]+)-----([\s\S]*?)-----END \1-----/)
    if (!block) {
        throw new Error("Not a PEM block")
    }
    const body = block[2].replace(/\s+/g, "")
    if (!body) {
        throw new Error("Empty PEM")
    }
    const binary = atob(body)
    return Uint8Array.from(binary, (char) => char.charCodeAt(0))
}

const toHex = (bytes: ArrayBuffer): string =>
    Array.from(new Uint8Array(bytes), (byte) => byte.toString(16).padStart(2, "0"))
        .join("")
        .toUpperCase()

/** Uppercase hex SHA-256 of the DER SubjectPublicKeyInfo in a public key PEM. */
export const publicKeySha256 = async (
    pem: string,
    subtle: SubtleCrypto = globalThis.crypto.subtle
): Promise<string> => {
    const der = pemToDer(pem)
    return toHex(await subtle.digest("SHA-256", der))
}

/** The start of a key's SHA-256, as the page shows it. */
export const shortKeyFingerprint = (sha256: string): string =>
    sha256.slice(0, KEY_FINGERPRINT_CHARS).toUpperCase()

/** The signatures a package needs: its signing request's, else the area's threshold. */
export const signaturesNeeded = (
    signingRequest: Pick<IMiruSigningRequest, "required"> | null | undefined,
    threshold: number | null | undefined
): number => signingRequest?.required ?? threshold ?? 0

export enum ESignaturesState {
    BELOW_NEEDED = "below-needed",
    REACHED = "reached",
}

/** Below the signatures needed until they are all in; no minimum means reached. */
export const signaturesState = (signed: number, needed: number): ESignaturesState =>
    needed <= 0 || signed >= needed ? ESignaturesState.REACHED : ESignaturesState.BELOW_NEEDED

/** The newest of a package's generations. */
export const latestDocument = (
    documents: IMiruDocument[] | null | undefined
): IMiruDocument | null => {
    let newestDate = new Date(0)
    let newest: IMiruDocument | null = null
    for (const document of documents ?? []) {
        const date = document.created_at ? new Date(document.created_at) : null
        if (date && date > newestDate) {
            newestDate = date
            newest = document
        }
    }
    return newest
}

export enum ETransmissionFile {
    EML = "eml",
    CSV = "csv",
    PACKAGE = "package",
    OVCS_PUBLIC_KEY = "ovcs-public-key",
}

const FILE_SUFFIX: Record<ETransmissionFile, string> = {
    [ETransmissionFile.EML]: ".eml",
    [ETransmissionFile.CSV]: ".csv",
    [ETransmissionFile.PACKAGE]: ".zip",
    [ETransmissionFile.OVCS_PUBLIC_KEY]: "-ovcs-public-key.pem",
}

/** The name a downloaded file of the package is saved with. */
export const transmissionFileName = (
    baseName: string,
    date: string,
    file: ETransmissionFile
): string => `${baseName}${date}${FILE_SUFFIX[file]}`
