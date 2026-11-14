// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The tenant's trust in configuration packages, settings.configuration_signing:
// read as the platform reads it (sequent-core ConfigurationSigning), edited as
// a form and submitted to the set_configuration_signing action, which checks it
// and is the only way to save it.

export enum EConfigurationSigningPolicy {
    /** A signed package is checked; an unsigned file imports as before. */
    OPTIONAL = "optional",
    /** Only a signed package that passes every check imports. */
    REQUIRED = "required",
}

/** The oldest package format a tenant imports (format 2 binds approvals to enrolled signers). */
export enum EPackageFormat {
    V1 = "v1",
    V2 = "v2",
}

/** Whether the final signer may also have approved (format 2). */
export enum EFinalSignerSeparation {
    SEPARATE = "separate",
    ALLOWED = "allowed",
}

/** One enrolled signer (format 2): a stable ID, its roles and its keys' SPKI pins. */
export interface EnrolledSigner {
    id: string
    name: string
    roles: ("approver" | "final-signer")[]
    keys: {spki_sha256: string; label?: string}[]
}

export interface ConfigurationSigningForm {
    policy: EConfigurationSigningPolicy
    packageRoots: string
    staffRoots: string
    requiredApprovals: number
    /** Absent only for a legacy setting; a new policy states it (default v2). */
    minimumFormat?: EPackageFormat
    separation?: EFinalSignerSeparation
    signers: EnrolledSigner[]
}

/** The most approvals a tenant may require (sequent-core MAX_REQUIRED_APPROVALS). */
export const MAX_REQUIRED_APPROVALS = 16

/**
 * What a tenant with no setting is offered: signatures required, from two
 * distinct approvers, as the Election Architect's service asks by default. The
 * roots are the tenant's own and are never filled in for it.
 */
export const SUGGESTED_FORM: ConfigurationSigningForm = {
    policy: EConfigurationSigningPolicy.REQUIRED,
    packageRoots: "",
    staffRoots: "",
    requiredApprovals: 2,
    // New policies are format 2: two enrolled approvers and a separate final
    // signer, loaded from the installer's signing policy file.
    minimumFormat: EPackageFormat.V2,
    separation: EFinalSignerSeparation.SEPARATE,
    signers: [],
}

const isSigner = (value: unknown): value is EnrolledSigner =>
    isRecord(value) &&
    typeof value.id === "string" &&
    typeof value.name === "string" &&
    Array.isArray(value.roles) &&
    value.roles.every((role) => role === "approver" || role === "final-signer") &&
    Array.isArray(value.keys) &&
    value.keys.every(
        (key) =>
            isRecord(key) &&
            typeof key.spki_sha256 === "string" &&
            (key.label === undefined || typeof key.label === "string")
    )

const validId = (id: string) => /^[a-z0-9._-]{1,64}$/.test(id)
const validPin = (pin: string) => /^[0-9a-f]{64}$/.test(pin)

/**
 * The problems sequent-core's `check_policy` finds in a record with format-2
 * fields (their IDs), mirrored so the page never shows OPTIONAL or REQUIRED
 * for a record the importer refuses as `invalid-stored-policy`. A record with
 * no format-2 field is a legacy one and has none.
 */
export const policyProblems = (form: ConfigurationSigningForm): string[] => {
    const {signers, minimumFormat, separation} = form
    if (minimumFormat === undefined && separation === undefined && signers.length === 0) return []
    const problems: string[] = []
    const ids = new Set<string>()
    const owners = new Map<string, string>()
    for (const signer of signers) {
        if (!validId(signer.id)) problems.push("signer-id")
        else if (ids.has(signer.id)) problems.push("duplicate-signer")
        else ids.add(signer.id)
        if (signer.name.trim() === "") problems.push("signer-name")
        if (signer.roles.length === 0) problems.push("signer-roles")
        if (signer.keys.length === 0) problems.push("signer-keys")
        for (const key of signer.keys) {
            if (!validPin(key.spki_sha256)) {
                problems.push("key-pin")
                continue
            }
            const owner = owners.get(key.spki_sha256)
            if (owner === undefined) owners.set(key.spki_sha256, signer.id)
            else problems.push(owner === signer.id ? "duplicate-key" : "shared-key")
        }
    }
    if (minimumFormat === undefined) problems.push("minimum-format-required")
    const v2 = minimumFormat === EPackageFormat.V2
    if (v2 && form.policy === EConfigurationSigningPolicy.OPTIONAL) problems.push("optional-with-v2")
    if (v2 || signers.length > 0) {
        const required = form.requiredApprovals
        const approverIds = signers.filter((s) => s.roles.includes("approver")).map((s) => s.id)
        const finals = signers.filter((s) => s.roles.includes("final-signer"))
        if (
            (separation ?? EFinalSignerSeparation.SEPARATE) === EFinalSignerSeparation.SEPARATE &&
            finals.length > 0 &&
            approverIds.length >= required &&
            !finals.some((f) => approverIds.filter((id) => id !== f.id).length >= required)
        ) {
            problems.push("separation-unsatisfiable")
        }
        if (approverIds.length < required) problems.push("too-few-approvers")
        if (finals.length === 0) problems.push("no-final-signer")
    }
    if (v2 && signers.length === 0) problems.push("no-signers")
    return problems
}

