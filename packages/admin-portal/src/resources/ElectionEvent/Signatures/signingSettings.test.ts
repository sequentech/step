// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IPermissions} from "@/types/keycloak"
import {
    RequesterSigning,
    SigningAction,
    SigningActionGroup,
    SigningRequestStatus,
    SigningRequirement,
    StaffCertificateStatus,
    type ISigningRequestListRow,
    type ISigningRule,
    type ISigningRuleCapacity,
} from "@/lib/signing/types"
import {
    CertificateDisplayStatus,
    SignaturesProblem,
    actionGroups,
    certificateStatus,
    checkSignatures,
    commonName,
    MAX_SIGNATURES,
    SigningErrorCode,
    afterLockdown,
    certificateFingerprint,
    errorKey,
    linkTarget,
    issuerUpload,
    pemToDer,
    personName,
    signingError,
    draftChanged,
    draftOf,
    expiryKey,
    filterCertificates,
    filterRequests,
    lastChange,
    lastSignature,
    parseSignatures,
    requestStatus,
    ruleInputOf,
    ruleOf,
    shortFingerprint,
    signaturesAccess,
    toPem,
} from "./signingSettings"

const holding =
    (...permissions: IPermissions[]) =>
    (permission: IPermissions) =>
        permissions.includes(permission)

describe("signaturesAccess", () => {
    it("shows the tab only with the tab permission and one read permission", () => {
        expect(signaturesAccess(holding(IPermissions.ELECTION_EVENT_SIGNATURES_TAB)).tab).toBe(
            false
        )
        expect(signaturesAccess(holding(IPermissions.SIGNING_REQUESTS_READ)).tab).toBe(false)
        for (const read of [
            IPermissions.SIGNING_RULES_READ,
            IPermissions.SIGNING_CERTIFICATES_READ,
            IPermissions.SIGNING_REQUESTS_READ,
        ]) {
            expect(
                signaturesAccess(holding(IPermissions.ELECTION_EVENT_SIGNATURES_TAB, read)).tab
            ).toBe(true)
        }
    })

    it("lets roles change from the drawer only with role-read, role-write and the rules' write", () => {
        const all = [
            IPermissions.SIGNING_RULES_READ,
            IPermissions.SIGNING_RULES_WRITE,
            IPermissions.ROLE_READ,
            IPermissions.ROLE_WRITE,
        ]
        expect(signaturesAccess(holding(...all))).toMatchObject({
            rulesWrite: true,
            rolesWrite: true,
        })
        // Each of the three is needed: the options come from /get-roles (role-read), the save
        // goes through the rule (rules' write) and changes roles (role-write).
        for (const missing of all.slice(1)) {
            const access = signaturesAccess(holding(...all.filter((p) => p !== missing)))
            expect(access.rolesWrite).toBe(false)
        }
    })

    it("makes the rules read-only after lockdown, and nothing else", () => {
        const access = signaturesAccess(
            holding(
                IPermissions.SIGNING_RULES_WRITE,
                IPermissions.ROLE_READ,
                IPermissions.ROLE_WRITE,
                IPermissions.SIGNING_CHECKS_WRITE
            )
        )
        expect(afterLockdown(access, false)).toBe(access)
        expect(afterLockdown(access, true)).toEqual({
            ...access,
            rulesWrite: false,
            rolesWrite: false,
        })
    })

    it("maps each certificate and request control to its own permission", () => {
        const access = signaturesAccess(
            holding(
                IPermissions.SIGNING_CERTIFICATES_READ,
                IPermissions.SIGNING_CHECKS_WRITE,
                IPermissions.SIGNING_REQUESTS_EXPORT
            )
        )
        expect(access).toMatchObject({
            certificatesRead: true,
            issuersWrite: false,
            checksWrite: true,
            certificatesRegister: false,
            certificatesRevoke: false,
            requestsRead: false,
            requestsCancel: false,
            requestsExport: true,
        })
    })
})

