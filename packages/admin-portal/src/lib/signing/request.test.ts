// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {createHash, webcrypto} from "node:crypto"
import {
    PayloadMismatchError,
    PayloadProblem,
    RESUME_KEY,
    SignBlock,
    canonicalJson,
    checkDocument,
    commonNameOf,
    isOpenForSigning,
    payloadToSign,
    pendingSigners,
    resumeFor,
    signBlock,
    signedView,
    signerCertificate,
    signerName,
    signerOf,
    takeResume,
    writeResume,
} from "./request"
import type {ISigningPanelData} from "./api"
import {
    RequesterSigning,
    SigningAction,
    SigningRequestStatus,
    SigningRequirement,
    type ISigningRequestPanel,
} from "./types"

if (!globalThis.crypto?.subtle) {
    Object.defineProperty(globalThis, "crypto", {value: webcrypto})
}

const NOW = new Date("2028-05-12T11:00:00Z")
// Sorted keys, no whitespace; "é" makes sure the UTF-8 bytes are signed.
const CANONICAL =
    '{"action":"close-voting","area_id":null,"code":"7F3A-91C2","domain":"step-signing/v1",' +
    '"election_event_id":"ev-1","election_id":"el-1","request_id":"req-1",' +
    '"subject":{"channels":["ONLINE"],"note":"é"},"tenant_id":"tn-1"}'
const sha256Hex = (text: string) => createHash("sha256").update(text, "utf8").digest("hex")

const panel = (overrides: Partial<ISigningRequestPanel["request"]> = {}): ISigningRequestPanel => ({
    request: {
        id: "req-1",
        tenant_id: "tn-1",
        election_event_id: "ev-1",
        action: SigningAction.CloseVoting,
        election_id: "el-1",
        area_id: null,
        trustee_id: null,
        subject: {channels: ["ONLINE"]},
        canonical_payload: CANONICAL,
        payload_sha256: sha256Hex(CANONICAL),
        document_id: null,
        document_sha256: null,
        code: "7F3A-91C2",
        config_revision: null,
        rule_revision: 1,
        required: 2,
        status: SigningRequestStatus.Waiting,
        cancel_reason: null,
        cancelled_by: null,
        requested_by: "u-maria",
        requested_by_username: "maria",
        created_at: "2028-05-12T10:30:00Z",
        expires_at: "2028-05-12T11:30:00Z",
        completed_at: null,
        executed_at: null,
        execution_result: null,
        ...overrides,
    },
    rule: {
        action: SigningAction.CloseVoting,
        requirement: SigningRequirement.Required,
        signatures: 2,
        requester_signing: RequesterSigning.Allowed,
        expires_minutes: 60,
        revision: 1,
    },
    count: 1,
    signers: [
        {
            user_id: "u-maria",
            username: "maria",
            display_name: "Maria L. Santos",
            title: "Chairperson",
            signed_at: "2028-05-12T10:31:00Z",
            certificate_subject: "C=PH, O=Test PNPKI, CN=MARIA L. SANTOS, serialNumber=PH-0001",
        },
        {
            user_id: "u-jose",
            username: "jose",
            display_name: "Jose R. Dela Cruz",
            title: "Poll Clerk",
            signed_at: null,
            certificate_subject: null,
        },
        {
            user_id: "u-ana",
            username: "ana",
            display_name: "ana",
            title: null,
            signed_at: null,
            certificate_subject: null,
        },
    ],
    document_url: null,
})

