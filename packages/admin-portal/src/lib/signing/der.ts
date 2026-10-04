// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Byte helpers shared by the signing modules. No dependencies.

/** Copies into a fresh Uint8Array backed by its own ArrayBuffer (WebCrypto and asn1js want exact buffers). */
export const toBytes = (b: ArrayBuffer | ArrayBufferView): Uint8Array<ArrayBuffer> =>
    b instanceof ArrayBuffer
        ? new Uint8Array(b.slice(0))
        : new Uint8Array((b.buffer as ArrayBuffer).slice(b.byteOffset, b.byteOffset + b.byteLength))

/** Exact-length ArrayBuffer for APIs that ignore byteOffset (asn1js valueHex, pkijs fromBER). */
export const toArrayBuffer = (b: Uint8Array): ArrayBuffer =>
    b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) as ArrayBuffer

export const hex = (b: ArrayBuffer | Uint8Array): string => {
    const bytes = b instanceof Uint8Array ? b : new Uint8Array(b)
    let out = ""
    for (let i = 0; i < bytes.length; i++) out += bytes[i].toString(16).padStart(2, "0")
    return out
}

/** "AB:CD:…" form of a lowercase hex fingerprint, as certificate viewers show it. */
export const colonHex = (lowerHex: string): string =>
    (lowerHex.match(/.{1,2}/g) ?? []).join(":").toUpperCase()

export const sha256 = async (
    data: Parameters<SubtleCrypto["digest"]>[1],
    subtle: SubtleCrypto = globalThis.crypto.subtle
): Promise<Uint8Array<ArrayBuffer>> => new Uint8Array(await subtle.digest("SHA-256", data))

export const bytesToBinaryString = (b: Uint8Array): string => {
    let s = ""
    for (let i = 0; i < b.length; i += 0x8000) {
        s += String.fromCharCode.apply(null, Array.from(b.subarray(i, i + 0x8000)))
    }
    return s
}

export const binaryStringToBytes = (s: string): Uint8Array<ArrayBuffer> => {
    const out = new Uint8Array(s.length)
    for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0xff
    return out
}

export const toBase64 = (b: Uint8Array): string => btoa(bytesToBinaryString(b))

export const fromBase64 = (s: string): Uint8Array<ArrayBuffer> => binaryStringToBytes(atob(s))

export const toPem = (label: string, der: Uint8Array): string => {
    const b64 = toBase64(der).replace(/(.{64})/g, "$1\n")
    return `-----BEGIN ${label}-----\n${b64}${b64.endsWith("\n") ? "" : "\n"}-----END ${label}-----\n`
}

const derLength = (n: number): number[] =>
    n < 0x80 ? [n] : n < 0x100 ? [0x81, n] : [0x82, n >> 8, n & 0xff]

const derInteger = (unsigned: Uint8Array): number[] => {
    let i = 0
    while (i < unsigned.length - 1 && unsigned[i] === 0) i++
    const value = Array.from(unsigned.subarray(i))
    if (value[0] & 0x80) value.unshift(0)
    return [0x02].concat(derLength(value.length), value)
}

/**
 * WebCrypto ECDSA returns IEEE P1363 r||s (64 bytes for P-256). OpenSSL, X.509
 * and CMS want DER `ECDSA-Sig-Value ::= SEQUENCE { r INTEGER, s INTEGER }`.
 */
export const ecdsaRawToDer = (raw: Uint8Array): Uint8Array<ArrayBuffer> => {
    if (raw.length === 0 || raw.length % 2 !== 0) {
        throw new Error(`bad P1363 signature length ${raw.length}`)
    }
    const n = raw.length / 2
    const body = derInteger(raw.subarray(0, n)).concat(derInteger(raw.subarray(n)))
    return Uint8Array.from([0x30].concat(derLength(body.length), body))
}
