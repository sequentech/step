// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {X509Certificate, createHash, verify as nodeVerify, webcrypto} from "node:crypto"
import {readFileSync} from "node:fs"
import {join} from "node:path"
import * as asn1js from "asn1js"
import * as pkijs from "pkijs"
import {SigningApiError, SigningApiErrorKind, type ISigningApi, type ISigningPanelData} from "./api"
import {CMS_OID} from "./cms"
import {PayloadProblem, canonicalJson} from "./request"
import {openP12, signAndApprove} from "./sign"
import {
    RequesterSigning,
    SigningAction,
    SigningRequestStatus,
    SigningRequirement,
    type IApproveSigningRequestInput,
} from "./types"

if (!globalThis.crypto?.subtle) {
    Object.defineProperty(globalThis, "crypto", {value: webcrypto})
}

const FX = join(__dirname, "__fixtures__")
const PASSWORD = (
    JSON.parse(readFileSync(join(FX, "fixtures.json"), "utf8")) as {
        files: Record<string, {password: string}>
    }
).files["jose-reyes-ec-aes.p12"].password
const open = (name: string) => openP12(new Uint8Array(readFileSync(join(FX, name))), PASSWORD)
const sha256Hex = (bytes: string | Uint8Array) => createHash("sha256").update(bytes).digest("hex")
const EML = Buffer.from("<EML><Count>42</Count></EML>")
const PDF_DIGEST = createHash("sha256").update("ByteRange bytes").digest()

const panelFor = (action: SigningAction, subject: Record<string, unknown>): ISigningPanelData => {
    const canonical = canonicalJson({
        action,
        area_id: "ar-1",
        code: "7F3A-91C2",
        domain: "step-signing/v1",
        election_event_id: "ev-1",
        election_id: "el-1",
        request_id: "req-1",
        subject,
        tenant_id: "tn-1",
    })
    return {
        request: {
            id: "req-1",
            tenant_id: "tn-1",
            election_event_id: "ev-1",
            action,
            election_id: "el-1",
            area_id: "ar-1",
            trustee_id: null,
            subject,
            canonical_payload: canonical,
            payload_sha256: sha256Hex(canonical),
            document_id: null,
            document_sha256: null,
            code: "7F3A-91C2",
            config_revision: null,
            rule_revision: 1,
            required: 3,
            status: SigningRequestStatus.Waiting,
            cancel_reason: null,
            cancelled_by: null,
            requested_by: "u-maria",
            requested_by_username: "maria",
            created_at: "2028-05-12T10:30:00Z",
            expires_at: null,
            completed_at: null,
            executed_at: null,
            execution_result: null,
        },
        rule: {
            action,
            requirement: SigningRequirement.Required,
            signatures: 3,
            requester_signing: RequesterSigning.Allowed,
            expires_minutes: null,
            revision: 1,
        },
        count: 0,
        signers: [],
        document_url: "https://documents.invalid/doc",
    }
}

const fakeApi = (overrides: Partial<ISigningApi> = {}): jest.Mocked<ISigningApi> => {
    let revision = 1
    return {
        getRequest: jest.fn(),
        checkCertificate: jest.fn(),
        pdfPrepare: jest.fn(async () => ({
            revision: revision++,
            digest_b64: PDF_DIGEST.toString("base64"),
            signing_time: "2028-05-12T11:00:00Z",
        })),
        approve: jest.fn(async () => ({
            status: SigningRequestStatus.Waiting,
            count: 1,
            required: 3,
        })),
        reportOpenFailure: jest.fn(),
        handover: jest.fn(),
        cancel: jest.fn(),
        fetchDocument: jest.fn(async () => new Uint8Array(EML)),
        ...overrides,
    } as jest.Mocked<ISigningApi>
}

const sentApproval = (api: jest.Mocked<ISigningApi>, call = 0) =>
    api.approve.mock.calls[call][1] as IApproveSigningRequestInput

