// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The fixtures come from scripts/signing/make-test-p12.sh. fixtures.json holds
// what the openssl CLI reported for each leaf, so the expectations below don't
// come from the code under test.

import {X509Certificate, verify as nodeVerify, webcrypto} from "node:crypto"
import {readFileSync} from "node:fs"
import {join} from "node:path"
import {
    CertificateFileError,
    CertificateFileErrorCode,
    openFailureReason,
    openP12,
    openP12Choices,
    rankCertificates,
    signPayload,
    type OpenedCertificate,
} from "./certificate"
import {multiKeyP12} from "./__stories__/multiKeyP12"
import {ecdsaRawToDer, fromBase64, toBase64, colonHex} from "./der"
import {CertificateOpenFailure, SignatureAlgorithm} from "./types"

if (!globalThis.crypto?.subtle) {
    Object.defineProperty(globalThis, "crypto", {value: webcrypto})
}

interface FixtureInfo {
    password: string
    algorithm: SignatureAlgorithm
    commonName: string
    issuerCommonName: string
    notAfter: string
    signingKeyUsage: boolean
    chainLength: number
    fingerprintSha256: string
    spkiSha256: string
}

const FX = join(__dirname, "__fixtures__")
const manifest = JSON.parse(readFileSync(join(FX, "fixtures.json"), "utf8")) as {
    files: Record<string, FixtureInfo>
    revokedSerials: string[]
}
const PASSWORD = manifest.files["maria-santos-rsa-aes.p12"].password
const fixture = (name: string): Uint8Array<ArrayBuffer> =>
    new Uint8Array(readFileSync(join(FX, name)))
const pemBlocks = (file: string): string[] =>
    (readFileSync(join(FX, file), "utf8").match(
        /-----BEGIN CERTIFICATE-----[\s\S]+?-----END CERTIFICATE-----\n/g
    ) ?? []) as string[]
const PNPKI_CHAIN = pemBlocks("test-pnpki-chain.pem")
const COMMERCIAL_CHAIN = pemBlocks("example-commercial-chain.pem")

// "Señal" -> "Senal": drops the combining tilde left by NFD.
const asciiOnly = (text: string) =>
    Array.from(text.normalize("NFD"))
        .filter((c) => c.charCodeAt(0) < 0x80)
        .join("")

const rejectsWith = async (promise: Promise<unknown>, code: CertificateFileErrorCode) => {
    const error = await promise.then(
        () => null,
        (e: unknown) => e
    )
    expect(error).toBeInstanceOf(CertificateFileError)
    expect((error as CertificateFileError).code).toBe(code)
}

// Canonical payload shape (design §2): sorted keys, no whitespace. The server sends these bytes.
const PAYLOAD = new TextEncoder().encode(
    '{"action":"close-voting","area_id":null,"code":"7K2Q-M9XD","domain":"step-signing/v1","subject":{"channels":["ONLINE"]}}'
)

