// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {TFunction} from "i18next"
import type {ILogMessage} from "./logMessage"
import {signingActivityDescription} from "./signingActivityDescription"

const t = ((key: string, options: Record<string, unknown> = {}) =>
    [key.split(".").at(-1), ...Object.values(options)].join(" ")) as unknown as TFunction

const row = (
    kind: string,
    details: Record<string, unknown>,
    event_type = "SYSTEM",
    log_type = "INFO"
): ILogMessage => ({
    statement: {
        head: {kind, event_type, log_type, description: "The original signed sentence"},
        body: {
            Signing: {
                kind,
                event_type,
                log_type,
                details_json: JSON.stringify({action: "close-voting", ...details}),
            },
        },
    },
})

const describeRow = (kind: string, details: Record<string, unknown>, who?: string, type?: string) =>
    signingActivityDescription(t, kind, row(kind, details, who, type))

it("uses only the current approval's structured, safe integer count", () => {
    expect(describeRow("SigningRequestSigned", {count: 1, required: 2})).toBe(
        "approved label progress 1 2"
    )
    for (const invalid of [
        {count: "1", required: 2},
        {count: 1, required: undefined},
        {count: 3, required: 2},
        {count: 1.5, required: 2},
        {count: 0, required: 2},
    ]) {
        expect(describeRow("SigningRequestSigned", invalid)).toBe("approved label")
    }
    expect(describeRow("SigningRequestCompleted", {count: 2, required: 2})).toBe(
        "quorum label progress 2 2"
    )
    expect(describeRow("SigningRequestCompleted", {count: 1, required: 2})).toBe("quorum label")
})

it("keeps threshold distinct from a settled success or failure in both rows", () => {
    expect(describeRow("SigningActionExecuted", {status: "executed"})).toBe("completed label")
    expect(describeRow("SigningActionExecuted", {status: "failed"}, "SYSTEM", "ERROR")).toBe(
        "failed label"
    )
    expect(describeRow("SigningActionExecuted", {status: "failed"}, "USER", "INFO")).toBe(
        "failed label"
    )
    expect(describeRow("SigningActionExecuted", {status: "completed"})).toBeNull()
    expect(describeRow("SigningActionExecuted", {status: "executed"}, "SYSTEM", "ERROR")).toBeNull()
})

it("labels only an explicit system rejection of signing and never emits raw detail", () => {
    const kind = "SigningSignatureRefused"
    const privateFields = {step: "sign", check: "other-check", detail: "secret-refusal-text"}
    expect(describeRow(kind, privateFields, "SYSTEM", "ERROR")).toBe("rejected")
    expect(describeRow(kind, {...privateFields, check: "signature"}, "SYSTEM", "ERROR")).toBe(
        "rejected: signature"
    )
    expect(describeRow(kind, privateFields, "USER")).toBeNull()
    expect(describeRow(kind, {...privateFields, step: "cancel"}, "SYSTEM", "ERROR")).toBeNull()
    expect(describeRow("SignerDeclined", privateFields, "SYSTEM", "ERROR")).toBeNull()
})

it("doesn't interpret unknown actions, malformed details, or other signed statement kinds", () => {
    expect(describeRow("SigningRequestCreated", {action: "unrecognized"})).toBeNull()
    const message = row("SigningRequestCancelled", {}, "USER")
    expect(signingActivityDescription(t, "SigningRequestExpired", message)).toBeNull()
    expect(signingActivityDescription(t, "SigningRequestCancelled", message)).toBe(
        "cancelled label"
    )
    expect(message.statement?.head?.description).toBe("The original signed sentence")
})