describe("rules", () => {
    const saved: ISigningRule = {
        action: SigningAction.CloseVoting,
        requirement: SigningRequirement.Required,
        signatures: 2,
        requester_signing: RequesterSigning.Allowed,
        expires_minutes: 30,
        revision: 4,
    }

    it("keeps the documented default for an action without a row", () => {
        expect(ruleOf(SigningAction.OpenVoting, [saved])).toEqual({
            action: SigningAction.OpenVoting,
            requirement: SigningRequirement.NotRequired,
            signatures: 1,
            requester_signing: RequesterSigning.NotAllowed,
            expires_minutes: 60,
            revision: 0,
        })
        expect(ruleOf(SigningAction.CloseVoting, [saved])).toBe(saved)
    })

    it("groups the ten actions in catalog order", () => {
        expect(actionGroups()).toEqual([
            {
                group: SigningActionGroup.Voting,
                actions: [
                    SigningAction.InitializeVoting,
                    SigningAction.OpenVoting,
                    SigningAction.CloseVoting,
                ],
            },
            {
                group: SigningActionGroup.ResultsAndReports,
                actions: [
                    SigningAction.GenerateElectionReturns,
                    SigningAction.GenerateReports,
                    SigningAction.TransmitResults,
                ],
            },
            {group: SigningActionGroup.Enrollment, actions: [SigningAction.ApproveVoter]},
            {
                group: SigningActionGroup.ConfigurationAndKeys,
                actions: [
                    SigningAction.ApproveConfiguration,
                    SigningAction.ConfirmKeyShare,
                    SigningAction.ContributeKeyShare,
                ],
            },
        ])
    })

    it("names each expiry option and keeps other stored values", () => {
        expect([30, 60, 120, 1440, null].map(expiryKey)).toEqual([
            "30",
            "60",
            "120",
            "1440",
            "none",
        ])
        expect(expiryKey(45)).toBe("other")
    })

    it("finds the latest change and who made it", () => {
        expect(lastChange([])).toBeNull()
        expect(
            lastChange([
                {
                    ...saved,
                    updated_by_name: "A",
                    updated_at: "2028-05-01T10:00:00Z",
                },
                {
                    ...saved,
                    updated_by_name: "B",
                    updated_at: "2028-05-02T10:00:00Z",
                },
                {...saved, updated_at: "2028-04-30T10:00:00Z"},
            ])
        ).toEqual({name: "B", updated_at: "2028-05-02T10:00:00Z"})
        // Rows saved before names were stored have none.
        expect(lastChange([{...saved, updated_at: "2028-04-30T10:00:00Z"}])).toEqual({
            name: null,
            updated_at: "2028-04-30T10:00:00Z",
        })
    })
})

describe("checkSignatures", () => {
    const capacity: ISigningRuleCapacity = {
        max: 3,
        posts: [
            {election_id: "madrid", name: "Madrid PE", count: 3},
            {election_id: "wellington", name: "Wellington PE", count: 2},
            {election_id: "dili", name: "Dili PE", count: 2},
            {election_id: "rome", name: "Rome PE", count: 1},
        ],
        posts_short: [],
        posts_short_without_requester: [],
        roles: [{id: "group-sbei", name: "SBEI", path: "/SBEI"}],
        waiting: 0,
        config_version: 1,
    }

    it("parses only whole numbers", () => {
        expect(parseSignatures("3")).toBe(3)
        expect(parseSignatures(" 2 ")).toBe(2)
        expect(parseSignatures("")).toBeNull()
        expect(parseSignatures("1.5")).toBeNull()
        expect(parseSignatures("two")).toBeNull()
    })

    it("needs at least one signature", () => {
        for (const value of [null, 0, -1]) {
            expect(checkSignatures(value, capacity).problem).toBe(SignaturesProblem.AtLeastOne)
        }
    })

    it("refuses more signatures than the Post with most signers can give", () => {
        expect(checkSignatures(4, capacity)).toMatchObject({
            problem: SignaturesProblem.TooMany,
            max: 3,
        })
        // The largest Post can give exactly the maximum.
        expect(checkSignatures(3, capacity).problem).toBeNull()
    })

    it("names the Posts short of signers, grouped by how many they have", () => {
        expect(checkSignatures(3, capacity).shortPosts).toEqual([
            {signers: 1, posts: ["Rome PE"]},
            {signers: 2, posts: ["Wellington PE", "Dili PE"]},
        ])
        expect(checkSignatures(2, capacity).shortPosts).toEqual([{signers: 1, posts: ["Rome PE"]}])
        expect(checkSignatures(1, capacity)).toEqual({
            problem: null,
            max: 3,
            shortPosts: [],
            requesterShort: [],
            minSigners: 1,
        })
    })

    it("caps the number at the contract's range", () => {
        expect(checkSignatures(MAX_SIGNATURES, undefined).problem).toBeNull()
        expect(checkSignatures(MAX_SIGNATURES + 1, undefined)).toMatchObject({
            problem: SignaturesProblem.OutOfRange,
            max: MAX_SIGNATURES,
        })
    })

    it("names the Posts that need the person who starts it, when they can't sign", () => {
        // Madrid has 3, so 2 without the requester: short of 3. The others are short anyway.
        expect(checkSignatures(3, capacity, {requesterExcluded: true})).toMatchObject({
            shortPosts: [
                {signers: 1, posts: ["Rome PE"]},
                {signers: 2, posts: ["Wellington PE", "Dili PE"]},
            ],
            requesterShort: [{signers: 2, posts: ["Madrid PE"]}],
        })
        expect(checkSignatures(2, capacity, {requesterExcluded: true}).requesterShort).toEqual([
            {signers: 1, posts: ["Wellington PE", "Dili PE"]},
        ])
        expect(checkSignatures(2, capacity).requesterShort).toEqual([])
    })

    it("leaves the number to the server when the roles change", () => {
        // The capacity is of the saved roles; the server checks the new ones.
        expect(checkSignatures(4, capacity, {rolesChanged: true})).toEqual({
            problem: null,
            max: null,
            shortPosts: [],
            requesterShort: [],
            minSigners: null,
        })
        expect(checkSignatures(0, capacity, {rolesChanged: true}).problem).toBe(
            SignaturesProblem.AtLeastOne
        )
    })

    it("uses the maximum when the action has no Posts, and nothing without a capacity", () => {
        expect(
            checkSignatures(2, {
                max: 2,
                posts: [],
                posts_short: [],
                posts_short_without_requester: [],
                roles: [],
                waiting: 0,
                config_version: 1,
            })
        ).toEqual({
            problem: null,
            max: 2,
            shortPosts: [],
            requesterShort: [],
            minSigners: 2,
        })
        expect(checkSignatures(9, undefined)).toEqual({
            problem: null,
            max: null,
            shortPosts: [],
            requesterShort: [],
            minSigners: null,
        })
    })
})