describe("openP12", () => {
    it.each(Object.entries(manifest.files))("opens %s", async (name, info) => {
        const opened = await openP12(fixture(name), info.password)

        expect(opened.algorithm).toBe(info.algorithm)
        expect(opened.commonName).toBe(info.commonName)
        expect(opened.issuerCommonName).toBe(info.issuerCommonName)
        expect(opened.notAfter.toISOString()).toBe(new Date(info.notAfter).toISOString())
        expect(opened.fingerprintSha256).toBe(info.fingerprintSha256)
        expect(opened.spkiSha256).toBe(info.spkiSha256)
        const signing = !!opened.keyUsage?.digitalSignature || !!opened.keyUsage?.nonRepudiation
        expect(signing).toBe(info.signingKeyUsage)
        // Leaf first, then the issuers in chain order, as in the chain file.
        expect(opened.chainPem).toHaveLength(info.chainLength)
        const issuers =
            info.chainLength === 1
                ? []
                : info.issuerCommonName.startsWith("Example")
                  ? COMMERCIAL_CHAIN
                  : PNPKI_CHAIN
        // Every certificate keeps the file's DER: the chain file's PEM, byte for byte.
        expect(opened.chainPem.slice(1)).toEqual(issuers)
        expect(new X509Certificate(opened.chainPem[0]).fingerprint256).toBe(
            colonHex(info.fingerprintSha256)
        )
        expect(opened.chainDer[0]).toEqual(
            new Uint8Array(new X509Certificate(opened.chainPem[0]).raw)
        )

        // The private key can sign and nothing else, and it can't be exported.
        expect(opened.privateKey.extractable).toBe(false)
        expect(opened.privateKey.usages).toEqual(["sign"])
        const exported = await globalThis.crypto.subtle.exportKey("pkcs8", opened.privateKey).then(
            () => null,
            (e: Error) => e
        )
        // Browsers: DOMException InvalidAccessError. Node: InvalidAccessException.
        expect(exported?.name).toMatch(/^InvalidAccess(Error|Exception)$/)
    })

    it("formats the subject and issuer in the certificate's order", async () => {
        const opened = await openP12(fixture("maria-santos-rsa-aes.p12"), PASSWORD)
        expect(opened.subject).toBe(
            "C=PH, O=Test PNPKI, OU=Individual, CN=MARIA L. SANTOS, serialNumber=PH-0001"
        )
        expect(opened.issuer).toBe("C=PH, O=Test PNPKI, CN=Test PNPKI Individual CA")
    })

    it("keeps the SPKI of a reissued certificate and tells apart two keys of one holder", async () => {
        const first = await openP12(fixture("maria-santos-rsa-aes.p12"), PASSWORD)
        const reissued = await openP12(fixture("maria-santos-rsa-reissue.p12"), PASSWORD)
        expect(reissued.spkiSha256).toBe(first.spkiSha256)
        expect(reissued.fingerprintSha256).not.toBe(first.fingerprintSha256)

        const a = await openP12(fixture("juan-delacruz-a.p12"), PASSWORD)
        const b = await openP12(fixture("juan-delacruz-b.p12"), PASSWORD)
        expect(a.subject).toBe(b.subject)
        expect(a.spkiSha256).not.toBe(b.spkiSha256)
    })

    it("reads the serial number of the revoked fixture", async () => {
        const opened = await openP12(fixture("carmen-villanueva-revoked.p12"), PASSWORD)
        expect(manifest.revokedSerials).toContain(opened.serialNumber)
    })

    const nonAscii = Object.entries(manifest.files).filter(
        ([, info]) => info.password !== asciiOnly(info.password)
    )

    // Covers PBES2 and PKCS#12 PBE bags in one file (both ways round), 2-, 3- and
    // 4-byte UTF-8 characters, and a file whose Latin-1 derivation passes the
    // padding check (pinned): the UTF-8 derivation is the only one ever tried.
    it.each(nonAscii)(
        "opens %s with its non-ASCII password and refuses its ASCII look-alike",
        async (name, info) => {
            await expect(openP12(fixture(name), info.password)).resolves.toMatchObject({
                commonName: info.commonName,
                fingerprintSha256: info.fingerprintSha256,
            })
            await rejectsWith(
                openP12(fixture(name), asciiOnly(info.password)),
                CertificateFileErrorCode.WrongPassword
            )
        }
    )

    it("covers every mixed and multi-byte case", () => {
        const names = nonAscii.map(([name]) => name)
        for (const expected of [
            "maria-santos-rsa-mixrc2-utf8.p12",
            "maria-santos-rsa-mix3des-utf8.p12",
            "maria-santos-rsa-mix3deskey-utf8.p12",
            "jose-reyes-ec-aes-cyrillic.p12",
            "ana-cruz-rsa-aes-cjk.p12",
            "pinned-latin1-unpad-utf8.p12",
        ]) {
            expect(names).toContain(expected)
        }
    })

    it.each([
        "maria-santos-rsa-aes.p12",
        "maria-santos-rsa-3des.p12",
        "maria-santos-rsa-rc2.p12",
        "jose-reyes-ec-aes.p12",
    ])("reports a wrong password for %s", async (name) => {
        await rejectsWith(
            openP12(fixture(name), "demo-2028"),
            CertificateFileErrorCode.WrongPassword
        )
        await rejectsWith(openP12(fixture(name), ""), CertificateFileErrorCode.WrongPassword)
    })

    it("reports files that aren't PKCS#12 as unreadable, not as a wrong password", async () => {
        const good = fixture("maria-santos-rsa-aes.p12")
        const pemText = new TextEncoder().encode(PNPKI_CHAIN[0])
        const derCertificate = new Uint8Array(new X509Certificate(PNPKI_CHAIN[0]).raw)
        const cases: Uint8Array[] = [
            pemText,
            derCertificate,
            good.subarray(0, Math.floor(good.length / 2)),
            good.subarray(0, good.length - 1),
            globalThis.crypto.getRandomValues(new Uint8Array(2048)),
            new Uint8Array(),
        ]
        for (const bytes of cases) {
            await rejectsWith(openP12(bytes, PASSWORD), CertificateFileErrorCode.UnreadableFile)
        }
    })

    it("reports a file with certificates but no key", async () => {
        await rejectsWith(
            openP12(fixture("no-key.p12"), PASSWORD),
            CertificateFileErrorCode.NoPrivateKey
        )
    })
})

