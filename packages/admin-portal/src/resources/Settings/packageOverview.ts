// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Presentation model for the configuration package page: the section rows in
// apply order, the run's stages, the receipt of what is applied on the tenant
// and the header's actions. Read from the platform's records only; nothing
// here admits an action. The page keeps its own gates (mayApply, trust,
// confirmation) and never feeds these values back into them.

import {IPermissions} from "@/types/keycloak"
import type {PackageImportPhase} from "./PackageImportWorkflow"
import {
    PACKAGE_PERMISSIONS,
    PACKAGE_SECTIONS,
    proposesChange,
    readApplication,
    type ApplicationView,
    type ApplyMode,
    type CensusRequirement,
    type PackageRequirements,
    type PackageSection,
    type SectionView,
    type ActiveSubmission,
} from "./configurationPackage"

export type SectionBadge =
    | "applied"
    | "unchanged"
    | "willChange"
    | "blocked"
    | "missing"
    | "notInPackage"
    | "failed"
    | "notConfirmed"
    | "applying"
    | "waiting"
    | "notStarted"
    | "notRun"

/** The badge for one section of the run being shown, from its recorded state. */
export const sectionBadge = (section: SectionView | undefined, running: boolean): SectionBadge => {
    if (!section) return "notRun"
    switch (section.state) {
        case "applied":
            return "applied"
        case "unchanged":
            return "unchanged"
        case "previewed":
            // The same predicate as the confirmation and the count, Messaging included.
            return proposesChange(section) ? "willChange" : "unchanged"
        case "blocked":
            return "blocked"
        case "missing":
            return "missing"
        case "not-included":
            return "notInPackage"
        case "failed":
            return "failed"
        case "submitting":
            // Begun and not confirmed: once the run has ended it is unsettled.
            return running ? "applying" : "notConfirmed"
        case "pending":
            return running ? "waiting" : "notStarted"
        default:
            // "unknown", or a state this portal does not know: never claim success.
            return "notConfirmed"
    }
}

export interface SectionRow {
    section: PackageSection
    badge: SectionBadge
    view?: SectionView
}

/** Every package section once, in the order the platform applies them. */
/**
 * Whether a run is a confirmed application: an apply record whose outcome is
 * applied on a task row that ended in SUCCESS. The header, the receipt, History
 * and the section badges all use this one rule.
 */
export const isConfirmedApply = (
    view: ApplicationView | null | undefined,
    executionStatus: string | null | undefined
): boolean => view?.mode === "apply" && view.outcome === "applied" && executionStatus === "SUCCESS"

/** A record that says applied on a task row that does not confirm it. */
export const isUnconfirmedApplied = (
    view: ApplicationView | null | undefined,
    executionStatus: string | null | undefined
): boolean => view?.outcome === "applied" && !isConfirmedApply(view, executionStatus)

/**
 * Every package section once, in the order the platform applies them. Pass the
 * run's task execution_status: when the record claims an applied run the task
 * row does not confirm, no section is shown as applied.
 */
export const orderedSectionBadges = (
    view: ApplicationView | null | undefined,
    running: boolean,
    executionStatus: string | null | undefined
): SectionRow[] => {
    const unconfirmed = isUnconfirmedApplied(view, executionStatus)
    return PACKAGE_SECTIONS.map((section) => {
        const found = view?.sections.find((s) => s.section === section)
        const badge = sectionBadge(found, running)
        return {
            section,
            badge: unconfirmed && badge === "applied" ? "notConfirmed" : badge,
            view: found,
        }
    })
}

export type RunStep = "package" | "preview" | "review" | "apply"
export type StageState =
    | "done"
    | "current"
    | "running"
    | "blocked"
    | "stale"
    | "attention"
    | "pending"

export interface RunStage {
    step: RunStep
    state: StageState
}

const STEPS: RunStep[] = ["package", "preview", "review", "apply"]

const stages = (...states: StageState[]): RunStage[] =>
    STEPS.map((step, index) => ({step, state: states[index] ?? "pending"}))

/** The current run's stages. No stage is done unless the phase says so. */
export const runStages = (phase: PackageImportPhase, mode?: ApplyMode): RunStage[] => {
    switch (phase) {
        case "choose":
            return stages("current")
        case "starting":
            return mode === "apply"
                ? stages("done", "done", "done", "running")
                : stages("done", "running")
        case "previewing":
            return stages("done", "running")
        case "previewBlocked":
            return stages("done", "blocked")
        case "previewAgain":
            return stages("done", "stale")
        case "review":
            return stages("done", "done", "current")
        case "applying":
            return stages("done", "done", "done", "running")
        case "complete":
            return stages("done", "done", "done", "done")
        case "attention":
            return mode === "apply"
                ? stages("done", "done", "done", "attention")
                : mode === "preview"
                  ? stages("done", "attention")
                  : stages("attention")
        default:
            // Trust missing or loading: no run can start, none is claimed.
            return stages()
    }
}

