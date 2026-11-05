// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// <reference path="../node-forge.d.ts" />

// Test helper: a .p12 holding several keys, which the openssl CLI can't write.
// It re-packs RSA fixtures (their key and certificates) with forge into one
// MAC-less file, 3DES-encrypted with the given ASCII password.

import forge from "node-forge/lib/forge"
import "node-forge/lib/pkcs12"
import "node-forge/lib/pbe"
import "node-forge/lib/aes"
import "node-forge/lib/des"
import "node-forge/lib/rc2"
import "node-forge/lib/sha256"
import "node-forge/lib/sha512"
import {binaryStringToBytes, bytesToBinaryString} from "../der"

export interface IP12Part {
    bytes: Uint8Array
    password: string
    friendlyName: string
}

type Bag = {key?: forge.pki.PrivateKey; cert?: forge.pki.Certificate}

const safeContents = (pfx: forge.asn1.Asn1): forge.asn1.Asn1[] => {
    // PFX ::= SEQUENCE {version, authSafe ContentInfo {type, [0] {OCTET STRING}}, …}
    const authSafe = (pfx.value as forge.asn1.Asn1[])[1]
    const octets = ((authSafe.value as forge.asn1.Asn1[])[1].value as forge.asn1.Asn1[])[0]
    return forge.asn1.fromDer(octets.value as string).value as forge.asn1.Asn1[]
}

export function multiKeyP12(parts: IP12Part[], password: string): Uint8Array<ArrayBuffer> {
    const pfxs = parts.map((part) => {
        const p12 = forge.pkcs12.pkcs12FromAsn1(
            forge.asn1.fromDer(bytesToBinaryString(part.bytes)),
            false,
            part.password
        )
        const bags = (type: string) => (p12.getBags({bagType: type})[type] ?? []) as Bag[]
        const key = bags(forge.pki.oids.pkcs8ShroudedKeyBag)[0]?.key
        const certs = bags(forge.pki.oids.certBag).map((bag) => bag.cert!)
        if (!key) throw new Error("the part has no RSA key forge can read")
        return forge.pkcs12.toPkcs12Asn1(key as forge.pki.rsa.PrivateKey, certs, password, {
            algorithm: "3des",
            useMac: false,
            friendlyName: part.friendlyName,
        })
    })
    const [first] = pfxs
    const merged = forge.asn1.create(
        forge.asn1.Class.UNIVERSAL,
        forge.asn1.Type.SEQUENCE,
        true,
        pfxs.flatMap(safeContents)
    )
    const authSafe = (first.value as forge.asn1.Asn1[])[1]
    const octets = ((authSafe.value as forge.asn1.Asn1[])[1].value as forge.asn1.Asn1[])[0]
    octets.value = forge.asn1.toDer(merged).getBytes()
    return binaryStringToBytes(forge.asn1.toDer(first).getBytes())
}
