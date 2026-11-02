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
    /** The nested zip the member is inside, or none for a root member. */
    container?: string
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
            container: typeof member.container === "string" ? member.container : undefined,
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
 * Whether the previewed package is the one whose SHA-256 the operator entered.
 * No entered digest is no check; the record's packageSha256 is the digest of
 * the bytes the platform read.
 */
export const shaMatches = (
    view: ApplicationView,
    expectedSha: string | null | undefined
): boolean => !expectedSha || view.packageSha256?.toLowerCase() === expectedSha.toLowerCase()

/**
 * An application is offered only for the document and requirements that were
 * just previewed, when that preview finished with nothing blocking it and, if
 * the operator entered a SHA-256, the platform read exactly that package.
 */
export const mayApply = (
    previewed: PreviewedPackage | null,
    tenantId: string | null | undefined,
    documentId: string | null,
    requirements: PackageRequirements,
    expectedSha?: string | null
): boolean =>
    !!previewed &&
    previewed.tenantId === tenantId &&
    previewed.documentId === documentId &&
    sameRequirements(previewed.requirements, requirements) &&
    previewed.view.outcome === "previewed" &&
    previewed.view.blockers.length === 0 &&
    shaMatches(previewed.view, expectedSha)

/** Whether a task's status is final: it will not change again. */
export const isFinished = (executionStatus: string | null | undefined): boolean =>
    executionStatus === "SUCCESS" || executionStatus === "FAILED" || executionStatus === "CANCELLED"

/**
 * The last line of a task row's log. A task that stops before it saves a
 * record (rejected signature, a missing document, another application on the
 * tenant) leaves its reason only there, with annotations `{document_id: null}`.
 */
export const lastLogLine = (logs: unknown): string | undefined => {
    const entries = Array.isArray(logs) ? logs : []
    const last = asObject(entries[entries.length - 1])
    return typeof last.log_text === "string" ? last.log_text : undefined
}

/** The submission a page is following, kept so a reload resumes it. */
export interface ActiveSubmission {
    taskId: string
    mode: ApplyMode
    documentId: string
    requirements: PackageRequirements
    /** The SHA-256 the operator entered for the upload, if any. */
    expectedSha?: string | null
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

/** Stop following this tenant's submission: nothing is cancelled or resubmitted. */
export const forgetSubmission = (tenantId: string): void => {
    try {
        sessionStorage.removeItem(storageKey(tenantId))
    } catch {
        // Nothing stored, nothing to forget.
    }
}

const names = (value: unknown): string[] =>
    Array.isArray(value) ? value.filter((v): v is string => typeof v === "string") : []

/**
 * One readable line per proposed change of a section's preview: what it
 * names (a field, group, username, trustee or event), the action, and the
 * keys, roles or files involved. Built from an allow-list, so a value the
 * platform did not mean as a name (a setting's value, a record) is never
 * shown; a display name's old and new text are the only values shown.
 */
export const changeLines = (changes: unknown): string[] => {
    const entries = Array.isArray(changes) ? changes : changes ? [changes] : []
    return entries.map((raw) => {
        const change = asObject(raw)
        const subject = ["field", "username", "group", "name", "electionEventId"]
            .map((key) => change[key])
            .filter((v): v is string => typeof v === "string")
            .join(" / ")
        const parts: string[] = []
        if (subject) parts.push(subject)
        if (typeof change.action === "string") parts.push(change.action)
        const listed: Array<[string, string[]]> = [
            ["keys", names(change.keys)],
            ["add", names(change.add)],
            ["remove", names(change.remove)],
            ["files", names(change.files)],
        ]
        for (const [label, values] of listed) {
            if (values.length) parts.push(`${label}: ${values.join(", ")}`)
        }
        if (typeof change.censusSize === "number") parts.push(`census: ${change.censusSize}`)
        if (
            change.field === "displayName" &&
            (typeof change.to === "string" || change.to === null)
        ) {
            parts.push(`${String(change.from ?? "—")} → ${String(change.to ?? "—")}`)
        }
        return parts.join(" · ") || "—"
    })
}

const SECRET_LIKE = /[A-Za-z0-9+/=_-]{32,}/g
const MAX_TEXT = 300

/**
 * Text from the platform, safe to show: long token-like runs (keys, hashes,
 * secrets) redacted, and the length bounded so a raw record is never echoed.
 */
export const safeText = (text: string): string => {
    const redacted = text.replace(SECRET_LIKE, "[redacted]")
    return redacted.length > MAX_TEXT ? `${redacted.slice(0, MAX_TEXT)}…` : redacted
}