const EVENT = "event-1"

describe("rule drafts", () => {
    const rule: ISigningRule = {
        action: SigningAction.GenerateElectionReturns,
        requirement: SigningRequirement.Required,
        signatures: 3,
        requester_signing: RequesterSigning.Allowed,
        expires_minutes: 60,
        revision: 7,
    }
    const roles = ["sbei"]

    it("detects no change until a field or the set of roles changes", () => {
        const initial = draftOf(rule, roles)
        expect(draftChanged(initial, draftOf(rule, ["sbei"]))).toBe(false)
        expect(draftChanged(initial, {...initial, signatures: "2"})).toBe(true)
        expect(draftChanged(initial, {...initial, expiresMinutes: null})).toBe(true)
        expect(draftChanged(initial, {...initial, roles: ["sbei", "ofov"]})).toBe(true)
        expect(draftChanged({...initial, roles: ["a", "b"]}, {...initial, roles: ["b", "a"]})).toBe(
            false
        )
    })

    it("sends the rule with the expected revision and no roles when they didn't change", () => {
        const draft = {...draftOf(rule, roles), signatures: "2", expiresMinutes: null}
        expect(ruleInputOf(EVENT, rule, draft, roles)).toEqual({
            election_event_id: EVENT,
            action: SigningAction.GenerateElectionReturns,
            requirement: SigningRequirement.Required,
            signatures: 2,
            requester_signing: RequesterSigning.Allowed,
            expires_minutes: null,
            expected_revision: 7,
        })
    })

    it("sends no roles while the rule is off", () => {
        const draft = {
            ...draftOf(rule, roles),
            requirement: SigningRequirement.NotRequired,
            roles: ["ofov"],
        }
        expect(ruleInputOf(EVENT, rule, draft, roles).roles).toBeUndefined()
    })

    it("sends the roles gained and lost", () => {
        const draft = {...draftOf(rule, ["sbei", "chair"]), roles: ["chair", "ofov"]}
        expect(ruleInputOf(EVENT, rule, draft, ["sbei", "chair"]).roles).toEqual({
            add: ["ofov"],
            remove: ["sbei"],
        })
    })

    it("keeps the saved number when an off rule's number is not valid", () => {
        const draft = {
            ...draftOf(rule, roles),
            requirement: SigningRequirement.NotRequired,
            signatures: "",
        }
        expect(ruleInputOf(EVENT, rule, draft, roles)).toMatchObject({
            requirement: SigningRequirement.NotRequired,
            signatures: 3,
        })
    })

    it("switches trustee actions only on or off", () => {
        const trustee: ISigningRule = {
            action: SigningAction.ConfirmKeyShare,
            requirement: SigningRequirement.NotRequired,
            signatures: 1,
            requester_signing: RequesterSigning.NotAllowed,
            expires_minutes: 60,
            revision: 0,
        }
        const draft = {
            ...draftOf(trustee, ["trustee"]),
            requirement: SigningRequirement.Required,
            signatures: "5",
            requesterSigning: RequesterSigning.NotAllowed,
        }
        expect(ruleInputOf(EVENT, trustee, draft, ["trustee"])).toEqual({
            election_event_id: EVENT,
            action: SigningAction.ConfirmKeyShare,
            requirement: SigningRequirement.Required,
            signatures: 1,
            requester_signing: RequesterSigning.Allowed,
            expires_minutes: 60,
            expected_revision: 0,
        })
    })
})