describe("signBlock", () => {
    it("lets an eligible member who hasn't signed sign an open request", () => {
        expect(signBlock(panel(), "u-jose", true, NOW)).toBe(SignBlock.None)
    })

    it.each([
        [
            "expired by time",
            panel(),
            "u-jose",
            true,
            new Date("2028-05-12T11:30:00Z"),
            SignBlock.Closed,
        ],
        [
            "cancelled",
            panel({status: SigningRequestStatus.Cancelled}),
            "u-jose",
            true,
            NOW,
            SignBlock.Closed,
        ],
        [
            "completed",
            panel({status: SigningRequestStatus.Completed}),
            "u-jose",
            true,
            NOW,
            SignBlock.Closed,
        ],
        ["without sign permission", panel(), "u-jose", false, NOW, SignBlock.NoPermission],
        ["already signed", panel(), "u-maria", true, NOW, SignBlock.AlreadySigned],
        ["not among the signers", panel(), "u-other", true, NOW, SignBlock.NotASigner],
    ])("blocks %s", (_label, data, userId, permission, now, expected) => {
        expect(signBlock(data, userId, permission, now)).toBe(expected)
    })

    it("blocks the requester only when the rule doesn't allow them to sign", () => {
        const data = panel({requested_by: "u-jose"})
        expect(signBlock(data, "u-jose", true, NOW)).toBe(SignBlock.None)
        data.rule.requester_signing = RequesterSigning.NotAllowed
        expect(signBlock(data, "u-jose", true, NOW)).toBe(SignBlock.Requester)
    })

    it("keeps a request without expiry open", () => {
        expect(isOpenForSigning(panel({expires_at: null}).request, new Date("2099-01-01"))).toBe(
            true
        )
    })
})

describe("pendingSigners", () => {
    it("lists who hasn't signed, without the viewer", () => {
        expect(pendingSigners(panel()).map((s) => s.user_id)).toEqual(["u-jose", "u-ana"])
        expect(pendingSigners(panel(), "u-jose").map((s) => s.user_id)).toEqual(["u-ana"])
    })
})

/** The panel with another canonical payload, hashed consistently. */
const withPayload = (
    change: (payload: Record<string, unknown>) => void,
    overrides: Partial<ISigningRequestPanel["request"]> = {},
    extra: Partial<ISigningPanelData> = {}
): ISigningPanelData => {
    const payload = JSON.parse(CANONICAL) as Record<string, unknown>
    change(payload)
    const canonical = canonicalJson(payload)
    return {
        ...panel({
            canonical_payload: canonical,
            payload_sha256: sha256Hex(canonical),
            ...overrides,
        }),
        ...extra,
    }
}

describe("signers", () => {
    it("names a signer by display name, the username without one", () => {
        const [maria, , ana] = panel().signers
        expect(signerName(maria)).toBe("Maria L. Santos")
        expect(signerName({...ana, display_name: ""})).toBe("ana")
    })

    it("finds the viewer by the server's is_you or the user id", () => {
        const data = panel()
        data.signers[2].is_you = true
        expect(signerOf(data, "someone-else")?.user_id).toBe("u-ana")
        expect(signerOf(data, "u-jose")?.user_id).toBe("u-jose")
        expect(pendingSigners(data, "someone-else").map((s) => s.user_id)).toEqual(["u-jose"])
    })

    it("names the certificate by its CN, from certificate_cn or the subject", () => {
        const [maria] = panel().signers
        expect(signerCertificate(maria)).toBe("MARIA L. SANTOS")
        expect(signerCertificate({...maria, certificate_cn: "OTHER CN"})).toBe("OTHER CN")
        expect(signerCertificate({...maria, certificate_subject: null})).toBeNull()
    })
})

describe("payloadToSign", () => {
    it("returns the canonical payload's UTF-8 bytes", async () => {
        const {bytes} = await payloadToSign(panel())
        expect(Buffer.from(bytes).equals(Buffer.from(CANONICAL, "utf8"))).toBe(true)
    })

    it.each([
        ["request_id", {id: "req-2"}],
        ["code", {code: "0000-0000"}],
        ["action", {action: SigningAction.OpenVoting}],
        ["tenant_id", {tenant_id: "tn-2"}],
        ["election_event_id", {election_event_id: "ev-2"}],
        ["election_id", {election_id: "el-2"}],
        ["area_id", {area_id: "ar-1"}],
    ])("refuses a payload whose %s isn't the request's", async (_field, overrides) => {
        await expect(payloadToSign(panel(overrides))).rejects.toMatchObject({
            problem: PayloadProblem.Mismatch,
        })
    })

    it("refuses a payload that doesn't hash to payload_sha256", async () => {
        await expect(
            payloadToSign(panel({payload_sha256: sha256Hex(CANONICAL + " ")}))
        ).rejects.toMatchObject({problem: PayloadProblem.Digest})
    })

    it("accepts an upper-case payload_sha256", async () => {
        const {bytes} = await payloadToSign(
            panel({payload_sha256: sha256Hex(CANONICAL).toUpperCase()})
        )
        expect(bytes).toHaveLength(Buffer.byteLength(CANONICAL))
    })

    it.each([
        ["not JSON", CANONICAL.slice(0, -1), PayloadProblem.Unparseable],
        ["an array", "[]", PayloadProblem.Unparseable],
        ["with whitespace", CANONICAL.replace(":", ": "), PayloadProblem.NotCanonical],
        [
            "with unsorted keys",
            '{"code":"7F3A-91C2",' + CANONICAL.slice(1).replace('"code":"7F3A-91C2",', ""),
            PayloadProblem.NotCanonical,
        ],
        [
            "of another domain",
            CANONICAL.replace("step-signing/v1", "step-signing/v2"),
            PayloadProblem.NotCanonical,
        ],
    ])("refuses a payload %s", async (_label, canonical, problem) => {
        const error = await payloadToSign(
            panel({canonical_payload: canonical, payload_sha256: sha256Hex(canonical)})
        ).catch((e: unknown) => e)
        expect(error).toBeInstanceOf(PayloadMismatchError)
        expect((error as PayloadMismatchError).problem).toBe(problem)
    })
})

