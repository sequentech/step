// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {X509Certificate, webcrypto} from "node:crypto"
import {readFileSync} from "node:fs"
import {join} from "node:path"
import {localOfficerSettings, readPublicLeaf} from "./localInitializeOfficerApi"

if (!globalThis.crypto?.subtle) {
    Object.defineProperty(globalThis, "crypto", {value: webcrypto})
}
const chain = readFileSync(join(__dirname, "__fixtures__", "example-commercial-chain.pem"), "utf8")
const pem = chain.match(/-----BEGIN CERTIFICATE-----[\s\S]+?-----END CERTIFICATE-----\n/)![0]
const received = (source: string | Uint8Array) => {
    const bytes = typeof source === "string" ? new TextEncoder().encode(source) : new Uint8Array(source)
    return {size: bytes.length, arrayBuffer: async () => bytes.buffer}
}

test("local officer widgets stay off by default, even if strings or inactive profile data are present", () => {
    const registration = {KEYCLOAK_URL: "http://localhost:18090/", FELIX3_LOCAL_INITIALIZE: {}}
    const origin = "http://localhost:3002"
    const tenant = "cb25f973-2738-411e-a3a3-04a96c97bb29"
    expect(localOfficerSettings(origin, tenant, registration)).toBeUndefined()
    for (const value of [false, "true", true]) {
        expect(localOfficerSettings(origin, tenant, {...registration, FELIX3_LOCAL_OFFICER_BRIDGE: value}))
            .toBeUndefined()
    }
})

test("review fingerprints the exact single public certificate in PEM or DER", async () => {
    const certificate = new X509Certificate(pem)
    const expected = {
        leaf_der: certificate.raw.toString("base64"),
        leaf_sha256: certificate.fingerprint256.toLowerCase().replace(/:/g, ""),
    }
    expect(await readPublicLeaf(received(pem))).toEqual(expected)
    expect(await readPublicLeaf(received(pem.replace(/\n/g, "\r\n")))).toEqual(expected)
    expect(await readPublicLeaf(received(certificate.raw))).toEqual(expected)
})

test("does not send chains, private keys, trailing DER, BER alias lengths or oversized files", async () => {
    const certificate = new X509Certificate(pem)
    const invalid = [
        chain,
        "-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n",
        Buffer.concat([certificate.raw, Buffer.of(0)]),
        Uint8Array.of(0x30, 0x80, 0, 0),
        Uint8Array.of(0x30, 0x81, 0x01, 0),
        new Uint8Array(46081),
    ]
    for (const input of invalid) await expect(readPublicLeaf(received(input))).rejects.toThrow()
})