describe("certificates", () => {
    const now = new Date("2028-05-08T10:00:00Z")
    const certificate = (
        status: StaffCertificateStatus,
        not_after: string,
        extra: Partial<{username: string; subject: string; election_id: string | null}> = {}
    ) => ({
        id: `${status}-${not_after}`,
        username: "sbei-madrid-1",
        subject: "CN=MARIA L. SANTOS,O=Example",
        issuer: "CN=Example Individual CA",
        fingerprint_sha256: "f7080f24aa55fe78",
        election_id: "madrid",
        status,
        not_after,
        ...extra,
    })

    it("marks certificates expiring within 30 days", () => {
        expect(
            certificateStatus(certificate(StaffCertificateStatus.Active, "2029-01-01"), now)
        ).toBe(CertificateDisplayStatus.Active)
        expect(
            certificateStatus(
                certificate(StaffCertificateStatus.Active, "2028-06-07T09:59:59Z"),
                now
            )
        ).toBe(CertificateDisplayStatus.ExpiresSoon)
        // 30 days and a second away is not soon yet.
        expect(
            certificateStatus(
                certificate(StaffCertificateStatus.Active, "2028-06-07T10:00:01Z"),
                now
            )
        ).toBe(CertificateDisplayStatus.Active)
        expect(
            certificateStatus(
                certificate(StaffCertificateStatus.Active, "2028-05-08T09:00:00Z"),
                now
            )
        ).toBe(CertificateDisplayStatus.Expired)
        expect(
            certificateStatus(certificate(StaffCertificateStatus.Revoked, "2028-05-20"), now)
        ).toBe(CertificateDisplayStatus.Revoked)
    })

    it("filters by status and by person, certificate, issuer or Post", () => {
        const rows = [
            certificate(StaffCertificateStatus.Active, "2029-01-01"),
            certificate(StaffCertificateStatus.Revoked, "2029-01-01", {
                username: "sbei-wellington-2",
                subject: "CN=TERESA M. LIM",
                election_id: "wellington",
            }),
            certificate(StaffCertificateStatus.Active, "2028-05-20", {
                username: "ofov.aquino",
                subject: "CN=LIZA M. AQUINO",
                election_id: null,
            }),
        ]
        const postName = (id: string | null) =>
            id === "madrid" ? "Madrid PE" : id === "wellington" ? "Wellington PE" : "All"
        const ids = (search: string, status: CertificateDisplayStatus | null) =>
            filterCertificates(rows, {search, status}, now, postName).map((row) => row.username)
        expect(ids("", null)).toEqual(["sbei-madrid-1", "sbei-wellington-2", "ofov.aquino"])
        expect(ids("", CertificateDisplayStatus.Revoked)).toEqual(["sbei-wellington-2"])
        expect(ids("", CertificateDisplayStatus.ExpiresSoon)).toEqual(["ofov.aquino"])
        expect(ids("teresa", null)).toEqual(["sbei-wellington-2"])
        expect(ids("wellington pe", null)).toEqual(["sbei-wellington-2"])
        expect(ids("F7080F", null)).toHaveLength(3)
        expect(ids("individual ca", CertificateDisplayStatus.Active)).toEqual(["sbei-madrid-1"])
    })

    it("reads the common name of a distinguished name", () => {
        expect(commonName("CN=MARIA L. SANTOS,O=Example,C=PH")).toBe("MARIA L. SANTOS")
        expect(commonName("C=PH, O=Example, CN=Example Root CA")).toBe("Example Root CA")
        expect(commonName("/C=PH/O=Example/CN=Staff CA")).toBe("Staff CA")
        expect(commonName("O=No common name")).toBe("O=No common name")
    })

    it("shortens a fingerprint to its first four and last two bytes", () => {
        expect(shortFingerprint("f7080f24aa55bb66cc77fe78")).toBe("F7:08:0F:24:…:FE:78")
        expect(shortFingerprint("F7:08:0F:24:AA:55:BB:66:CC:77:FE:78")).toBe("F7:08:0F:24:…:FE:78")
        expect(shortFingerprint("a1b2c3")).toBe("A1:B2:C3")
        expect(shortFingerprint(null)).toBe("")
    })

    it("sends an issuer file as PEM text, or its DER bytes as base64", () => {
        const pem = "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n"
        expect(issuerUpload(new TextEncoder().encode(pem))).toEqual({pem, der_base64: null})
        expect(issuerUpload(Uint8Array.from([0x30, 0x82, 0x01]))).toEqual({
            pem: null,
            der_base64: "MIIB",
        })
    })

    it("imports PEM text as it is and wraps DER bytes as PEM", () => {
        const pem = "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n"
        expect(toPem(new TextEncoder().encode(pem))).toBe(pem)
        // OpenSSL's older header names the same content.
        expect(
            toPem(
                new TextEncoder().encode(
                    "-----BEGIN X509 CERTIFICATE-----\nMIIB\n-----END X509 CERTIFICATE-----\n"
                )
            )
        ).toBe(pem)
        // 49 bytes: base64 of 68 characters, wrapped at 64.
        const der = Uint8Array.from({length: 49}, (_, index) => index)
        expect(toPem(der)).toBe(
            "-----BEGIN CERTIFICATE-----\n" +
                "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4v\n" +
                "MA==\n" +
                "-----END CERTIFICATE-----\n"
        )
    })
})