describe("signedView", () => {
    it("shows the subject the payload signs, not the request's copy", () => {
        const view = signedView(panel({subject: {channels: ["KIOSK"]}}))
        expect(view.rows).toEqual([
            {key: "channels", label: null, value: "ONLINE"},
            {key: "note", label: null, value: "é"},
        ])
    })

    it("labels subject values with the server's details, in their order", () => {
        const data = withPayload(
            (p) => (p.subject = {application_id: "APP-1", seats: 3}),
            {},
            {details: [{key: "seats", label: "Seats", value: "3"}]}
        )
        expect(signedView(data).rows).toEqual([
            {key: "seats", label: "Seats", value: "3"},
            {key: "application_id", label: null, value: "APP-1"},
        ])
    })

    it("names a trustee request's ceremony and trustee only beside the ids the panel names", () => {
        const trustee = (extra: Partial<ISigningPanelData>, trusteeId = "tr-1") =>
            signedView(
                withPayload(
                    (p) =>
                        (p.subject = {
                            keys_ceremony_id: "kc-1",
                            key_share_sha256: "ab",
                            trustee_id: "tr-1",
                        }),
                    {trustee_id: trusteeId},
                    extra
                )
            ).rows
        const names = {ceremony_id: "kc-1", ceremony_name: "Madrid PE keys", trustee_name: "Jose"}
        expect(trustee(names)).toEqual([
            {key: "key_share_sha256", label: null, value: "ab"},
            {key: "keys_ceremony_id", label: null, value: "kc-1", name: "Madrid PE keys"},
            {key: "trustee_id", label: null, value: "tr-1", name: "Jose"},
        ])
        // A name the panel gives another id is not shown beside the signed one.
        expect(trustee({...names, ceremony_id: "kc-2"}, "tr-2").map((row) => row.name)).toEqual([
            undefined,
            undefined,
            undefined,
        ])
    })

    it.each([
        ["a value the payload doesn't sign", {key: "channels", value: "KIOSK"}],
        ["a field the payload doesn't have", {key: "why", value: "No automatic match"}],
    ])("refuses details showing %s", (_label, detail) => {
        expect(() => signedView({...panel(), details: [detail]})).toThrow(
            expect.objectContaining({problem: PayloadProblem.Mismatch})
        )
    })

    it("takes a PDF's hash from the payload and requires the card to agree", () => {
        const pdf = (subject: Record<string, unknown>, shown: string | null) =>
            withPayload(
                (p) => {
                    p.action = SigningAction.GenerateReports
                    p.subject = subject
                },
                {action: SigningAction.GenerateReports, document_sha256: shown}
            )
        expect(signedView(pdf({document_sha256: "AB12"}, "ab12")).documentSha256).toBe("ab12")
        expect(signedView(pdf({document_sha256: "ab12"}, null)).documentSha256).toBe("ab12")
        for (const data of [pdf({document_sha256: "ab12"}, "cd34"), pdf({}, "ab12")]) {
            expect(() => signedView(data)).toThrow(
                expect.objectContaining({problem: PayloadProblem.Document})
            )
        }
    })

    it("takes an EML's hash from the payload's eml_sha256", () => {
        const eml = withPayload(
            (p) => {
                p.action = SigningAction.TransmitResults
                p.subject = {eml_sha256: "EF56", package_sha256: "0000"}
            },
            {action: SigningAction.TransmitResults, subject: {eml_sha256: "ffff"}}
        )
        expect(signedView(eml).documentSha256).toBe("ef56")
    })

    it("checks downloaded bytes against that hash", async () => {
        const bytes = Buffer.from("<EML/>")
        const view = {...signedView(panel()), documentSha256: sha256Hex("<EML/>")}
        await expect(checkDocument(new Uint8Array(bytes), view)).resolves.toBeUndefined()
        await expect(
            checkDocument(new Uint8Array(Buffer.from("<EML />")), view)
        ).rejects.toMatchObject({problem: PayloadProblem.Document})
    })
})

