// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The signing dialog's crypto entry point, loaded with import() when the
// dialog opens: opening the certificate file and making one approval.

import {SigningApiError, SigningApiErrorKind, type ISigningApi, type ISigningPanelData} from "./api"
import {signPayload, type OpenedCertificate} from "./certificate"
import {buildDetachedCms} from "./cms"
import {fromBase64, toBase64} from "./der"
import {
    PayloadMismatchError,
    PayloadProblem,
    checkDocument,
    documentKindOf,
    payloadToSign,
} from "./request"
import {DocumentKind, type IApproveSigningRequestOutput} from "./types"

export {
    CertificateFileError,
    CertificateFileErrorCode,
    openFailureReason,
    openP12,
    openP12Choices,
} from "./certificate"

/** A stale prepared revision is prepared once more; a second 409 is reported. */
const PDF_ATTEMPTS = 2

const approvePdf = async (
    api: ISigningApi,
    requestId: string,
    certificate: OpenedCertificate,
    base: {
        chain_pem: string[]
        algorithm: OpenedCertificate["algorithm"]
        payload_signature_b64: string
    }
): Promise<IApproveSigningRequestOutput> => {
    for (let attempt = 1; ; attempt++) {
        const prepared = await api.pdfPrepare(requestId, {chain_pem: certificate.chainPem})
        const digest = fromBase64(prepared.digest_b64)
        const cms = await buildDetachedCms({
            privateKey: certificate.privateKey,
            chainDer: certificate.chainDer,
            digest,
        })
        try {
            return await api.approve(requestId, {
                ...base,
                pdf_cms_b64: toBase64(cms),
                revision: prepared.revision,
            })
        } catch (error) {
            const stale =
                error instanceof SigningApiError && error.kind === SigningApiErrorKind.Stale
            if (!stale || attempt >= PDF_ATTEMPTS) {
                throw error
            }
        }
    }
}

/**
 * Signs the request with the opened certificate and sends the approval.
 * `data` is the request as just fetched (its document URL is fresh). Every
 * approval signs the canonical payload; a PDF request adds the detached CMS
 * over the prepared revision's digest, an EML request a signature over the
 * EML bytes, which must hash to the eml_sha256 the signed payload names.
 */
export const signAndApprove = async (
    api: ISigningApi,
    data: ISigningPanelData,
    certificate: OpenedCertificate
): Promise<IApproveSigningRequestOutput> => {
    const {request} = data
    const {bytes, view} = await payloadToSign(data)
    const base = {
        chain_pem: certificate.chainPem,
        algorithm: certificate.algorithm,
        payload_signature_b64: toBase64(
            await signPayload(certificate.privateKey, certificate.algorithm, bytes)
        ),
    }
    switch (documentKindOf(request)) {
        case DocumentKind.Pdf:
            return approvePdf(api, request.id, certificate, base)
        case DocumentKind.Eml: {
            if (!data.document_url) {
                throw new PayloadMismatchError(
                    PayloadProblem.Document,
                    "the request has no document to download"
                )
            }
            const document = await api.fetchDocument(data.document_url)
            await checkDocument(document, view)
            const signature = await signPayload(
                certificate.privateKey,
                certificate.algorithm,
                document
            )
            return api.approve(request.id, {...base, document_signature_b64: toBase64(signature)})
        }
        default:
            return api.approve(request.id, base)
    }
}