describe("files with several keys", () => {
    const part = (name: string, friendlyName: string) => ({
        bytes: fixture(name),
        password: manifest.files[name].password,
        friendlyName,
    })

    it("pairs each key with its own certificate and prefers the valid one", async () => {
        const file = multiKeyP12(
            [part("ramon-garcia-expired.p12", "old"), part("maria-santos-rsa-3des.p12", "current")],
            PASSWORD
        )
        const {choices, ambiguous} = await openP12Choices(file, PASSWORD)
        expect(ambiguous).toBe(false)
        expect(choices.map((c) => c.fingerprintSha256)).toEqual([
            manifest.files["maria-santos-rsa-3des.p12"].fingerprintSha256,
            manifest.files["ramon-garcia-expired.p12"].fingerprintSha256,
        ])
        expect(choices.map((c) => c.friendlyName)).toEqual(["current", "old"])
        // Each key signs for its own certificate.
        for (const choice of choices) {
            const signature = await signPayload(choice.privateKey, choice.algorithm, PAYLOAD)
            const publicKey = new X509Certificate(choice.chainPem[0]).publicKey
            expect(nodeVerify("sha256", PAYLOAD, publicKey, signature)).toBe(true)
        }
    })

    it("prefers a certificate made for signing", async () => {
        const file = multiKeyP12(
            [
                part("pedro-bautista-noku.p12", "encryption"),
                part("ana-cruz-rsa-aes.p12", "signing"),
            ],
            PASSWORD
        )
        const {choices, ambiguous} = await openP12Choices(file, PASSWORD)
        expect(ambiguous).toBe(false)
        expect(choices[0].commonName).toBe(manifest.files["ana-cruz-rsa-aes.p12"].commonName)
    })

    it("leaves two equally good certificates to the person", async () => {
        const file = multiKeyP12(
            [part("maria-santos-rsa-3des.p12", "a"), part("ana-cruz-rsa-aes.p12", "b")],
            PASSWORD
        )
        const {choices, ambiguous} = await openP12Choices(file, PASSWORD)
        expect(ambiguous).toBe(true)
        expect(choices).toHaveLength(2)
    })

    it("ranks valid, then signing, then the latest expiry", () => {
        const at = (from: string, to: string, signing = true) =>
            ({
                notBefore: new Date(from),
                notAfter: new Date(to),
                keyUsage: {digitalSignature: signing, nonRepudiation: false},
            }) as OpenedCertificate
        const now = new Date("2028-01-01")
        const expired = at("2020-01-01", "2025-01-01")
        const notSigning = at("2027-01-01", "2031-01-01", false)
        const early = at("2027-01-01", "2029-01-01")
        const late = at("2027-01-01", "2030-01-01")
        expect(rankCertificates([expired, notSigning, early, late], now).choices).toEqual([
            late,
            early,
            notSigning,
            expired,
        ])
    })
})

