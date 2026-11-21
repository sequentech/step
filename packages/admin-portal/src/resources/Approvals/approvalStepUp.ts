// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// An event can ask for a fresh second factor before an enrollment is approved
// (`presentation.approval_step_up_policy`). Harvest refuses an approval
// without one with a 403 `ApprovalStepUpRequired`; the portal signs in again
// first and resumes the approval when Keycloak sends it back.

export enum EApprovalStepUpPolicy {
    DISABLED = "disabled",
    ENABLED = "enabled",
}

export const APPROVAL_STEP_UP_POLICY_KEY = "approval_step_up_policy"
export const APPROVAL_STEP_UP_REQUIRED = "ApprovalStepUpRequired"
/** The event tab Keycloak sends the administrator back to. */
export const APPROVALS_TAB_ID = "approvals"
export const PENDING_APPROVAL_KEY = "PENDING_APPROVAL_STEP_UP"

/** The approval waiting for the second factor. */
export interface PendingApproval {
    tenantId: string
    electionEventId: string
    applicationId: string
    voterId: string
}

const record = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" ? (value as Record<string, unknown>) : {}

/**
 * Whether approving needs the second factor, as Harvest decides it: missing
 * is disabled, and a value that isn't a policy requires it.
 */
export function requiresApprovalStepUp(presentation: unknown): boolean {
    const policy = record(presentation)[APPROVAL_STEP_UP_POLICY_KEY]
    return policy !== undefined && policy !== null && policy !== EApprovalStepUpPolicy.DISABLED
}

function parsed(body: unknown): Record<string, unknown> {
    if (typeof body !== "string") return record(body)
    try {
        return record(JSON.parse(body))
    } catch {
        return {}
    }
}

/** Whether a failed approval is Harvest asking for the second factor. */
export function isStepUpRefusal(error: unknown): boolean {
    const graphQLErrors = record(error).graphQLErrors
    if (!Array.isArray(graphQLErrors)) return false
    return graphQLErrors.some((graphQLError) => {
        const extensions = record(record(graphQLError).extensions)
        const body = parsed(record(record(extensions.internal).response).body)
        return [extensions.code, record(body.extensions).code].includes(APPROVAL_STEP_UP_REQUIRED)
    })
}

/** Where Keycloak sends the administrator back to: this page, on the approvals tab. */
export function stepUpReturnUrl(href: string): string {
    const url = new URL(href)
    url.searchParams.set("tabId", APPROVALS_TAB_ID)
    return url.toString()
}

export function savePendingApproval(pending: PendingApproval, storage: Storage = sessionStorage) {
    storage.setItem(PENDING_APPROVAL_KEY, JSON.stringify(pending))
}

/** The pending approval of this tenant and event, if any. */
export function readPendingApproval(
    tenantId: string | null | undefined,
    electionEventId: string,
    storage: Storage = sessionStorage
): PendingApproval | null {
    let pending: Record<string, unknown>
    try {
        pending = parsed(storage.getItem(PENDING_APPROVAL_KEY))
    } catch {
        return null
    }
    const {applicationId, voterId} = pending
    return typeof tenantId === "string" &&
        pending.tenantId === tenantId &&
        pending.electionEventId === electionEventId &&
        typeof applicationId === "string" &&
        typeof voterId === "string"
        ? {tenantId, electionEventId, applicationId, voterId}
        : null
}

export function clearPendingApproval(storage: Storage = sessionStorage) {
    try {
        storage.removeItem(PENDING_APPROVAL_KEY)
    } catch {
        // Nothing was stored.
    }
}
