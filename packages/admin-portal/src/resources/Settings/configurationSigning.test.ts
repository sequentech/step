// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EConfigurationSigningPolicy,
    EConfigurationSigningState,
    EFinalSignerSeparation,
    EPackageFormat,
    SUGGESTED_FORM,
    readPolicyFile,
    certificateCount,
    readCheck,
    readSetting,
    toSetting,
    validApprovals,
} from "./configurationSigning"

describe("readSetting", () => {
    it("reads an installed setting with the importer's defaults for absent fields", () => {
        expect(
            readSetting({configuration_signing: {policy: "required", required_approvals: 2}})
        ).toEqual({
            state: EConfigurationSigningState.REQUIRED,
            form: {
                policy: EConfigurationSigningPolicy.REQUIRED,
                packageRoots: "",
                staffRoots: "",
                requiredApprovals: 2,
                // A legacy setting: no format-2 field is invented.
                signers: [],
            },
        })
        expect(readSetting({configuration_signing: {}}).state).toBe(
            EConfigurationSigningState.OPTIONAL
        )
    })

    it("tells no setting apart from one the importer can't read", () => {
        expect(readSetting({theme: "dark"}).state).toBe(EConfigurationSigningState.NOT_CONFIGURED)
        expect(readSetting(null).state).toBe(EConfigurationSigningState.NOT_CONFIGURED)
        for (const value of [
            null,
            "required",
            {policy: "sometimes"},
            {policy: "required", required_approvals: -1},
            {policy: "required", required_approvals: 1.5},
            {policy: "required", package_roots: 3},
        ]) {
            expect(readSetting({configuration_signing: value})).toEqual({
                state: EConfigurationSigningState.UNREADABLE,
                form: null,
            })
        }
    })

    // The importer's serde defaults apply to absent fields; an explicit null
    // fails to parse there, so it must not show as an active policy here.
    it("shows an explicit null field as unreadable, not as a default", () => {
        for (const key of ["policy", "required_approvals", "package_roots", "staff_roots"]) {
            const value: Record<string, unknown> = {policy: "required", required_approvals: 2}
            value[key] = null
            expect(readSetting({configuration_signing: value}).state).toBe(
                EConfigurationSigningState.UNREADABLE
            )
        }
    })
})

describe("the form", () => {
    it("suggests required signatures, two approvals and format 2, with no roots or signers filled in", () => {
        expect(SUGGESTED_FORM).toEqual({
            policy: EConfigurationSigningPolicy.REQUIRED,
            packageRoots: "",
            staffRoots: "",
            requiredApprovals: 2,
            minimumFormat: EPackageFormat.V2,
            separation: EFinalSignerSeparation.SEPARATE,
            signers: [],
        })
    })

    it("submits the platform's field names only", () => {
        expect(toSetting({...SUGGESTED_FORM, packageRoots: "P", staffRoots: "S"})).toEqual({
            policy: "required",
            package_roots: "P",
            staff_roots: "S",
            required_approvals: 2,
            minimum_package_format: "v2",
            final_signer_separation: "separate",
        })
        // A legacy form stays a legacy setting: no format-2 field is sent.
        const legacy = readSetting({configuration_signing: {policy: "required", required_approvals: 2}}).form!
        expect(toSetting(legacy)).toEqual({
            policy: "required",
            package_roots: "",
            staff_roots: "",
            required_approvals: 2,
        })
    })

    it("accepts whole approval counts up to the platform's limit", () => {
        expect([0, 2, 16].map(validApprovals)).toEqual([true, true, true])
        expect([-1, 17, 1.5, NaN].map(validApprovals)).toEqual([false, false, false, false])
    })

    it("counts certificates", () => {
        const block = "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n"
        expect(certificateCount("")).toBe(0)
        expect(certificateCount(block + block)).toBe(2)
    })
})

