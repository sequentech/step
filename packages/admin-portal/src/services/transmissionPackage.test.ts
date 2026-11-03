// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createHash, generateKeyPairSync} from "node:crypto"
import type {IMiruDocument} from "@/types/miru"
import {
    ESignaturesState,
    ETransmissionFile,
    groupHash,
    isManualReceipt,
    latestDocument,
    networkServers,
    onlyManualReceipt,
    pemToDer,
    publicKeySha256,
    shortKeyFingerprint,
    signaturesNeeded,
    signaturesState,
    transmissionFileName,
} from "./transmissionPackage"

const EML_SHA256 = [
    "e8d67ac4",
    "1b2c3d4e",
    "5f607182",
    "93a4b5c6",
    "d7e8f901",
    "12233445",
    "56677889",
    "73cd9254",
].join("")

describe("groupHash", () => {
    it("splits a digest into uppercase groups of eight", () => {
        expect(groupHash(EML_SHA256)).toBe(
            "E8D67AC4 1B2C3D4E 5F607182 93A4B5C6 D7E8F901 12233445 56677889 73CD9254"
        )
    })

    it("keeps a short last group and ignores whitespace", () => {
        expect(groupHash(" abcdef0123 45 ")).toBe("ABCDEF01 2345")
    })

    it("returns nothing for an empty digest", () => {
        expect(groupHash("")).toBe("")
    })
})

describe("manual receipt", () => {
    const manual = {address: ""}
    const blank = {address: "   "}
    const network = {address: "https://ccs.example.test"}

    it.each([
        [manual, true],
        [blank, true],
        [network, false],
    ])("detects a destination without an address (%j)", (server, expected) => {
        expect(isManualReceipt(server)).toBe(expected)
    })

    it("treats a missing address as manual receipt", () => {
        expect(isManualReceipt({} as {address: string})).toBe(true)
    })

    it("is manual receipt only when every destination is", () => {
        expect(onlyManualReceipt([manual, blank])).toBe(true)
        expect(onlyManualReceipt([manual, network])).toBe(false)
        expect(onlyManualReceipt([])).toBe(false)
    })

    it("keeps the network destinations of a mixed list", () => {
        expect(networkServers([manual, network, blank])).toEqual([network])
    })
})

describe("key fingerprint", () => {
    const {publicKey} = generateKeyPairSync("ec", {namedCurve: "P-256"})
    const pem = publicKey.export({type: "spki", format: "pem"}).toString()
    const der = publicKey.export({type: "spki", format: "der"})
    const expected = createHash("sha256").update(der).digest("hex").toUpperCase()

    it("decodes the PEM body to the DER SubjectPublicKeyInfo", () => {
        expect(Buffer.from(pemToDer(pem))).toEqual(der)
    })

    it("hashes the DER SubjectPublicKeyInfo", async () => {
        await expect(publicKeySha256(pem)).resolves.toBe(expected)
    })

    it("shows the first sixteen characters", async () => {
        const short = shortKeyFingerprint(await publicKeySha256(pem))
        expect(short).toBe(expected.slice(0, 16))
        expect(short).toMatch(/^[0-9A-F]{16}$/)
    })

    it("rejects an empty PEM", async () => {
        await expect(
            publicKeySha256("-----BEGIN PUBLIC KEY-----\n-----END PUBLIC KEY-----")
        ).rejects.toThrow("Empty PEM")
    })

    it("rejects text without a PEM block", async () => {
        await expect(publicKeySha256("not a key")).rejects.toThrow("Not a PEM block")
    })

    it("rejects a PEM body that is not base64", async () => {
        await expect(
            publicKeySha256("-----BEGIN PUBLIC KEY-----\n%%%%\n-----END PUBLIC KEY-----")
        ).rejects.toThrow()
    })
})

describe("signatures chip", () => {
    it("uses the signing request's signatures before the threshold", () => {
        expect(signaturesNeeded({required: 3}, 2)).toBe(3)
        expect(signaturesNeeded(null, 2)).toBe(2)
        expect(signaturesNeeded(undefined, undefined)).toBe(0)
    })

    it.each([
        [0, 2, ESignaturesState.BELOW_NEEDED],
        [1, 2, ESignaturesState.BELOW_NEEDED],
        [2, 2, ESignaturesState.REACHED],
        [3, 2, ESignaturesState.REACHED],
        [0, 0, ESignaturesState.REACHED],
        [0, -1, ESignaturesState.REACHED],
    ])("%i signed of %i needed is %s", (signed, needed, expected) => {
        expect(signaturesState(signed, needed)).toBe(expected)
    })
})

describe("latestDocument", () => {
    const document = (created_at: string, transaction_id: string): IMiruDocument => ({
        document_ids: {eml: "eml", xz: "xz", all_servers: "all"},
        servers_sent_to: [],
        transaction_id,
        created_at,
        signatures: [],
    })

    it("picks the newest generation", () => {
        expect(
            latestDocument([
                document("2026-01-15T12:45:00Z", "new"),
                document("2026-01-15T09:00:00Z", "old"),
            ])?.transaction_id
        ).toBe("new")
    })

    it("has none without documents", () => {
        expect(latestDocument([])).toBeNull()
        expect(latestDocument(null)).toBeNull()
    })
})

describe("transmissionFileName", () => {
    it.each([
        [ETransmissionFile.EML, "area__Spain15/01/2026 12:45.eml"],
        [ETransmissionFile.CSV, "area__Spain15/01/2026 12:45.csv"],
        [ETransmissionFile.PACKAGE, "area__Spain15/01/2026 12:45.zip"],
        [ETransmissionFile.OVCS_PUBLIC_KEY, "area__Spain15/01/2026 12:45-ovcs-public-key.pem"],
    ])("names the %s file", (file, expected) => {
        expect(transmissionFileName("area__Spain", "15/01/2026 12:45", file)).toBe(expected)
    })
})
