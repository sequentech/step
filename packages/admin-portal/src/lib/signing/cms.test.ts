// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {X509Certificate, createHash, verify as nodeVerify, webcrypto} from "node:crypto"
import {readFileSync} from "node:fs"
import {join} from "node:path"
import * as asn1js from "asn1js"
import * as pkijs from "pkijs"
import {openP12} from "./certificate"
import {buildDetachedCms, CMS_OID} from "./cms"
import {hex, toArrayBuffer} from "./der"

if (!globalThis.crypto?.subtle) {
    Object.defineProperty(globalThis, "crypto", {value: webcrypto})
}

const FX = join(__dirname, "__fixtures__")
const manifest = JSON.parse(readFileSync(join(FX, "fixtures.json"), "utf8")) as {
    files: Record<string, {password: string; fingerprintSha256: string}>
}
const fixture = (name: string) => new Uint8Array(readFileSync(join(FX, name)))

const parse = (der: Uint8Array) => {
    const contentInfo = pkijs.ContentInfo.fromBER(toArrayBuffer(der))
    expect(contentInfo.contentType).toBe(CMS_OID.signedData)
    return new pkijs.SignedData({schema: contentInfo.content})
}

describe("buildDetachedCms", () => {
    it.each(["maria-santos-rsa-3des.p12", "jose-reyes-ec-aes.p12"])(
        "signs the server's digest with %s, detached and without signingTime",
        async (name) => {
            const {password, fingerprintSha256} = manifest.files[name]
            const opened = await openP12(fixture(name), password)
            // Stands in for the PDF ByteRange bytes; the browser only sees the digest.
            const content = Buffer.from("%PDF-1.7 ByteRange bytes the browser never sees")
            const digest = new Uint8Array(createHash("sha256").update(content).digest())

            const der = await buildDetachedCms({
                privateKey: opened.privateKey,
                chainDer: opened.chainDer,
                digest,
            })

            const signedData = parse(der)
            expect(signedData.encapContentInfo.eContentType).toBe(CMS_OID.data)
            expect(signedData.encapContentInfo.eContent).toBeUndefined()
            expect(signedData.certificates).toHaveLength(opened.chainDer.length)
            expect(signedData.signerInfos).toHaveLength(1)

            const signer = signedData.signerInfos[0]
            const attributes = signer.signedAttrs?.attributes ?? []
            expect(attributes.map((a) => a.type).sort()).toEqual(
                [CMS_OID.contentType, CMS_OID.messageDigest, CMS_OID.signingCertificateV2].sort()
            )
            expect(attributes.map((a) => a.type)).not.toContain(CMS_OID.signingTime)

            const messageDigest = attributes.find((a) => a.type === CMS_OID.messageDigest)
            const digestValue = messageDigest?.values[0] as asn1js.OctetString
            expect(hex(digestValue.valueBlock.valueHexView)).toBe(hex(digest))

            // ESSCertIDv2.certHash is the SHA-256 of the signer's certificate (openssl's fingerprint).
            const essAttr = attributes.find((a) => a.type === CMS_OID.signingCertificateV2)
            const ess = essAttr?.values[0] as asn1js.Sequence
            const certs = ess.valueBlock.value[0] as asn1js.Sequence
            const certId = certs.valueBlock.value[0] as asn1js.Sequence
            const certHash = certId.valueBlock.value[0] as asn1js.OctetString
            expect(hex(certHash.valueBlock.valueHexView)).toBe(fingerprintSha256)

            // The signature covers DER(SET OF Attribute): the [0] tag becomes a SET (0x31).
            const signedAttrs = new Uint8Array(signer.signedAttrs!.encodedValue)
            const asSet = new Uint8Array(signedAttrs)
            asSet[0] = 0x31
            const signature = new Uint8Array(signer.signature.valueBlock.valueHexView)
            const publicKey = new X509Certificate(opened.chainPem[0]).publicKey
            expect(nodeVerify("sha256", asSet, publicKey, signature)).toBe(true)
        }
    )

    it("refuses a digest that is not SHA-256", async () => {
        const {password} = manifest.files["jose-reyes-ec-aes.p12"]
        const opened = await openP12(fixture("jose-reyes-ec-aes.p12"), password)
        await expect(
            buildDetachedCms({
                privateKey: opened.privateKey,
                chainDer: opened.chainDer,
                digest: new Uint8Array(20),
            })
        ).rejects.toThrow("SHA-256")
    })
})
