// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    ELogListLayout,
    SIGNING_LOG_STATEMENT_KINDS,
    TENANT_LOG_STATEMENT_KINDS,
    logListLayout,
} from "./logScope"

it("exposes separate event audit filters for approval, quorum and finalization", () => {
    const kinds = new Map(SIGNING_LOG_STATEMENT_KINDS)
    expect(kinds.get("SigningRequestCreated")).toBe("created")
    expect(kinds.get("SigningRequestSigned")).toBe("approved")
    expect(kinds.get("SigningRequestCompleted")).toBe("quorum")
    expect(kinds.get("SigningActionExecuted")).toBe("actionResult")
    expect(kinds.get("SigningRequestCancelled")).toBe("cancelled")
    expect(kinds.get("SigningRequestExpired")).toBe("expired")
    expect(kinds.get("SigningSignatureRefused")).toBe("refused")
    expect(kinds.size).toBe(SIGNING_LOG_STATEMENT_KINDS.length)
})

describe("the layout a log list takes from its own width", () => {
    it.each([
        [null, ELogListLayout.WIDE],
        [1920, ELogListLayout.WIDE],
        [1500, ELogListLayout.WIDE],
        [1499, ELogListLayout.MEDIUM],
        [1118, ELogListLayout.MEDIUM],
        [1000, ELogListLayout.MEDIUM],
        [999, ELogListLayout.COMPACT],
        [360, ELogListLayout.COMPACT],
        [0, ELogListLayout.COMPACT],
    ])("at %s px is %s", (width, layout) => {
        expect(logListLayout(width)).toBe(layout)
    })
})

it("names the statement kinds of the tenant log once each", () => {
    expect(new Set(TENANT_LOG_STATEMENT_KINDS).size).toBe(TENANT_LOG_STATEMENT_KINDS.length)
    expect(TENANT_LOG_STATEMENT_KINDS).toContain("AdminChange")
    expect(TENANT_LOG_STATEMENT_KINDS).toContain("KeycloakUserEvent")
    // The requests Harvest records outside an election event.
    expect(TENANT_LOG_STATEMENT_KINDS).toContain("TenantAdminAction")
})
