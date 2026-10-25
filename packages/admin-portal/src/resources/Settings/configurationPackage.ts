// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Applying a complete configuration package (the delivery zip) to the
// caller's tenant: the record the platform keeps in the task row's
// annotations.application, read the way step-cli's apply-configuration-package
// reads it, and the rule for when an application may be submitted.

import {IPermissions} from "@/types/keycloak"

/** The sections in the order the platform applies them. */
export const PACKAGE_SECTIONS = [
    "tenant-configuration",
    "tenant-settings",
    "role-permissions",
    "admin-accounts",
    "trustees",
    "election-event",
] as const

export type PackageSection = (typeof PACKAGE_SECTIONS)[number]

export type CensusRequirement = "optional" | "required"

export interface PackageRequirements {
    sections: PackageSection[]
    census: CensusRequirement
}

/** What a complete delivery carries: every section and its census. */
export const COMPLETE_DELIVERY: PackageRequirements = {
    sections: [...PACKAGE_SECTIONS],
    census: "required",
}

/** Every permission the platform's route requires, for preview as for apply. */
export const PACKAGE_PERMISSIONS: IPermissions[] = [
    IPermissions.TENANT_WRITE,
    IPermissions.USER_WRITE,
    IPermissions.ROLE_WRITE,
    IPermissions.TRUSTEE_CREATE,
    IPermissions.TRUSTEE_WRITE,
    IPermissions.ELECTION_EVENT_CREATE,
]

/** Whether the caller holds all of them on the tenant: one missing hides the page. */
export const mayApplyPackages = (
    tenantId: string | null | undefined,
    isAuthorized: (checkSuperAdmin: boolean, tenantId: string | null, role: IPermissions) => boolean
): boolean =>
    !!tenantId &&
    PACKAGE_PERMISSIONS.every((permission) => isAuthorized(false, tenantId, permission))

export type ApplyMode = "preview" | "apply"

export type Outcome = "running" | "previewed" | "applied" | "refused" | "failed" | "unknown"

export interface SectionPreview {
    needsApply: boolean
    changes: unknown
    blockers: string[]
}

export interface SectionView {
    section: string
    members: string[]
    state: string
    preview?: SectionPreview
    /** This run's task for the section: its submission's, or a failed or unsettled one's. */
    taskId?: string
    reconciled?: boolean
    error?: string
    /** Whether any run so far confirmed the section applied. */
    earlierApplied: boolean
    /** Every task begun for the section, by any run, oldest first. */
    submissions: string[]
    /** A submission still not settled. */
    inFlight?: string
}

export interface MemberView {
    path: string
    disposition: string
    section?: string
}

export interface ApplicationView {
    outcome: Outcome | "unknown"
    packageSha256?: string
    mode?: ApplyMode
    members: MemberView[]
    sections: SectionView[]
    blockers: string[]
}

const asObject = (value: unknown): Record<string, any> =>
    value && typeof value === "object" ? (value as Record<string, any>) : {}

/** The record in a task row's annotations, or null while the task has none yet. */
export const readApplication = (annotations: unknown): ApplicationView | null => {
    const record = asObject(asObject(annotations).application)
    if (!record.outcome) {
        return null
    }
    const sections: SectionView[] = (Array.isArray(record.sections) ? record.sections : []).map(
        (raw: unknown) => {
            const section = asObject(raw)
            const status = asObject(section.status)
            const preview = status.preview ? asObject(status.preview) : undefined
            const submission = asObject(status.submission)
            const history = asObject(section.history)
            return {
                section: String(section.section ?? "?"),
                members: Array.isArray(section.members) ? section.members.map(String) : [],
                state: String(status.state ?? "?"),
                preview: preview && {
                    needsApply: preview.needsApply === true,
                    changes: preview.changes,
                    blockers: Array.isArray(preview.blockers) ? preview.blockers.map(String) : [],
                },
                // SectionSubmission is camelCase (taskId); the fields inside a
                // SectionStatus variant keep their Rust name (task_id).
                taskId: submission.taskId ?? status.task_id ?? undefined,
                reconciled: typeof status.reconciled === "boolean" ? status.reconciled : undefined,
                error: typeof status.error === "string" ? status.error : undefined,
                earlierApplied: history.applied === true,
                submissions: (Array.isArray(history.submissions) ? history.submissions : [])
                    .map((entry: unknown) => asObject(entry).taskId)
                    .filter((id: unknown): id is string => typeof id === "string"),
                inFlight: typeof history.inFlight === "string" ? history.inFlight : undefined,
            }
        }
    )
    const members: MemberView[] = (
        Array.isArray(asObject(record.coverage).members) ? record.coverage.members : []
    ).map((raw: unknown) => {
        const member = asObject(raw)
        const disposition = asObject(member.disposition)
        return {
            path: String(member.path ?? "?"),
            disposition: String(disposition.kind ?? member.disposition ?? "?"),
            section: disposition.section ? String(disposition.section) : undefined,
        }
    })
    return {
        outcome: record.outcome,
        packageSha256: record.packageSha256,
        mode: record.mode,
        members,
        sections,
        blockers: sections.flatMap((s) =>
            (s.preview?.blockers ?? []).map((b) => `${s.section}: ${b}`)
        ),
    }
}

/** How many sections a preview would change. */
export const sectionsToChange = (view: ApplicationView): number =>
    view.sections.filter((s) => s.preview?.needsApply).length

export interface PreviewedPackage {
    tenantId: string
    documentId: string
    requirements: PackageRequirements
    view: ApplicationView
}

const sameRequirements = (a: PackageRequirements, b: PackageRequirements): boolean =>
    a.census === b.census &&
    a.sections.length === b.sections.length &&
    a.sections.every((section) => b.sections.includes(section))

/**
 * An application is offered only for the document and requirements that were
 * just previewed, when that preview finished with nothing blocking it.
 */
export const mayApply = (
    previewed: PreviewedPackage | null,
    tenantId: string | null | undefined,
    documentId: string | null,
    requirements: PackageRequirements
): boolean =>
    !!previewed &&
    previewed.tenantId === tenantId &&
    previewed.documentId === documentId &&
    sameRequirements(previewed.requirements, requirements) &&
    previewed.view.outcome === "previewed" &&
    previewed.view.blockers.length === 0

/** Whether a task's status is final. */
export const isFinished = (executionStatus: string | null | undefined): boolean =>
    executionStatus === "SUCCESS" || executionStatus === "FAILED"

/** The submission a page is following, kept so a reload resumes it. */
export interface ActiveSubmission {
    taskId: string
    mode: ApplyMode
    documentId: string
    requirements: PackageRequirements
}

const storageKey = (tenantId: string) => `configurationPackage.active.${tenantId}`

/** Remember the submission for this tenant in this browser session. */
export const rememberSubmission = (tenantId: string, submission: ActiveSubmission): void => {
    try {
        sessionStorage.setItem(storageKey(tenantId), JSON.stringify(submission))
    } catch {
        // Without storage the page still works; a reload just loses the task.
    }
}

/** The submission this tenant's page was following before a reload, if any. */
export const recalledSubmission = (
    tenantId: string | null | undefined
): ActiveSubmission | null => {
    if (!tenantId) {
        return null
    }
    try {
        const raw = sessionStorage.getItem(storageKey(tenantId))
        const value = raw ? JSON.parse(raw) : null
        return value && typeof value.taskId === "string" && typeof value.documentId === "string"
            ? (value as ActiveSubmission)
            : null
    } catch {
        return null
    }
}
