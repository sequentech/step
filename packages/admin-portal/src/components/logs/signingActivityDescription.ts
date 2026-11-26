// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {SigningAction} from "@/lib/signing/types"
import type {TFunction} from "i18next"
import type {ILogMessage} from "./logMessage"

const record = (value: unknown): Record<string, unknown> | null =>
    value && typeof value === "object" && !Array.isArray(value)
        ? (value as Record<string, unknown>)
        : null

const decodeDetails = (value: string): Record<string, unknown> | null => {
    try {
        return record(JSON.parse(value))
    } catch {
        return null
    }
}

const signingKinds = new Set([
    "SigningRequestCreated",
    "SigningRequestSigned",
    "SigningRequestCompleted",
    "SigningActionExecuted",
    "SigningRequestCancelled",
    "SigningRequestExpired",
    "SigningSignatureRefused",
])

/** A row describes the signed statement; the entry dialog keeps the original. */
export const signingActivityDescription = (
    t: TFunction,
    kind: string | null | undefined,
    message: ILogMessage | null
): string | null => {
    if (!kind || !signingKinds.has(kind)) return null
    const head = message?.statement?.head
    const entry = record(record(message?.statement?.body)?.Signing)
    if (
        head?.kind !== kind ||
        entry?.kind !== kind ||
        entry.event_type !== head.event_type ||
        entry.log_type !== head.log_type ||
        (head.event_type !== "USER" && head.event_type !== "SYSTEM") ||
        typeof entry.details_json !== "string"
    ) {
        return null
    }
    const details = decodeDetails(entry.details_json)
    if (!details) return null

    const row = "logsScreen.signingActivity.row"
    // The same statement kind also carries refused cancel/handover attempts.
    // Do not present those, or the user's attempted action, as a signature rejection.
    if (kind === "SigningSignatureRefused") {
        if (head.event_type !== "SYSTEM" || head.log_type !== "ERROR" || details.step !== "sign") {
            return null
        }
        const rejected = String(t(`${row}.rejected`))
        const check = details.check
        // Only reuse known, parameter-free descriptions, not the raw reason/detail.
        return check === "already-signed" || check === "signature"
            ? `${rejected}: ${t(`signing.dialog.checks.failed.${check}`)}`
            : rejected
    }

    if (kind !== "SigningActionExecuted" && head.log_type !== "INFO") return null
    const actionId = Object.values(SigningAction).find((action) => action === details.action)
    if (!actionId) return null
    const action = String(t(`signing.actions.${actionId}.label`))
    const describe = (key: string) => String(t(`${row}.${key}`, {action}))
    const progress = (atQuorum: boolean) => {
        const {count, required} = details
        if (
            typeof count !== "number" ||
            typeof required !== "number" ||
            !Number.isSafeInteger(count) ||
            !Number.isSafeInteger(required) ||
            count <= 0 ||
            count > required ||
            (atQuorum && count !== required)
        ) {
            return ""
        }
        return ` ${t(`${row}.progress`, {approved: count, required})}`
    }

    switch (kind) {
        case "SigningRequestCreated":
            return describe("created")
        case "SigningRequestSigned":
            return describe("approved") + progress(false)
        case "SigningRequestCompleted":
            return describe("quorum") + progress(true)
        case "SigningActionExecuted":
            // A USER record is informational even when the SYSTEM effect failed.
            // Use the recorded request status, not the log severity or quorum count.
            if (details.status === "executed" && head.log_type === "INFO") {
                return describe("completed")
            }
            if (
                details.status === "failed" &&
                ((head.event_type === "USER" && head.log_type === "INFO") ||
                    (head.event_type === "SYSTEM" && head.log_type === "ERROR"))
            ) {
                return describe("failed")
            }
            return null
        case "SigningRequestCancelled":
            return describe("cancelled")
        case "SigningRequestExpired":
            return describe("expired")
        default:
            return null
    }
}
