// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// <reference path="./node-forge.d.ts" />

// Opens a staff certificate file (.p12/.pfx) in the browser and signs with it.
//
// node-forge only decrypts the PKCS#12 container: it is the only JS library that
// opens PBES2/AES, 3DES and RC2-40 files. Certificates are parsed with pkijs
// (forge can't parse EC certificates), and the key becomes a non-extractable
// WebCrypto key; forge never signs. Neither the file nor the password leaves
// this module: callers get the key handle, the public chain and display data.
//
// Load this module with import() when the signing dialog opens (it is large).

import forge from "node-forge/lib/forge"
import "node-forge/lib/pkcs12"
import "node-forge/lib/pbe"
import "node-forge/lib/aes"
import "node-forge/lib/des"
import "node-forge/lib/rc2"
import "node-forge/lib/sha256"
import "node-forge/lib/sha512"
import * as asn1js from "asn1js"
import * as pkijs from "pkijs"
import {
    binaryStringToBytes,
    bytesToBinaryString,
    ecdsaRawToDer,
    hex,
    sha256,
    toArrayBuffer,
    toBytes,
    toPem,
} from "./der"
import {CertificateFileError, CertificateFileErrorCode} from "./errors"
import {SignatureAlgorithm} from "./types"

export {CertificateFileError, CertificateFileErrorCode, openFailureReason} from "./errors"

export interface KeyUsage {
    digitalSignature: boolean
    nonRepudiation: boolean
}

export interface OpenedCertificate {
    /** Non-extractable, usages ["sign"]. */
    privateKey: CryptoKey
    algorithm: SignatureAlgorithm
    /** Leaf first, then its issuers in order. */
    chainDer: Uint8Array<ArrayBuffer>[]
    chainPem: string[]
    /** "C=PH, O=…, CN=…", in the certificate's RDN order (display only). */
    subject: string
    issuer: string
    commonName: string | null
    issuerCommonName: string | null
    /** Lowercase hex. */
    serialNumber: string
    notBefore: Date
    notAfter: Date
    keyUsage: KeyUsage | null
    /** SHA-256 of the certificate DER, lowercase hex (design §7). */
    fingerprintSha256: string
    /** SHA-256 of the SubjectPublicKeyInfo DER, lowercase hex. */
    spkiSha256: string
    /** The name the file gives the key, if any. */
    friendlyName: string | null
}

/** Every key of a file with its certificate, best first (see `rankCertificates`). */
export interface OpenedCertificates {
    choices: OpenedCertificate[]
    /** The best two are equally good: the person chooses. */
    ambiguous: boolean
}

const OID = {
    rsaEncryption: "1.2.840.113549.1.1.1",
    ecPublicKey: "1.2.840.10045.2.1",
    p256: "1.2.840.10045.3.1.7",
    keyUsage: "2.5.29.15",
    commonName: "2.5.4.3",
}

const NAME_LABELS: Record<string, string> = {
    "2.5.4.6": "C",
    "2.5.4.8": "ST",
    "2.5.4.7": "L",
    "2.5.4.10": "O",
    "2.5.4.11": "OU",
    "2.5.4.3": "CN",
    "2.5.4.5": "serialNumber",
    "2.5.4.4": "SN",
    "2.5.4.42": "GN",
    "1.2.840.113549.1.9.1": "emailAddress",
}

const formatName = (name: pkijs.RelativeDistinguishedNames): string =>
    name.typesAndValues
        .map((tv) => `${NAME_LABELS[tv.type] ?? tv.type}=${tv.value.valueBlock.value}`)
        .join(", ")

const commonNameOf = (name: pkijs.RelativeDistinguishedNames): string | null => {
    const cn = name.typesAndValues.find((tv) => tv.type === OID.commonName)
    return cn ? String(cn.value.valueBlock.value) : null
}

const classifyForgeError = (e: unknown): CertificateFileError => {
    const msg = e instanceof Error ? e.message : String(e)
    const code =
        /MAC could not be verified|wrong password|Failed to decrypt PKCS#12 SafeContents/i.test(msg)
            ? CertificateFileErrorCode.WrongPassword
            : /unsupported|Unknown.*algorithm|not supported/i.test(msg)
              ? CertificateFileErrorCode.UnsupportedEncryption
              : CertificateFileErrorCode.UnreadableFile
    return new CertificateFileError(code, msg, {cause: e})
}

type ForgeBag = {
    key?: forge.pki.PrivateKey | null
    asn1?: forge.asn1.Asn1
    cert?: forge.pki.Certificate | null
    attributes: {localKeyId?: string[]; friendlyName?: string[]}
}

type ForgePbe = {
    getCipherForPBES2: (oid: string, params: unknown, password: string) => unknown
}

const UTF8_PBES2 = Symbol.for("sequent.signing.pbes2-utf8")

