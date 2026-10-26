// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {Certificate} from "pkijs"
import {fromBER} from "asn1js"
import {colonHex, fromBase64, hex, sha256, toArrayBuffer, toPem} from "@/lib/signing/der"

export interface CertificateAuthority {
    pem: string
    subject: string
    fingerprint: string
    expires: string
}
export interface ConfigurationSigning {
    policy: "required"
    package_roots: string
    staff_roots: string
    required_approvals: number
}
export const MAX_CERTIFICATE_FILE_BYTES = 256 * 1024

/** Parse public CA certificates locally; private material never reaches a mutation. */
export async function parseCertificateAuthorities(
    pem: string,
    now = new Date()
): Promise<CertificateAuthority[]> {
    if (pem.length > MAX_CERTIFICATE_FILE_BYTES)
        throw new Error("Certificate files must be at most 256 KiB.")
    const blocks = pem.match(/-----BEGIN CERTIFICATE-----[\s\S]*?-----END CERTIFICATE-----/g) ?? []
    if (
        !blocks.length ||
        pem.replace(/-----BEGIN CERTIFICATE-----[\s\S]*?-----END CERTIFICATE-----/g, "").trim()
    ) {
        throw new Error("Only public PEM certificates are accepted; do not upload private keys.")
    }
    const seen = new Set<string>()
    const result: CertificateAuthority[] = []
    for (const block of blocks) {
        const der = fromBase64(block.replace(/-----[A-Z ]+-----/g, "").replace(/\s/g, ""))
        const decoded = fromBER(toArrayBuffer(der))
        if (decoded.offset !== der.byteLength)
            throw new Error("The certificate contains invalid or trailing data.")
        const certificate = new Certificate({schema: decoded.result})
        const constraints = certificate.extensions?.find(({extnID}) => extnID === "2.5.29.19")
        if (!constraints?.parsedValue?.cA)
            throw new Error("Choose a CA certificate, not a person's signing certificate.")
        if (certificate.notBefore.value > now || certificate.notAfter.value <= now)
            throw new Error("Every CA certificate must be currently valid.")
        const fingerprint = colonHex(hex(await sha256(der)))
        if (seen.has(fingerprint)) throw new Error("The file contains a duplicate certificate.")
        seen.add(fingerprint)
        result.push({
            pem: toPem("CERTIFICATE", der),
            subject: certificate.subject.typesAndValues
                .map(
                    ({type, value}) =>
                        `${({"2.5.4.3": "CN", "2.5.4.6": "C", "2.5.4.10": "O", "2.5.4.11": "OU"} as Record<string, string>)[type] ?? type}=${value.valueBlock.value}`
                )
                .join(", "),
            fingerprint,
            expires: certificate.notAfter.value.toISOString().slice(0, 10),
        })
    }
    return result
}

export function signingConfiguration(
    packageRoots: string,
    staffRoots: string,
    approvals: number
): ConfigurationSigning {
    if (!Number.isInteger(approvals) || approvals < 2 || approvals > 65535)
        throw new Error("Required approvals must be a whole number from 2 to 65535.")
    if (!packageRoots.trim() || !staffRoots.trim())
        throw new Error("Both sets of CA certificates are required.")
    return {
        policy: "required",
        package_roots: packageRoots,
        staff_roots: staffRoots,
        required_approvals: approvals,
    }
}