describe("requests", () => {
    const request = (
        id: string,
        status: SigningRequestStatus,
        approvals: ISigningRequestListRow["approvals"] = []
    ): ISigningRequestListRow => ({
        id,
        action: SigningAction.CloseVoting,
        election_id: null,
        area_id: null,
        code: "0C7E-2B19",
        required: 2,
        status,
        cancel_reason: null,
        requested_by_username: "sbei-wellington-1",
        created_at: "2028-05-08T11:02:00Z",
        expires_at: null,
        approvals,
    })

    const now = new Date("2028-05-08T12:00:00Z")

    it("shows a waiting request past its expiry as expired", () => {
        const waiting = (expires_at: string | null) => ({
            ...request("a", SigningRequestStatus.Waiting),
            expires_at,
        })
        expect(requestStatus(waiting("2028-05-08T11:59:59Z"), now)).toBe(
            SigningRequestStatus.Expired
        )
        expect(requestStatus(waiting("2028-05-08T12:00:01Z"), now)).toBe(
            SigningRequestStatus.Waiting
        )
        expect(requestStatus(waiting(null), now)).toBe(SigningRequestStatus.Waiting)
        expect(
            requestStatus(
                {
                    ...request("b", SigningRequestStatus.Executed),
                    expires_at: "2020-01-01T00:00:00Z",
                },
                now
            )
        ).toBe(SigningRequestStatus.Executed)
    })

    it("filters by the status shown", () => {
        const rows = [
            request("a", SigningRequestStatus.Waiting),
            request("b", SigningRequestStatus.Executed),
            request("c", SigningRequestStatus.Waiting),
            {...request("d", SigningRequestStatus.Waiting), expires_at: "2028-05-08T11:00:00Z"},
        ]
        const ids = (status: SigningRequestStatus | null) =>
            filterRequests(rows, status, now).map(({id}) => id)
        expect(ids(null)).toEqual(["a", "b", "c", "d"])
        expect(ids(SigningRequestStatus.Waiting)).toEqual(["a", "c"])
        expect(ids(SigningRequestStatus.Expired)).toEqual(["d"])
    })

    it("finds the last signature", () => {
        expect(lastSignature(request("a", SigningRequestStatus.Waiting))).toBeNull()
        expect(
            lastSignature(
                request("b", SigningRequestStatus.Executed, [
                    {id: "1", username: "maria", signed_at: "2028-05-08T11:03:00Z"},
                    {id: "2", username: "jose", signed_at: "2028-05-08T11:05:00Z"},
                    {id: "3", username: "ana", signed_at: "2028-05-08T11:04:00Z"},
                ])
            )?.username
        ).toBe("jose")
    })
})