/**
 * PKCS#12 derives the MAC key and the PKCS#12 PBE keys (3DES, RC2) from the
 * password as a BMPString, which forge does right, but PBES2/PBKDF2 (AES)
 * from its UTF-8 bytes, where forge takes the low byte of each character.
 * Every PBES2 derivation gets the UTF-8 bytes instead (the same string for an
 * ASCII password), so a file mixing PBES2 and PKCS#12 PBE bags opens with one
 * password and a wrong derivation is never tried.
 */
const deriveUtf8ForPbes2 = () => {
    const pbe = (forge as unknown as {pbe: ForgePbe & {[UTF8_PBES2]?: boolean}}).pbe
    if (pbe[UTF8_PBES2]) return
    const original = pbe.getCipherForPBES2
    pbe.getCipherForPBES2 = (oid, params, password) =>
        original(oid, params, forge.util.encodeUtf8(password))
    pbe[UTF8_PBES2] = true
}
deriveUtf8ForPbes2()

/**
 * Decodes the bags. Certificates are kept as ASN.1 (forge's X.509 model would
 * re-encode them, and can't read EC certificates), so their DER is the file's.
 */
const decodePfx = (pfxAsn1: forge.asn1.Asn1, password: string) => {
    const pki = forge.pki as unknown as {certificateFromAsn1: unknown}
    const certificateFromAsn1 = pki.certificateFromAsn1
    pki.certificateFromAsn1 = () => {
        throw new Error("kept as ASN.1")
    }
    try {
        const p12 = forge.pkcs12.pkcs12FromAsn1(pfxAsn1, false, password)
        const bagsOf = (type: string) => (p12.getBags({bagType: type})[type] ?? []) as ForgeBag[]
        return {
            keys: [...bagsOf(forge.pki.oids.pkcs8ShroudedKeyBag), ...bagsOf(forge.pki.oids.keyBag)],
            certs: bagsOf(forge.pki.oids.certBag),
        }
    } finally {
        pki.certificateFromAsn1 = certificateFromAsn1
    }
}

const openPfx = (fileBytes: Uint8Array, password: string) => {
    let pfxAsn1: forge.asn1.Asn1
    try {
        // Strict: a truncated file must fail here. Without it forge accepts the
        // file, the MAC is cut, and it surfaces as a wrong password.
        // (@types/node-forge only declares the boolean form of the options.)
        const fromDer = forge.asn1.fromDer as unknown as (
            bytes: string,
            options: {strict: boolean; parseAllBytes: boolean}
        ) => forge.asn1.Asn1
        pfxAsn1 = fromDer(bytesToBinaryString(fileBytes), {strict: true, parseAllBytes: false})
    } catch (e) {
        throw new CertificateFileError(
            CertificateFileErrorCode.UnreadableFile,
            e instanceof Error ? e.message : String(e),
            {cause: e}
        )
    }
    try {
        return decodePfx(pfxAsn1, password)
    } catch (e) {
        throw classifyForgeError(e)
    }
}

const pkcs8FromBag = (bag: ForgeBag): Uint8Array<ArrayBuffer> => {
    // RSA: forge parsed it into a key object and dropped the ASN.1. EC and
    // anything else: forge couldn't parse it and kept the PrivateKeyInfo as asn1.
    const info = bag.key
        ? forge.pki.wrapRsaPrivateKey(
              forge.pki.privateKeyToAsn1(bag.key as forge.pki.rsa.PrivateKey)
          )
        : bag.asn1
    if (!info) {
        throw new CertificateFileError(CertificateFileErrorCode.NoPrivateKey, "empty key bag")
    }
    return binaryStringToBytes(forge.asn1.toDer(info).getBytes())
}

const detectAlgorithm = (pkcs8: Uint8Array): SignatureAlgorithm => {
    const info = pkijs.PrivateKeyInfo.fromBER(toArrayBuffer(pkcs8))
    const alg = info.privateKeyAlgorithm
    if (alg.algorithmId === OID.rsaEncryption) {
        return SignatureAlgorithm.RsaPkcs1Sha256
    }
    if (alg.algorithmId === OID.ecPublicKey) {
        const curve = (
            alg.algorithmParams as asn1js.ObjectIdentifier | undefined
        )?.valueBlock?.toString()
        if (curve === OID.p256) {
            return SignatureAlgorithm.EcdsaP256Sha256
        }
        throw new CertificateFileError(
            CertificateFileErrorCode.UnsupportedKey,
            `EC curve ${curve ?? "(implicit)"} is not supported; use P-256`
        )
    }
    throw new CertificateFileError(
        CertificateFileErrorCode.UnsupportedKey,
        `key algorithm ${alg.algorithmId} is not supported`
    )
}

