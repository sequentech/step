/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    hasFreshBoardAuthentication,
    savePendingBoardAction,
    consumePendingBoardAction,
    saveBoardSigningResume,
    readBoardSigningResume,
    matchesBoardSigningResume,
    BOARD_SIGNING_RESUME_KEY,
    requiresFreshBoardAuthentication,
} from "./boardAuthentication"
import {SigningAction, type ISigningRequest} from "./signing/types"
const now = 1_800_000_000_000
const scope = {tenantId: "tenant", userId: "person", eventId: "event", electionId: null}
const request = {
    id: "request",
    tenant_id: "tenant",
    election_event_id: "event",
    action: SigningAction.ApproveConfiguration,
    payload_sha256: "a".repeat(64),
    code: "ABCD-EFGH",
    expires_at: null,
} as ISigningRequest
beforeEach(() => sessionStorage.clear())
it.each([undefined, NaN, Infinity, "1800000000", -1, 1.5])(
    "rejects invalid auth_time %s",
    (auth_time) => {
        expect(hasFreshBoardAuthentication({auth_time}, now)).toBe(false)
    }
)
it.each([
    [0, true],
    [59_999, true],
    [60_000, false],
    [-1, false],
])("checks exact millisecond age %s", (age, expected) => {
    expect(hasFreshBoardAuthentication({auth_time: 1_800_000_000}, now + Number(age))).toBe(
        expected
    )
})
it("keeps pending scope through a nonfresh return, then executes at most once", () => {
    savePendingBoardAction(sessionStorage, "open", scope, ["ONLINE"], now)
    expect(consumePendingBoardAction(sessionStorage, "open", scope, false, now + 1000)).toEqual({
        status: "waiting",
    })
    expect(sessionStorage.getItem("open")).not.toBeNull()
    expect(consumePendingBoardAction(sessionStorage, "open", scope, true, now + 2000)).toEqual({
        status: "ready",
        channels: ["ONLINE"],
    })
    expect(consumePendingBoardAction(sessionStorage, "open", scope, true, now + 3000)).toEqual({
        status: "none",
    })
})
it.each(["tenantId", "userId", "eventId", "electionId"])(
    "refuses pending action after %s changed",
    (field) => {
        savePendingBoardAction(sessionStorage, "publish", scope, undefined, now)
        expect(
            consumePendingBoardAction(
                sessionStorage,
                "publish",
                {...scope, [field]: "other"},
                true,
                now
            )
        ).toEqual({status: "refused"})
    }
)
it.each([now - 1, now + 300_000])("rejects future or expired pending action at %s", (time) => {
    savePendingBoardAction(sessionStorage, "publish", scope, undefined, now)
    expect(consumePendingBoardAction(sessionStorage, "publish", scope, true, time)).toEqual({
        status: "refused",
    })
})
it("rejects a legacy unbound flag and malformed channel instead of widening scope", () => {
    sessionStorage.setItem("publish", "true")
    expect(consumePendingBoardAction(sessionStorage, "publish", scope, true, now)).toEqual({
        status: "refused",
    })
    savePendingBoardAction(sessionStorage, "open", scope, ["OTHER"], now)
    expect(consumePendingBoardAction(sessionStorage, "open", scope, true, now)).toEqual({
        status: "refused",
    })
})
it("stores only public exact-request bindings for reauthentication", () => {
    saveBoardSigningResume(sessionStorage, request, "person", now)
    const x = readBoardSigningResume(sessionStorage, "tenant", "person", now)!
    expect(Object.keys(x).sort()).toEqual(
        [
            "action",
            "code",
            "eventId",
            "expiresAt",
            "payloadHash",
            "requestId",
            "tenantId",
            "userId",
        ].sort()
    )
    expect(x.expiresAt).toBe(now + 300_000)
    expect(matchesBoardSigningResume(x, request)).toBe(true)
})
it.each(["tenant_id", "election_event_id", "id", "action", "payload_sha256", "code"])(
    "refuses changed signing binding %s",
    (field) => {
        saveBoardSigningResume(sessionStorage, request, "person", now)
        const x = readBoardSigningResume(sessionStorage, "tenant", "person", now)!
        expect(matchesBoardSigningResume(x, {...request, [field]: "changed"})).toBe(false)
    }
)
it.each([
    ["other", "person", now],
    ["tenant", "other", now],
    ["tenant", "person", now + 300_000],
])("refuses account/scope/expiry return %s %s %s", (tenant, user, time) => {
    saveBoardSigningResume(sessionStorage, request, "person", now)
    expect(() =>
        readBoardSigningResume(sessionStorage, String(tenant), String(user), Number(time))
    ).toThrow()
    expect(sessionStorage.getItem(BOARD_SIGNING_RESUME_KEY)).toBeNull()
})
it("retains request expiry if earlier than resume TTL", () => {
    saveBoardSigningResume(
        sessionStorage,
        {...request, expires_at: new Date(now + 1000).toISOString()},
        "person",
        now
    )
    expect(readBoardSigningResume(sessionStorage, "tenant", "person", now)?.expiresAt).toBe(
        now + 1000
    )
})
it("preserves the two trustee custody authentication contracts", () => {
    for (const action of Object.values(SigningAction))
        expect(requiresFreshBoardAuthentication(action)).toBe(
            ![SigningAction.ConfirmCustodyProof, SigningAction.ContributeCustodyProof].includes(
                action
            )
        )
})

it("refuses moving an intent to a different protected action", () => {
    savePendingBoardAction(sessionStorage, "open", scope, ["ONLINE"], now)
    sessionStorage.setItem("close", sessionStorage.getItem("open")!)
    expect(consumePendingBoardAction(sessionStorage, "close", scope, true, now)).toEqual({
        status: "refused",
    })
})