describe("identities", () => {
    // The DER of the PEM below is these three bytes; its SHA-256 is well known for "abc".
    const PEM = "-----BEGIN CERTIFICATE-----\nYWJj\n-----END CERTIFICATE-----\n"
    const ABC_SHA256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

    it("reads the first certificate's DER and fingerprint", async () => {
        expect(Array.from(pemToDer(`${PEM}${PEM.replace("YWJj", "ZGVm")}`) ?? [])).toEqual([
            0x61, 0x62, 0x63,
        ])
        expect(pemToDer("not a certificate")).toBeNull()
        expect(await certificateFingerprint(PEM)).toBe(ABC_SHA256)
    })

    it("links to the account that holds the certificate, or to the first one it links", () => {
        const row = (
            user_id: string,
            linked_to: string | null,
            status: StaffCertificateStatus
        ) => ({
            user_id,
            linked_to,
            status,
            fingerprint_sha256: ABC_SHA256,
        })
        expect(linkTarget([row("maria", null, StaffCertificateStatus.Active)], ABC_SHA256)).toBe(
            "maria"
        )
        expect(
            linkTarget(
                [
                    row("old", null, StaffCertificateStatus.Revoked),
                    row("trustee-2", "sbei-2", StaffCertificateStatus.Active),
                ],
                ABC_SHA256
            )
        ).toBe("sbei-2")
        expect(linkTarget([row("x", null, StaffCertificateStatus.Active)], "other")).toBeNull()
    })

    it("names a person by display name, falling back to the username", () => {
        expect(personName("Maria L. Santos", "sbei-madrid-1")).toBe("Maria L. Santos")
        expect(personName(null, "sbei-madrid-1")).toBe("sbei-madrid-1")
        expect(personName(" ", "sbei-madrid-1")).toBe("sbei-madrid-1")
    })
})

describe("server errors", () => {
    const graphQLError = (extensions: Record<string, unknown>) => ({
        graphQLErrors: [{message: "refused", extensions}],
    })

    it("classifies by extensions.code", () => {
        expect(signingError(graphQLError({code: "forbidden"}))).toMatchObject({
            code: SigningErrorCode.Forbidden,
            check: null,
        })
        expect(signingError(graphQLError({code: "invalid"})).code).toBe(SigningErrorCode.Invalid)
        expect(signingError(graphQLError({code: "conflict"})).code).toBe(SigningErrorCode.Conflict)
        expect(signingError(graphQLError({code: "stale-revision"})).code).toBe(
            SigningErrorCode.Conflict
        )
        // A registered-to-other refusal names the account holding the key or holder.
        expect(
            signingError(
                graphQLError({
                    code: "signing-refused",
                    check: "registered-to-other",
                    user_id: "maria",
                    display_name: "Maria Santos",
                })
            )
        ).toEqual({
            code: SigningErrorCode.Refused,
            check: "registered-to-other",
            holder: {userId: "maria", name: "Maria Santos"},
        })
    })

    it("falls back to the HTTP status Hasura forwards", () => {
        const status = (value: number) =>
            signingError(graphQLError({internal: {response: {status: value}}})).code
        expect([403, 400, 409, 404, 422, 500].map(status)).toEqual([
            SigningErrorCode.Forbidden,
            SigningErrorCode.Invalid,
            SigningErrorCode.Conflict,
            SigningErrorCode.NotFound,
            SigningErrorCode.Refused,
            SigningErrorCode.Other,
        ])
        expect(signingError(new Error("network")).code).toBe(SigningErrorCode.Other)
    })

    it("names each error, and a refused rule edit after lockdown as the lockdown", () => {
        expect(errorKey(SigningErrorCode.Forbidden, false)).toBe("signing.errors.forbidden")
        expect(errorKey(SigningErrorCode.Invalid, false)).toBe("signing.errors.invalid")
        expect(errorKey(SigningErrorCode.Invalid, true)).toBe("signing.errors.lockedDown")
        expect(errorKey(signingError(graphQLError({code: "locked-down"})).code, false)).toBe(
            "signing.errors.lockedDown"
        )
        expect(errorKey(SigningErrorCode.Conflict, false)).toBe("signing.errors.conflict")
        expect(errorKey(SigningErrorCode.NotFound, false)).toBe("signing.errors.notFound")
        expect(errorKey(SigningErrorCode.Other, false)).toBeNull()
    })
})