/** How the tenant stands today, as the importer will treat a file. */
export enum EConfigurationSigningState {
    /** No setting: unsigned files import, signed ones are refused (no roots). */
    NOT_CONFIGURED = "not-configured",
    OPTIONAL = "optional",
    REQUIRED = "required",
    /** A setting the importer can't read: every import fails until it is fixed. */
    UNREADABLE = "unreadable",
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null && !Array.isArray(value)
}

/** The installed setting, read with the importer's defaults. */
export const readSetting = (
    settings: unknown
): {state: EConfigurationSigningState; form: ConfigurationSigningForm | null} => {
    if (!isRecord(settings) || !("configuration_signing" in settings)) {
        return {state: EConfigurationSigningState.NOT_CONFIGURED, form: null}
    }
    const value = settings.configuration_signing
    if (!isRecord(value)) {
        return {state: EConfigurationSigningState.UNREADABLE, form: null}
    }
    // The importer's defaults apply to absent fields only: an explicit null is
    // a setting it can't read, and every import fails until it is fixed.
    const field = (key: string, fallback: unknown): unknown =>
        key in value ? value[key] : fallback
    const policy = field("policy", EConfigurationSigningPolicy.OPTIONAL)
    const approvals = field("required_approvals", 0)
    const roots = [field("package_roots", ""), field("staff_roots", "")]
    const minimum = field("minimum_package_format", undefined)
    const separation = field("final_signer_separation", undefined)
    const signers = field("signers", [])
    if (
        (minimum !== undefined && !Object.values(EPackageFormat).includes(minimum as EPackageFormat)) ||
        (separation !== undefined &&
            !Object.values(EFinalSignerSeparation).includes(separation as EFinalSignerSeparation)) ||
        !Array.isArray(signers) ||
        !signers.every(isSigner)
    ) {
        return {state: EConfigurationSigningState.UNREADABLE, form: null}
    }
    if (
        !Object.values(EConfigurationSigningPolicy).includes(
            policy as EConfigurationSigningPolicy
        ) ||
        typeof approvals !== "number" ||
        !Number.isInteger(approvals) ||
        approvals < 0 ||
        roots.some((root) => typeof root !== "string")
    ) {
        return {state: EConfigurationSigningState.UNREADABLE, form: null}
    }
    const form: ConfigurationSigningForm = {
        policy: policy as EConfigurationSigningPolicy,
        packageRoots: roots[0] as string,
        staffRoots: roots[1] as string,
        requiredApprovals: approvals,
        ...(minimum === undefined ? {} : {minimumFormat: minimum as EPackageFormat}),
        ...(separation === undefined ? {} : {separation: separation as EFinalSignerSeparation}),
        signers: signers as EnrolledSigner[],
    }
    // A format-2 record the importer refuses (invalid-stored-policy) refuses
    // every file, signed or not: it is unreadable here too.
    if (policyProblems(form).length > 0) {
        return {state: EConfigurationSigningState.UNREADABLE, form: null}
    }
    return {
        state:
            form.policy === EConfigurationSigningPolicy.REQUIRED
                ? EConfigurationSigningState.REQUIRED
                : EConfigurationSigningState.OPTIONAL,
        form,
    }
}

/** The action's input: the platform's field names, nothing else. */
export const toSetting = (form: ConfigurationSigningForm) => ({
    policy: form.policy,
    package_roots: form.packageRoots,
    staff_roots: form.staffRoots,
    required_approvals: form.requiredApprovals,
    // Format-2 fields only when stated: a legacy form stays a legacy setting.
    ...(form.minimumFormat === undefined ? {} : {minimum_package_format: form.minimumFormat}),
    ...(form.separation === undefined ? {} : {final_signer_separation: form.separation}),
    ...(form.signers.length === 0 ? {} : {signers: form.signers}),
})