describe("signAndApprove", () => {
    it("signs the canonical payload bytes and sends the chain for an action without a document", async () => {
        const certificate = await open("jose-reyes-ec-aes.p12")
        const data = panelFor(SigningAction.CloseVoting, {channels: ["ONLINE"]})
        const api = fakeApi()

        await signAndApprove(api, data, certificate)

        expect(api.approve).toHaveBeenCalledTimes(1)
        expect(api.pdfPrepare).not.toHaveBeenCalled()
        expect(api.fetchDocument).not.toHaveBeenCalled()
        const approval = sentApproval(api)
        expect(api.approve.mock.calls[0][0]).toBe("req-1")
        expect(Object.keys(approval).sort()).toEqual(
            ["algorithm", "chain_pem", "payload_signature_b64"].sort()
        )
        expect(approval.algorithm).toBe(certificate.algorithm)
        expect(approval.chain_pem).toEqual(certificate.chainPem)
        const publicKey = new X509Certificate(certificate.chainPem[0]).publicKey
        const signature = Buffer.from(approval.payload_signature_b64, "base64")
        expect(
            nodeVerify("sha256", Buffer.from(data.request.canonical_payload), publicKey, signature)
        ).toBe(true)
    })

    it("signs the prepared PDF digest and sends the prepared revision", async () => {
        const certificate = await open("maria-santos-rsa-aes.p12")
        const api = fakeApi()
        await signAndApprove(
            api,
            panelFor(SigningAction.GenerateElectionReturns, {
                document_sha256: "ab".repeat(32),
                report_type: "er",
            }),
            certificate
        )

        expect(api.pdfPrepare).toHaveBeenCalledWith("req-1", {chain_pem: certificate.chainPem})
        const approval = sentApproval(api)
        expect(approval.revision).toBe(1)
        const cms = Buffer.from(approval.pdf_cms_b64!, "base64")
        const contentInfo = pkijs.ContentInfo.fromBER(
            cms.buffer.slice(cms.byteOffset, cms.byteOffset + cms.byteLength)
        )
        const signedData = new pkijs.SignedData({schema: contentInfo.content})
        const digest = signedData.signerInfos[0].signedAttrs?.attributes.find(
            (a) => a.type === CMS_OID.messageDigest
        )?.values[0] as asn1js.OctetString
        expect(Buffer.from(digest.valueBlock.valueHexView).equals(PDF_DIGEST)).toBe(true)
    })

    it("prepares the PDF once more when the prepared revision went stale", async () => {
        const certificate = await open("jose-reyes-ec-aes.p12")
        const api = fakeApi()
        api.approve.mockRejectedValueOnce(
            new SigningApiError(SigningApiErrorKind.Stale, "stale", {status: 409})
        )
        await signAndApprove(
            api,
            panelFor(SigningAction.GenerateReports, {
                document_sha256: "ab".repeat(32),
                report_type: "x",
            }),
            certificate
        )
        expect(api.pdfPrepare).toHaveBeenCalledTimes(2)
        expect(api.approve).toHaveBeenCalledTimes(2)
        expect(sentApproval(api, 1).revision).toBe(2)
    })

    it("reports a second stale revision instead of looping", async () => {
        const certificate = await open("jose-reyes-ec-aes.p12")
        const stale = new SigningApiError(SigningApiErrorKind.Stale, "stale", {status: 409})
        const api = fakeApi({approve: jest.fn(async () => Promise.reject(stale))})
        await expect(
            signAndApprove(
                api,
                panelFor(SigningAction.GenerateReports, {document_sha256: "ab".repeat(32)}),
                certificate
            )
        ).rejects.toBe(stale)
        expect(api.pdfPrepare).toHaveBeenCalledTimes(2)
    })

    it.each([SigningApiErrorKind.AlreadySigned, SigningApiErrorKind.Closed])(
        "doesn't prepare again after a %s refusal",
        async (kind) => {
            const certificate = await open("jose-reyes-ec-aes.p12")
            const refusal = new SigningApiError(kind, "refused", {status: 409})
            const api = fakeApi({approve: jest.fn(async () => Promise.reject(refusal))})
            await expect(
                signAndApprove(
                    api,
                    panelFor(SigningAction.GenerateReports, {document_sha256: "ab".repeat(32)}),
                    certificate
                )
            ).rejects.toBe(refusal)
            expect(api.pdfPrepare).toHaveBeenCalledTimes(1)
        }
    )

    it("signs the EML bytes the request names", async () => {
        const certificate = await open("maria-santos-rsa-rc2.p12")
        const api = fakeApi()
        const data = panelFor(SigningAction.TransmitResults, {eml_sha256: sha256Hex(EML)})
        await signAndApprove(api, data, certificate)

        expect(api.fetchDocument).toHaveBeenCalledWith("https://documents.invalid/doc")
        const approval = sentApproval(api)
        const publicKey = new X509Certificate(certificate.chainPem[0]).publicKey
        expect(
            nodeVerify(
                "sha256",
                EML,
                publicKey,
                Buffer.from(approval.document_signature_b64!, "base64")
            )
        ).toBe(true)
        expect(approval.pdf_cms_b64).toBeUndefined()
    })

    it("refuses to sign an EML that isn't the one the request names", async () => {
        const certificate = await open("maria-santos-rsa-rc2.p12")
        const api = fakeApi()
        const data = panelFor(SigningAction.TransmitResults, {
            eml_sha256: sha256Hex(Buffer.concat([EML, Buffer.from(" ")])),
        })
        await expect(signAndApprove(api, data, certificate)).rejects.toMatchObject({
            problem: PayloadProblem.Document,
        })
        expect(api.approve).not.toHaveBeenCalled()
    })

    it("sends nothing when the payload isn't the request's", async () => {
        const certificate = await open("jose-reyes-ec-aes.p12")
        const api = fakeApi()
        const data = panelFor(SigningAction.CloseVoting, {})
        data.request.code = "0000-0000"
        await expect(signAndApprove(api, data, certificate)).rejects.toMatchObject({
            problem: PayloadProblem.Mismatch,
        })
        expect(api.approve).not.toHaveBeenCalled()
    })
})