describe("readCheck", () => {
    it("reads anchors and problems, leaving out malformed entries", () => {
        expect(
            readCheck({
                anchors: [
                    {
                        setting: "package_roots",
                        subject: "CN=Root",
                        not_before: "2026-01-01T00:00:00Z",
                        not_after: "2036-01-01T00:00:00Z",
                        fingerprint_sha256: "ab",
                    },
                    {setting: "package_roots"},
                ],
                report: {
                    problems: [
                        {severity: "warning", path: "required_approvals", message: "m"},
                        {path: "x"},
                    ],
                },
            })
        ).toEqual({
            anchors: [
                {
                    setting: "package_roots",
                    subject: "CN=Root",
                    not_before: "2026-01-01T00:00:00Z",
                    not_after: "2036-01-01T00:00:00Z",
                    fingerprint_sha256: "ab",
                },
            ],
            problems: [{severity: "warning", path: "required_approvals", message: "m"}],
        })
        expect(readCheck(null)).toEqual({anchors: [], problems: []})
    })
})

const SIGNERS = [
    {id: "approver-1", name: "DEMO approver-1", roles: ["approver"], keys: [{spki_sha256: "a".repeat(64)}]},
    {id: "custodian", name: "DEMO custodian", roles: ["final-signer"], keys: [{spki_sha256: "c".repeat(64)}]},
]