type ImportParams = Parameters<SubtleCrypto["importKey"]>[2]
type SignParams = Parameters<SubtleCrypto["sign"]>[0]

const importParams = (a: SignatureAlgorithm): ImportParams =>
    a === SignatureAlgorithm.RsaPkcs1Sha256
        ? {name: "RSASSA-PKCS1-v1_5", hash: "SHA-256"}
        : {name: "ECDSA", namedCurve: "P-256"}

const signParams = (a: SignatureAlgorithm): SignParams =>
    a === SignatureAlgorithm.RsaPkcs1Sha256
        ? {name: "RSASSA-PKCS1-v1_5"}
        : {name: "ECDSA", hash: "SHA-256"}

/**
 * Signs `bytes` (the canonical payload the server sent, or the EML document)
 * and returns what OpenSSL verifies: PKCS#1 v1.5 bytes for RSA, a DER
 * ECDSA-Sig-Value for EC. SHA-256 in both cases.
 */
export const signPayload = async (
    key: CryptoKey,
    algorithm: SignatureAlgorithm,
    bytes: Uint8Array<ArrayBuffer>,
    subtle: SubtleCrypto = globalThis.crypto.subtle
): Promise<Uint8Array<ArrayBuffer>> => {
    const signature = new Uint8Array(await subtle.sign(signParams(algorithm), key, bytes))
    return algorithm === SignatureAlgorithm.EcdsaP256Sha256 ? ecdsaRawToDer(signature) : signature
}

const keyMatchesCertificate = async (
    key: CryptoKey,
    algorithm: SignatureAlgorithm,
    cert: pkijs.Certificate,
    subtle: SubtleCrypto
): Promise<boolean> => {
    try {
        const spki = cert.subjectPublicKeyInfo.toSchema().toBER()
        const pub = await subtle.importKey("spki", spki, importParams(algorithm), false, ["verify"])
        const probe = globalThis.crypto.getRandomValues(new Uint8Array(32))
        const sig = await subtle.sign(signParams(algorithm), key, probe)
        return await subtle.verify(signParams(algorithm), pub, sig, probe)
    } catch {
        // e.g. an EC certificate against an RSA key
        return false
    }
}

const readKeyUsage = (cert: pkijs.Certificate): KeyUsage | null => {
    const ext = cert.extensions?.find((e) => e.extnID === OID.keyUsage)
    if (!ext) {
        return null
    }
    const bits = new Uint8Array((ext.parsedValue as asn1js.BitString).valueBlock.valueHexView)
    return {
        digitalSignature: (bits[0] & 0x80) !== 0,
        nonRepudiation: (bits[0] & 0x40) !== 0,
    }
}

const sameBytes = (a: ArrayBuffer, b: ArrayBuffer) =>
    a.byteLength === b.byteLength && hex(a) === hex(b)

interface ParsedCertificate {
    der: Uint8Array<ArrayBuffer>
    cert: pkijs.Certificate
    localKeyId?: string
}

/** Leaf first, then each issuer found among the file's other certificates. */
const orderChain = (leaf: ParsedCertificate, all: ParsedCertificate[]): ParsedCertificate[] => {
    const chain = [leaf]
    const pool = all.filter((c) => c !== leaf)
    for (;;) {
        const current = chain[chain.length - 1].cert
        const issuer = current.issuer.toSchema().toBER()
        if (sameBytes(issuer, current.subject.toSchema().toBER())) {
            break
        }
        const i = pool.findIndex((c) => sameBytes(c.cert.subject.toSchema().toBER(), issuer))
        if (i < 0) {
            break
        }
        chain.push(pool.splice(i, 1)[0])
    }
    return chain
}

const isValidAt = (c: OpenedCertificate, now: Date) =>
    c.notBefore.getTime() <= now.getTime() && now.getTime() <= c.notAfter.getTime()

/** Without a key usage extension any use is allowed (RFC 5280). */
const signsWith = (c: OpenedCertificate) =>
    !c.keyUsage || c.keyUsage.digitalSignature || c.keyUsage.nonRepudiation

/** Valid now, then made for signing, then the latest expiry. */
const rankKey = (c: OpenedCertificate, now: Date): [number, number, number] => [
    isValidAt(c, now) ? 1 : 0,
    signsWith(c) ? 1 : 0,
    c.notAfter.getTime(),
]

export const rankCertificates = (
    candidates: OpenedCertificate[],
    now: Date = new Date()
): OpenedCertificates => {
    const ranked = [...candidates].sort((a, b) => {
        const [ka, kb] = [rankKey(a, now), rankKey(b, now)]
        return kb[0] - ka[0] || kb[1] - ka[1] || kb[2] - ka[2]
    })
    const [first, second] = ranked.map((c) => rankKey(c, now).join("/"))
    return {choices: ranked, ambiguous: second !== undefined && first === second}
}

