// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {X509Certificate, webcrypto} from "node:crypto"
import {readFileSync} from "node:fs"
import {join} from "node:path"
import {execFileSync} from "node:child_process"
import {parseCertificateAuthorities, signingConfiguration} from "./configurationSigning"

Object.defineProperty(globalThis, "crypto", {value: webcrypto, configurable: true})
const bundle = readFileSync(
    join(__dirname, "../../lib/signing/__fixtures__/test-pnpki-chain.pem"),
    "utf8"
)
const certificates = bundle.match(/-----BEGIN CERTIFICATE-----[\s\S]*?-----END CERTIFICATE-----/g)!
const authorities = certificates.filter((pem) => new X509Certificate(pem).ca).join("\n")
const now = new Date("2026-10-26T00:00:00Z")

test("shows each CA's real DER fingerprint and rejects private keys and leaf certificates", async () => {
    const result = await parseCertificateAuthorities(authorities, now)
    expect(result).toHaveLength(2)
    expect(result[0].fingerprint).toBe(new X509Certificate(result[0].pem).fingerprint256)
    const leaf = execFileSync(
        "openssl",
        [
            "pkcs12",
            "-in",
            join(__dirname, "../../lib/signing/__fixtures__/maria-santos-rsa-aes.p12"),
            "-passin",
            "pass:Demo-2028",
            "-clcerts",
            "-nokeys",
        ],
        {encoding: "utf8", timeout: 5000}
    ).match(/-----BEGIN CERTIFICATE-----[\s\S]*?-----END CERTIFICATE-----/)![0]
    await expect(parseCertificateAuthorities(leaf, now)).rejects.toThrow("CA certificate")
    await expect(
        parseCertificateAuthorities(
            authorities + "\n-----BEGIN PRIVATE KEY-----\nAA==\n-----END PRIVATE KEY-----",
            now
        )
    ).rejects.toThrow("Only public")
})

test("rejects corrupt, expired, duplicate and oversized uploads", async () => {
    await expect(
        parseCertificateAuthorities(
            "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----",
            now
        )
    ).rejects.toThrow()
    await expect(parseCertificateAuthorities(authorities, new Date("2100-01-01"))).rejects.toThrow(
        "valid"
    )
    await expect(
        parseCertificateAuthorities(authorities + "\n" + authorities, now)
    ).rejects.toThrow("duplicate")
    await expect(parseCertificateAuthorities("x".repeat(262145), now)).rejects.toThrow("256")
})

test("requires at least two approvals and both independently confirmed CA sets", () => {
    expect(signingConfiguration(authorities, authorities, 2)).toEqual({
        policy: "required",
        package_roots: authorities,
        staff_roots: authorities,
        required_approvals: 2,
    })
    for (const count of [0, 1, 2.5, 65536, NaN])
        expect(() => signingConfiguration(authorities, authorities, count)).toThrow("approvals")
    expect(() => signingConfiguration("", authorities, 2)).toThrow("certificates")
})