describe("format 2", () => {
    it("reads enrolled signers, the minimum format and separation", () => {
        const read = readSetting({
            configuration_signing: {
                policy: "required",
                required_approvals: 1,
                minimum_package_format: "v2",
                final_signer_separation: "separate",
                signers: SIGNERS,
            },
        })
        expect(read.form?.minimumFormat).toBe(EPackageFormat.V2)
        expect(read.form?.separation).toBe(EFinalSignerSeparation.SEPARATE)
        expect(read.form?.signers.map((s) => s.id)).toEqual(["approver-1", "custodian"])
        expect(toSetting(read.form!)).toMatchObject({signers: SIGNERS, minimum_package_format: "v2"})
    })

    it("reads malformed format-2 fields as a setting the importer can't read", () => {
        for (const bad of [
            {minimum_package_format: "v3"},
            {final_signer_separation: "sometimes"},
            {signers: "everyone"},
            {signers: [{id: "x", name: "x", roles: ["admin"], keys: []}]},
            {signers: [{id: "x", name: "x", roles: ["approver"], keys: [{spki_sha256: 1}]}]},
        ]) {
            expect(readSetting({configuration_signing: {policy: "required", ...bad}}).state).toBe(
                EConfigurationSigningState.UNREADABLE
            )
        }
    })

    it("loads the installer's policy file for this tenant only, with its minimum stated", () => {
        const file = (tenant: string, signing: Record<string, unknown>) => JSON.stringify({tenant_id: tenant, signing})
        const signing = {policy: "required", required_approvals: 1, minimum_package_format: "v2", signers: SIGNERS}
        expect(readPolicyFile(file("t-1", signing), "t-1").form?.signers).toHaveLength(2)
        expect(readPolicyFile(file("t-2", signing), "t-1").error).toBe("other-tenant")
        expect(readPolicyFile(file("t-1", {...signing, minimum_package_format: undefined}), "t-1").error).toBe(
            "no-minimum"
        )
        expect(readPolicyFile("{nope", "t-1").error).toBe("not-json")
        expect(readPolicyFile(JSON.stringify([1]), "t-1").error).toBe("not-a-policy")
    })

    it("refuses a policy file that is not exactly {tenant_id, signing} for this tenant", () => {
        const signing = {policy: "required", required_approvals: 1, minimum_package_format: "v2", signers: SIGNERS}
        // The bare signing object (no tenant to match) is refused.
        expect(readPolicyFile(JSON.stringify(signing), "t-1").error).toBe("not-a-policy")
        for (const tenant of [undefined, null, 7, ""]) {
            expect(readPolicyFile(JSON.stringify({tenant_id: tenant, signing}), "t-1").error).toBe("no-tenant")
        }
        const misspelled = [
            {tenant_id: "t-1", signing, comment: "x"},
            {tenant_id: "t-1", signing: {...signing, final_signer_seperation: "allowed"}},
            {tenant_id: "t-1", signing: {...signing, signers: [{...SIGNERS[0], role: ["approver"]}, SIGNERS[1]]}},
            {
                tenant_id: "t-1",
                signing: {...signing, signers: [{...SIGNERS[0], keys: [{spki_sha256: "a".repeat(64), lable: "x"}]}, SIGNERS[1]]},
            },
        ]
        for (const bad of misspelled) {
            expect(readPolicyFile(JSON.stringify(bad), "t-1").error).toBe("unknown-field")
        }
        // A policy the platform would refuse is refused here too.
        expect(
            readPolicyFile(JSON.stringify({tenant_id: "t-1", signing: {...signing, policy: "optional"}}), "t-1").error
        ).toBe("not-a-policy")
    })

    it("reads a format-2 record the importer refuses as unreadable, never OPTIONAL or REQUIRED", () => {
        const base = {policy: "required", required_approvals: 1, minimum_package_format: "v2", signers: SIGNERS}
        const approver = (id: string, pin: string) => ({id, name: id, roles: ["approver"], keys: [{spki_sha256: pin}]})
        const cases: Record<string, unknown>[] = [
            // Signers (or separation) without a minimum format.
            {policy: "required", required_approvals: 1, signers: SIGNERS},
            {policy: "required", final_signer_separation: "separate"},
            // Optional with format 2.
            {...base, policy: "optional"},
            // Format 2 with nobody enrolled, or no final signer.
            {...base, signers: []},
            {...base, signers: [SIGNERS[0]]},
            // Too few approvers for the count required.
            {...base, required_approvals: 2},
            // One key shared by two people; a bad pin; a bad or repeated ID; no name, role or key.
            {...base, signers: [...SIGNERS, approver("approver-2", "a".repeat(64))]},
            {...base, signers: [...SIGNERS, approver("approver-2", "A".repeat(64))]},
            {...base, signers: [...SIGNERS, approver("Approver 2", "b".repeat(64))]},
            {...base, signers: [...SIGNERS, approver("approver-1", "b".repeat(64))]},
            {...base, signers: [...SIGNERS, {...approver("approver-2", "b".repeat(64)), name: " "}]},
            {...base, signers: [...SIGNERS, {...approver("approver-2", "b".repeat(64)), roles: []}]},
            {...base, signers: [...SIGNERS, {...approver("approver-2", "b".repeat(64)), keys: []}]},
            // Separate, and the only final signer is the only approver.
            {
                ...base,
                final_signer_separation: "separate",
                signers: [{...approver("both", "a".repeat(64)), roles: ["approver", "final-signer"]}],
            },
        ]
        for (const record of cases) {
            expect([JSON.stringify(record), readSetting({configuration_signing: record}).state]).toEqual([
                JSON.stringify(record),
                EConfigurationSigningState.UNREADABLE,
            ])
        }
        // The same dual-role person is fine where separation is allowed.
        expect(
            readSetting({
                configuration_signing: {
                    ...base,
                    final_signer_separation: "allowed",
                    signers: [{...approver("both", "a".repeat(64)), roles: ["approver", "final-signer"]}],
                },
            }).state
        ).toBe(EConfigurationSigningState.REQUIRED)
        // A legacy record (no format-2 field) reads as before.
        expect(readSetting({configuration_signing: {policy: "optional"}}).state).toBe(
            EConfigurationSigningState.OPTIONAL
        )
    })

    it("shows the policy digest the platform computed", () => {
        expect(readCheck({anchors: [], report: {problems: []}, policy_sha256: "d".repeat(64)}).policySha256).toBe(
            "d".repeat(64)
        )
        expect(readCheck({anchors: [], report: {problems: []}}).policySha256).toBeUndefined()
    })
})
