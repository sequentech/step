// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A detached CMS SignedData (the PAdES baseline B-B shape) over a digest the
// server computed: the PDF revision's ByteRange digest (design §6). The PDF
// never reaches the browser, only its SHA-256.
//
// Signed attributes: contentType, messageDigest and signingCertificateV2.
// There is no signingTime: PAdES forbids it; the time is in the /Sig
// dictionary's /M, written by the server.

import * as asn1js from "asn1js"
import * as pkijs from "pkijs"
import {sha256, toArrayBuffer, toBytes} from "./der"

export const CMS_OID = {
    data: "1.2.840.113549.1.7.1",
    signedData: "1.2.840.113549.1.7.2",
    contentType: "1.2.840.113549.1.9.3",
    messageDigest: "1.2.840.113549.1.9.4",
    signingTime: "1.2.840.113549.1.9.5",
    signingCertificateV2: "1.2.840.113549.1.9.16.2.47",
}

/**
 * RFC 5035 SigningCertificateV2 ::= SEQUENCE { certs SEQUENCE OF ESSCertIDv2 }
 * ESSCertIDv2 ::= SEQUENCE { hashAlgorithm DEFAULT sha256 (omitted in DER),
 *   certHash OCTET STRING, issuerSerial IssuerSerial }
 * IssuerSerial ::= SEQUENCE { issuer GeneralNames, serialNumber INTEGER }
 */
const signingCertificateV2 = async (cert: pkijs.Certificate, certDer: Uint8Array<ArrayBuffer>) =>
    new asn1js.Sequence({
        value: [
            new asn1js.Sequence({
                value: [
                    new asn1js.Sequence({
                        value: [
                            new asn1js.OctetString({
                                valueHex: toArrayBuffer(await sha256(certDer)),
                            }),
                            new asn1js.Sequence({
                                value: [
                                    // GeneralNames ::= SEQUENCE OF GeneralName; directoryName [4] EXPLICIT Name
                                    new asn1js.Sequence({
                                        value: [
                                            new asn1js.Constructed({
                                                idBlock: {tagClass: 3, tagNumber: 4},
                                                value: [cert.issuer.toSchema()],
                                            }),
                                        ],
                                    }),
                                    new asn1js.Integer({
                                        valueHex: toArrayBuffer(
                                            new Uint8Array(
                                                cert.serialNumber.valueBlock.valueHexView
                                            )
                                        ),
                                    }),
                                ],
                            }),
                        ],
                    }),
                ],
            }),
        ],
    })

export interface DetachedCmsInput {
    privateKey: CryptoKey
    /** Leaf first. All of it goes into SignedData.certificates. */
    chainDer: Uint8Array<ArrayBuffer>[]
    /** SHA-256 of the detached content, computed by the server. */
    digest: Uint8Array
}

/** Returns the DER ContentInfo(SignedData). The signer is identified by issuer and serial number. */
export const buildDetachedCms = async ({
    privateKey,
    chainDer,
    digest,
}: DetachedCmsInput): Promise<Uint8Array<ArrayBuffer>> => {
    if (digest.length !== 32) {
        throw new Error("digest must be SHA-256 (32 bytes)")
    }
    if (chainDer.length === 0) {
        throw new Error("the certificate chain is empty")
    }
    const chain = chainDer.map((d) => pkijs.Certificate.fromBER(toArrayBuffer(d)))
    const leaf = chain[0]

    const signerInfo = new pkijs.SignerInfo({
        version: 1,
        sid: new pkijs.IssuerAndSerialNumber({
            issuer: leaf.issuer,
            serialNumber: leaf.serialNumber,
        }),
        signedAttrs: new pkijs.SignedAndUnsignedAttributes({
            type: 0,
            attributes: [
                new pkijs.Attribute({
                    type: CMS_OID.contentType,
                    values: [new asn1js.ObjectIdentifier({value: CMS_OID.data})],
                }),
                new pkijs.Attribute({
                    type: CMS_OID.messageDigest,
                    values: [new asn1js.OctetString({valueHex: toArrayBuffer(digest)})],
                }),
                new pkijs.Attribute({
                    type: CMS_OID.signingCertificateV2,
                    values: [await signingCertificateV2(leaf, chainDer[0])],
                }),
            ],
        }),
    })

    const signedData = new pkijs.SignedData({
        version: 1,
        // No eContent: detached.
        encapContentInfo: new pkijs.EncapsulatedContentInfo({eContentType: CMS_OID.data}),
        signerInfos: [signerInfo],
        certificates: chain,
    })
    // With signedAttrs, pkijs signs DER(SET OF Attribute) and converts ECDSA to DER itself.
    const engine = new pkijs.CryptoEngine({name: "signing", crypto: globalThis.crypto})
    await signedData.sign(privateKey, 0, "SHA-256", undefined, engine)

    const contentInfo = new pkijs.ContentInfo({
        contentType: CMS_OID.signedData,
        content: signedData.toSchema(true),
    })
    return toBytes(contentInfo.toSchema().toBER())
}