const describe = async (
    privateKey: CryptoKey,
    algorithm: SignatureAlgorithm,
    chain: ParsedCertificate[],
    friendlyName: string | null,
    subtle: SubtleCrypto
): Promise<OpenedCertificate> => {
    const chainDer = chain.map((c) => c.der)
    const cert = chain[0].cert
    return {
        privateKey,
        algorithm,
        chainDer,
        chainPem: chainDer.map((d) => toPem("CERTIFICATE", d)),
        subject: formatName(cert.subject),
        issuer: formatName(cert.issuer),
        commonName: commonNameOf(cert.subject),
        issuerCommonName: commonNameOf(cert.issuer),
        serialNumber: hex(cert.serialNumber.valueBlock.valueHexView),
        notBefore: cert.notBefore.value,
        notAfter: cert.notAfter.value,
        keyUsage: readKeyUsage(cert),
        fingerprintSha256: hex(await sha256(chainDer[0], subtle)),
        spkiSha256: hex(
            await sha256(toBytes(cert.subjectPublicKeyInfo.toSchema().toBER()), subtle)
        ),
        friendlyName,
    }
}

/**
 * Opens a .p12/.pfx with its password: every key, as a non-extractable
 * WebCrypto key, with the certificate it proves to match and that
 * certificate's chain. The decrypted PKCS#8 bytes are zeroed.
 * Throws `CertificateFileError`.
 */
export const openP12Choices = async (
    fileBytes: Uint8Array,
    password: string,
    subtle: SubtleCrypto = globalThis.crypto.subtle,
    now: Date = new Date()
): Promise<OpenedCertificates> => {
    const {keys, certs} = openPfx(fileBytes, password)
    if (keys.length === 0) {
        throw new CertificateFileError(
            CertificateFileErrorCode.NoPrivateKey,
            "the file has no private key"
        )
    }
    const parsed: ParsedCertificate[] = certs
        .filter((bag) => bag.asn1)
        .map((bag) => {
            const der = binaryStringToBytes(forge.asn1.toDer(bag.asn1!).getBytes())
            return {
                der,
                cert: pkijs.Certificate.fromBER(toArrayBuffer(der)),
                localKeyId: bag.attributes.localKeyId?.[0],
            }
        })
    if (parsed.length === 0) {
        throw new CertificateFileError(
            CertificateFileErrorCode.NoCertificate,
            "the file has no certificate"
        )
    }

    const candidates: OpenedCertificate[] = []
    let firstError: CertificateFileError | null = null
    for (const keyBag of keys) {
        const pkcs8 = pkcs8FromBag(keyBag)
        let privateKey: CryptoKey
        let algorithm: SignatureAlgorithm
        try {
            algorithm = detectAlgorithm(pkcs8)
            privateKey = await subtle.importKey("pkcs8", pkcs8, importParams(algorithm), false, [
                "sign",
            ])
        } catch (error) {
            firstError ??=
                error instanceof CertificateFileError
                    ? error
                    : new CertificateFileError(
                          CertificateFileErrorCode.UnsupportedKey,
                          "the key could not be imported",
                          {cause: error}
                      )
            continue
        } finally {
            // Best effort: forge's intermediate binary strings can't be wiped.
            pkcs8.fill(0)
        }
        // The certificate paired by localKeyId first, but the key must prove it matches.
        const keyId = keyBag.attributes.localKeyId?.[0]
        const paired = (p: ParsedCertificate) => keyId !== undefined && p.localKeyId === keyId
        let leaf: ParsedCertificate | undefined
        for (const candidate of [...parsed.filter(paired), ...parsed.filter((p) => !paired(p))]) {
            if (await keyMatchesCertificate(privateKey, algorithm, candidate.cert, subtle)) {
                leaf = candidate
                break
            }
        }
        if (!leaf) {
            firstError ??= new CertificateFileError(
                CertificateFileErrorCode.KeyCertificateMismatch,
                "no certificate in the file matches its private key"
            )
            continue
        }
        candidates.push(
            await describe(
                privateKey,
                algorithm,
                orderChain(leaf, parsed),
                keyBag.attributes.friendlyName?.[0] ?? null,
                subtle
            )
        )
    }
    if (candidates.length === 0) {
        throw (
            firstError ??
            new CertificateFileError(CertificateFileErrorCode.NoPrivateKey, "no usable key")
        )
    }
    return rankCertificates(candidates, now)
}

/** The best key of the file with its certificate (see `openP12Choices`). */
export const openP12 = async (
    fileBytes: Uint8Array,
    password: string,
    subtle: SubtleCrypto = globalThis.crypto.subtle
): Promise<OpenedCertificate> => (await openP12Choices(fileBytes, password, subtle)).choices[0]