const SIGNING_FIELDS = [
    "policy",
    "package_roots",
    "staff_roots",
    "required_approvals",
    "minimum_package_format",
    "final_signer_separation",
    "signers",
]

/** Fields not in `allowed`, so a misspelled one is refused, not dropped. */
const unknownFields = (value: Record<string, unknown>, allowed: string[]) =>
    Object.keys(value).some((key) => !allowed.includes(key))

/**
 * The installer's signing policy file (`signing-policy.json`, exactly
 * `{tenant_id, signing}`, the same file the standalone Architect installs) as
 * this form. Fails closed: an unknown or misspelled field at any level, a
 * missing or non-string tenant, another tenant's file or a policy the
 * platform would refuse is an error, and the caller keeps what it had.
 */
export const readPolicyFile = (
    text: string,
    tenantId: string
): {form?: ConfigurationSigningForm; error?: string} => {
    let value: unknown
    try {
        value = JSON.parse(text)
    } catch {
        return {error: "not-json"}
    }
    if (!isRecord(value) || !isRecord(value.signing)) return {error: "not-a-policy"}
    if (unknownFields(value, ["tenant_id", "signing"])) return {error: "unknown-field"}
    if (typeof value.tenant_id !== "string" || value.tenant_id === "") return {error: "no-tenant"}
    if (value.tenant_id !== tenantId) return {error: "other-tenant"}
    const signing = value.signing
    if (
        unknownFields(signing, SIGNING_FIELDS) ||
        (Array.isArray(signing.signers) &&
            signing.signers.some(
                (signer) =>
                    !isRecord(signer) ||
                    unknownFields(signer, ["id", "name", "roles", "keys"]) ||
                    (Array.isArray(signer.keys) &&
                        signer.keys.some(
                            (key) => !isRecord(key) || unknownFields(key, ["spki_sha256", "label"])
                        ))
            ))
    ) {
        return {error: "unknown-field"}
    }
    if (signing.minimum_package_format === undefined) return {error: "no-minimum"}
    const read = readSetting({configuration_signing: signing})
    if (read.form === null) return {error: "not-a-policy"}
    return {form: read.form}
}

/** An approval count the form can submit; anything else is refused before sending. */
export const validApprovals = (value: number): boolean =>
    Number.isInteger(value) && value >= 0 && value <= MAX_REQUIRED_APPROVALS

/** How many certificates a PEM text holds, as a quick hint while typing. */
export const certificateCount = (pem: string): number =>
    (pem.match(/-----BEGIN CERTIFICATE-----/g) ?? []).length

export interface TrustAnchor {
    setting: "package_roots" | "staff_roots" | string
    subject: string
    not_before: string
    not_after: string
    fingerprint_sha256: string
}

export interface TrustProblem {
    severity: "error" | "warning" | string
    id?: string | null
    path: string
    message: string
}

export interface TrustCheck {
    anchors: TrustAnchor[]
    problems: TrustProblem[]
    /** Format 2: the policy digest approvals bind to, as the platform computed it. */
    policySha256?: string
}

/** The action's check, read defensively: anything malformed is left out. */
export const readCheck = (check: unknown): TrustCheck => {
    if (!isRecord(check)) return {anchors: [], problems: []}
    const anchors = Array.isArray(check.anchors)
        ? check.anchors.filter(
              (anchor): anchor is TrustAnchor =>
                  isRecord(anchor) &&
                  typeof anchor.subject === "string" &&
                  typeof anchor.setting === "string" &&
                  typeof anchor.not_after === "string" &&
                  typeof anchor.fingerprint_sha256 === "string"
          )
        : []
    const report = isRecord(check.report) ? check.report : {}
    const problems = Array.isArray(report.problems)
        ? report.problems.filter(
              (problem): problem is TrustProblem =>
                  isRecord(problem) &&
                  typeof problem.message === "string" &&
                  typeof problem.severity === "string"
          )
        : []
    const policySha256 = typeof check.policy_sha256 === "string" ? check.policy_sha256 : undefined
    return {anchors, problems, ...(policySha256 === undefined ? {} : {policySha256})}
}