export type HeaderAction =
    | "upload"
    | "previewRemaining"
    | "openTrust"
    | "checkAgain"
    | "previewAgain"
    | "reviewAndApply"
    | "importFixed"
    | "changeRequirements"

/** Why import is blocked by trust: no active signer roster, or the check itself failed. */
export type TrustBlock = "missingRoster" | "checkFailed"

/** The header's actions. A blocked preview never offers Apply. */
export const headerActions = (
    phase: PackageImportPhase,
    trustBlock: TrustBlock = "checkFailed"
): {primary: HeaderAction; secondary?: HeaderAction} => {
    switch (phase) {
        case "trustBlocked":
            // The existing Signing trust path; import stays shown, disabled, with its reason.
            return trustBlock === "missingRoster"
                ? {primary: "openTrust", secondary: "upload"}
                : {primary: "upload", secondary: "checkAgain"}
        case "review":
            return {primary: "reviewAndApply", secondary: "upload"}
        case "previewBlocked":
            return {primary: "importFixed", secondary: "changeRequirements"}
        case "previewAgain":
            // The page falls back to Import package when it cannot preview the same document.
            return {primary: "previewAgain", secondary: "upload"}
        default:
            return {primary: "upload"}
    }
}

export interface HistoryRow {
    id: string
    created_at: string
    execution_status?: string | null
    executed_by_user?: string | null
    annotations?: unknown
}

export interface AppliedReceipt {
    taskId: string
    startedAt: string
    appliedBy?: string
    packageSha256?: string
}

/**
 * The newest run in the history that the platform recorded as applied.
 * undefined while the history has not answered; null when none of the rows
 * the history returned is an applied run (an older one may exist beyond it).
 */
export const lastAppliedReceipt = (
    rows: readonly HistoryRow[] | null | undefined
): AppliedReceipt | null | undefined => {
    if (!rows) return undefined
    for (const row of rows) {
        const view = readApplication(row.annotations)
        if (view && isConfirmedApply(view, row.execution_status)) {
            return {
                taskId: row.id,
                startedAt: row.created_at,
                ...(row.executed_by_user ? {appliedBy: row.executed_by_user} : {}),
                ...(typeof view.packageSha256 === "string" && view.packageSha256
                    ? {packageSha256: view.packageSha256}
                    : {}),
            }
        }
    }
    return null
}

/** The package permissions the operator does not hold on the tenant. */
export const missingPackagePermissions = (
    tenantId: string | null | undefined,
    isAuthorized: (checkSuperAdmin: boolean, tenantId: string | null, role: IPermissions) => boolean
): string[] =>
    tenantId
        ? PACKAGE_PERMISSIONS.filter((permission) => !isAuthorized(false, tenantId, permission))
        : []

/**
 * Sections a finished preview would change, in the order Apply would run them.
 * The confirmation's list, its proposed changes and the page's count all use proposesChange.
 */
export const changingSections = (view: ApplicationView | null | undefined): PackageSection[] =>
    PACKAGE_SECTIONS.filter((section) => {
        const found = view?.sections.find((s) => s.section === section)
        return !!found && proposesChange(found)
    })

export interface RunIdentity {
    packageSha256?: string
    /** The requirements the receiver bound to the run, when it recorded them. */
    requirements?: PackageRequirements
    /** The signer roster the receiver bound to the run, when it recorded it. */
    roster?: {sha256: string; generation: number; requiredApprovals: number}
}

const asRecord = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" ? (value as Record<string, unknown>) : {}

/**
 * What a run's saved record says it was bound to: the package digest, and the
 * requirements and signer roster when the receiver recorded them. A field the
 * run did not record, or recorded in a shape this portal does not accept, is
 * left out rather than filled in.
 */
export const readRunIdentity = (annotations: unknown): RunIdentity | null => {
    const view = readApplication(annotations)
    if (!view) return null
    const record = asRecord(asRecord(annotations).application)
    const identity: RunIdentity = {}
    if (typeof view.packageSha256 === "string" && view.packageSha256) {
        identity.packageSha256 = view.packageSha256
    }
    const plan = asRecord(record.managedRequirements)
    const sections = plan.sections
    if (
        Array.isArray(sections) &&
        sections.every((section) => (PACKAGE_SECTIONS as readonly unknown[]).includes(section)) &&
        new Set(sections).size === sections.length &&
        (plan.census === "required" || plan.census === "optional")
    ) {
        identity.requirements = {
            sections: PACKAGE_SECTIONS.filter((section) => sections.includes(section)),
            census: plan.census as CensusRequirement,
        }
    }
    const binding = asRecord(record.managedRosterBinding)
    if (
        typeof binding.sha256 === "string" &&
        /^[0-9a-f]{64}$/.test(binding.sha256) &&
        Number.isSafeInteger(binding.generation) &&
        Number.isSafeInteger(binding.requiredApprovals)
    ) {
        identity.roster = {
            sha256: binding.sha256,
            generation: binding.generation as number,
            requiredApprovals: binding.requiredApprovals as number,
        }
    }
    return identity
}