describe("canonicalJson", () => {
    it("sorts keys recursively, keeps arrays in order and refuses floats", () => {
        expect(canonicalJson({b: [2, {d: 1, c: "é"}], a: null})).toBe(
            '{"a":null,"b":[2,{"c":"é","d":1}]}'
        )
        expect(() => canonicalJson({a: 1.5})).toThrow()
    })
})

describe("commonNameOf", () => {
    it.each([
        ["C=PH, O=Test PNPKI, CN=MARIA L. SANTOS, serialNumber=PH-0001", "MARIA L. SANTOS"],
        ["/C=PH/O=Test/CN=JOSE P. REYES", "JOSE P. REYES"],
        ["CN=ANA M. CRUZ", "ANA M. CRUZ"],
        ["ANA M. CRUZ", "ANA M. CRUZ"],
    ])("reads %s", (subject, expected) => {
        expect(commonNameOf(subject)).toBe(expected)
    })
})

describe("handover note", () => {
    const memoryStorage = (): Storage => {
        const items = new Map<string, string>()
        return {
            get length() {
                return items.size
            },
            clear: () => items.clear(),
            getItem: (key) => items.get(key) ?? null,
            key: (index) => Array.from(items.keys())[index] ?? null,
            removeItem: (key) => void items.delete(key),
            setItem: (key, value) => void items.set(key, value),
        }
    }
    const note = resumeFor(panel().request, NOW)

    it("expires with the request, or after 30 minutes", () => {
        expect(note).toEqual({
            requestId: "req-1",
            tenantId: "tn-1",
            eventId: "ev-1",
            expiresAt: "2028-05-12T11:30:00.000Z",
        })
        expect(resumeFor(panel({expires_at: "2028-05-12T11:10:00Z"}).request, NOW).expiresAt).toBe(
            "2028-05-12T11:10:00.000Z"
        )
        expect(resumeFor(panel({expires_at: null}).request, NOW).expiresAt).toBe(
            "2028-05-12T11:30:00.000Z"
        )
    })

    it("is read once, in its tenant, before it expires", () => {
        const storage = memoryStorage()
        writeResume(storage, note)
        expect(takeResume(storage, {tenantId: "tn-1", now: NOW})).toEqual(note)
        expect(takeResume(storage, {tenantId: "tn-1", now: NOW})).toBeNull()
    })

    it.each([
        ["another tenant", "tn-2", NOW],
        ["no tenant", null, NOW],
        ["an expired note", "tn-1", new Date("2028-05-12T11:30:00Z")],
    ])("ignores %s and drops it", (_label, tenantId, now) => {
        const storage = memoryStorage()
        writeResume(storage, note)
        expect(takeResume(storage, {tenantId, now})).toBeNull()
        expect(storage.getItem(RESUME_KEY)).toBeNull()
    })

    it.each(["not json", "{}", '{"requestId":"","tenantId":"tn-1","eventId":"ev-1"}'])(
        "drops a malformed note %s",
        (raw) => {
            const storage = memoryStorage()
            storage.setItem(RESUME_KEY, raw)
            expect(takeResume(storage, {tenantId: "tn-1", now: NOW})).toBeNull()
            expect(storage.getItem(RESUME_KEY)).toBeNull()
        }
    )

    it("survives storage that refuses access", () => {
        const refusing = {
            getItem: () => {
                throw new Error("SecurityError")
            },
        } as unknown as Storage
        expect(takeResume(refusing, {tenantId: "tn-1", now: NOW})).toBeNull()
    })
})