describe("openFailureReason", () => {
    it.each([
        [CertificateFileErrorCode.WrongPassword, CertificateOpenFailure.WrongPassword],
        [CertificateFileErrorCode.NoPrivateKey, CertificateOpenFailure.NoKey],
        [CertificateFileErrorCode.UnreadableFile, CertificateOpenFailure.Unreadable],
        [CertificateFileErrorCode.UnsupportedEncryption, CertificateOpenFailure.Unreadable],
        [CertificateFileErrorCode.UnsupportedKey, CertificateOpenFailure.Unreadable],
        [CertificateFileErrorCode.NoCertificate, CertificateOpenFailure.Unreadable],
        [CertificateFileErrorCode.KeyCertificateMismatch, CertificateOpenFailure.Unreadable],
    ])("maps %s to %s", (code, reason) => {
        expect(openFailureReason(code)).toBe(reason)
    })
})

describe("signPayload", () => {
    it.each([
        ["maria-santos-rsa-rc2.p12", SignatureAlgorithm.RsaPkcs1Sha256],
        ["jose-reyes-ec-aes.p12", SignatureAlgorithm.EcdsaP256Sha256],
    ])("signs with %s so that OpenSSL verifies the signature", async (name, algorithm) => {
        const opened = await openP12(fixture(name), PASSWORD)
        expect(opened.algorithm).toBe(algorithm)
        const publicKey = new X509Certificate(opened.chainPem[0]).publicKey

        const signature = await signPayload(opened.privateKey, opened.algorithm, PAYLOAD)

        // node:crypto uses OpenSSL: PKCS#1 v1.5 for RSA, DER ECDSA-Sig-Value for EC.
        expect(nodeVerify("sha256", PAYLOAD, publicKey, signature)).toBe(true)
        const tampered = new Uint8Array(PAYLOAD.length + 1)
        tampered.set(PAYLOAD)
        tampered[PAYLOAD.length] = 0x20
        expect(nodeVerify("sha256", tampered, publicKey, signature)).toBe(false)
        if (algorithm === SignatureAlgorithm.EcdsaP256Sha256) {
            expect(signature[0]).toBe(0x30)
        } else {
            expect(signature).toHaveLength(256)
        }
    })

    it("produces DER that OpenSSL accepts for many ECDSA signatures", async () => {
        // About 1 in 256 signatures has an r or s with a leading zero byte or its high bit set.
        const opened = await openP12(fixture("jose-reyes-ec-aes.p12"), PASSWORD)
        const publicKey = new X509Certificate(opened.chainPem[0]).publicKey
        for (let i = 0; i < 200; i++) {
            const message = new TextEncoder().encode(`message ${i}`)
            const signature = await signPayload(opened.privateKey, opened.algorithm, message)
            expect(nodeVerify("sha256", message, publicKey, signature)).toBe(true)
        }
    })
})

describe("ecdsaRawToDer", () => {
    it("strips leading zeros and pads a set high bit", () => {
        const raw = new Uint8Array(64)
        raw[31] = 0x01 // r = 1
        raw[32] = 0x80 // s = 0x80 00 … 00 (32 bytes, high bit set)
        const s = [0x00, 0x80, ...new Array(31).fill(0)]
        expect(Array.from(ecdsaRawToDer(raw))).toEqual([
            0x30,
            3 + 2 + s.length,
            0x02,
            0x01,
            0x01,
            0x02,
            s.length,
            ...s,
        ])
    })

    it("refuses an odd or empty signature", () => {
        expect(() => ecdsaRawToDer(new Uint8Array(63))).toThrow()
        expect(() => ecdsaRawToDer(new Uint8Array())).toThrow()
    })
})

describe("base64", () => {
    it("round-trips every byte value", () => {
        const all = Uint8Array.from({length: 256}, (_, i) => i)
        expect(toBase64(all)).toBe(Buffer.from(all).toString("base64"))
        expect(fromBase64(Buffer.from(all).toString("base64"))).toEqual(all)
    })
})