/** The signer roster the receiver enforces now, as the page reads it. */
export interface CurrentTrust {
    managed: boolean
    binding: {sha256: string; generation: string | number; requiredApprovals: number} | null
}

export type RecoveryBlock = "unsettled" | "rosterChanged" | "requirementsUnknown"

export type Recovery =
    | {kind: "none"}
    | {kind: "blocked"; taskId: string; reason: RecoveryBlock; applied: PackageSection[]}
    | {
          kind: "recoverable"
          taskId: string
          /** The original signed package: the next Preview must read exactly these bytes. */
          packageSha256: string
          /** The original requirements, locked for the continuation. */
          requirements: PackageRequirements
          applied: PackageSection[]
          remaining: PackageSection[]
          /**
           * The same uploaded document, only from the browser that followed the
           * run and only for a plain upload; otherwise the same file is chosen again.
           */
          documentId: string | null
      }

const SHA256_HEX = /^[0-9a-f]{64}$/

/**
 * Whether the latest run in History is an Apply that stopped partway and can be
 * continued: the same signed package (by its digest), on this tenant, with its
 * original requirements, under the roster it began with, applying only what is
 * left. Only that run's saved record decides; a bare failure, a refusal that
 * changed nothing, a later run or anything malformed offers nothing. The
 * receiver checks all of it again, and the page still needs a fresh Preview and
 * an explicit confirmation before anything is applied.
 */
export const readRecovery = (
    rows: readonly HistoryRow[] | null | undefined,
    tenantId: string | null | undefined,
    trust: CurrentTrust,
    followed: Pick<
        ActiveSubmission,
        "taskId" | "mode" | "documentId" | "requirements" | "encryptedPayloadSha256"
    > | null
): Recovery => {
    // A preview cannot settle an earlier Apply. A newer unclassified/in-flight task
    // still stops the search rather than letting us assume it was only a preview.
    const latest = rows?.find((row) => {
        const report = readApplication(row.annotations)
        return (
            report?.mode !== "preview" ||
            !["SUCCESS", "FAILED"].includes(row.execution_status ?? "")
        )
    })
    if (!latest || !tenantId || latest.execution_status !== "FAILED") return {kind: "none"}
    const view = readApplication(latest.annotations)
    const record = asRecord(asRecord(latest.annotations).application)
    const sha = typeof view?.packageSha256 === "string" ? view.packageSha256.toLowerCase() : ""
    if (
        !view ||
        view.mode !== "apply" ||
        (view.outcome !== "failed" && view.outcome !== "unknown") ||
        record.format !== "sequent.package-application/1" ||
        record.tenantId !== tenantId ||
        !SHA256_HEX.test(sha)
    ) {
        return {kind: "none"}
    }
    const isApplied = (section: SectionView) =>
        section.state === "applied" || section.earlierApplied
    const applied = PACKAGE_SECTIONS.filter((name) =>
        view.sections.some((section) => section.section === name && isApplied(section))
    )
    const remaining = PACKAGE_SECTIONS.filter((name) =>
        view.sections.some(
            (section) =>
                section.section === name &&
                !isApplied(section) &&
                section.state !== "not-included" &&
                section.state !== "unchanged"
        )
    )
    // A persisted submission proves Apply started even when no section completed.
    const beganSubmitting = view.sections.some(
        (section) => section.submissions.length > 0 || !!section.inFlight
    )
    if ((!applied.length && !beganSubmitting) || !remaining.length) return {kind: "none"}
    const taskId = latest.id
    // A submission whose effect is not known yet is never worked around.
    if (
        view.outcome === "unknown" ||
        view.sections.some((section) => !!section.inFlight || section.state === "submitting")
    ) {
        return {kind: "blocked", taskId, reason: "unsettled", applied}
    }
    const began = readRunIdentity(latest.annotations)?.roster
    const now = trust.binding
    const sameRoster = began
        ? trust.managed &&
          !!now &&
          now.sha256 === began.sha256 &&
          String(now.generation) === String(began.generation) &&
          now.requiredApprovals === began.requiredApprovals
        : !trust.managed
    if (!sameRoster) return {kind: "blocked", taskId, reason: "rosterChanged", applied}
    const ownFollow =
        followed && followed.taskId === taskId && followed.mode === "apply" ? followed : null
    const requirements =
        readRunIdentity(latest.annotations)?.requirements ?? ownFollow?.requirements ?? null
    if (!requirements) return {kind: "blocked", taskId, reason: "requirementsUnknown", applied}
    return {
        kind: "recoverable",
        taskId,
        packageSha256: sha,
        requirements,
        applied,
        remaining,
        documentId: ownFollow && !ownFollow.encryptedPayloadSha256 ? ownFollow.documentId : null,
    }
}
