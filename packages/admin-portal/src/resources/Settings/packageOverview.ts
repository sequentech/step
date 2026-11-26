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
    readApplication,
    type ApplicationView,
    type ApplyMode,
    type PackageSection,
    type SectionView,
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
            return section.preview?.needsApply ? "willChange" : "unchanged"
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
export const orderedSectionBadges = (
    view: ApplicationView | null | undefined,
    running: boolean
): SectionRow[] =>
    PACKAGE_SECTIONS.map((section) => {
        const found = view?.sections.find((s) => s.section === section)
        return {section, badge: sectionBadge(found, running), view: found}
    })

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
    | "reviewAndApply"
    | "importFixed"
    | "changeRequirements"

/** The header's actions. A blocked preview never offers Apply. */
export const headerActions = (
    phase: PackageImportPhase
): {primary: HeaderAction; secondary?: HeaderAction} => {
    switch (phase) {
        case "review":
            return {primary: "reviewAndApply", secondary: "upload"}
        case "previewBlocked":
            return {primary: "importFixed", secondary: "changeRequirements"}
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
    appliedAt: string
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
        if (
            row.execution_status === "SUCCESS" &&
            view?.mode === "apply" &&
            view.outcome === "applied"
        ) {
            return {
                taskId: row.id,
                appliedAt: row.created_at,
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
