// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {normalizeBallotLogPrefix} from "./BallotLogSearch"

test("accepts partial short codes and canonicalizes their separator", () => {
    expect(normalizeBallotLogPrefix(" e9 ")).toBe("E9")
    expect(normalizeBallotLogPrefix("7qkm-2h")).toBe("7QKM2H")
    expect(normalizeBallotLogPrefix(" ")).toBe("")
})

test("rejects invalid or overlong codes without issuing an unrestricted search", () => {
    expect(normalizeBallotLogPrefix("U")).toBeNull()
    expect(normalizeBallotLogPrefix("123456789")).toBeNull()
    expect(normalizeBallotLogPrefix("%_")).toBeNull()
})
