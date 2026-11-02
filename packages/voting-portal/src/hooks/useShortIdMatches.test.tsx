/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {renderHook, waitFor} from "@testing-library/react"
import type {Policy} from "@vitrine/verify"
import {EShortIdKind, EShortIdStatus} from "../services/vitrine/shortId"
import {ETransparencyMode} from "../services/vitrine/types"
import type {ITransparency} from "../services/vitrine/presentation"
import {useShortIdMatches} from "./useShortIdMatches"

const mockLookup = jest.fn()
let mockPolicy: {policy?: Policy; settled: boolean} = {settled: true}

jest.mock("./useVitrinePolicy", () => ({useVitrinePolicyState: () => mockPolicy}))
jest.mock("../services/vitrine/shortId", () => ({
    ...jest.requireActual("../services/vitrine/shortId"),
    lookupShortId: (...args: unknown[]) => mockLookup(...args),
}))

const POLICY = {
    logs: new Map([
        [
            "ballots",
            {origin: "logs.example.org/e/ballots", url: "https://logs.example.org/e/ballots/"},
        ],
    ]),
    monitors: new Map(),
} as unknown as Policy

const PUBLIC: ITransparency = {
    mode: ETransparencyMode.PUBLIC,
    vitrine: {policy: "x"},
} as unknown as ITransparency
const PRIVATE: ITransparency = {mode: ETransparencyMode.PRIVATE} as unknown as ITransparency

const RESULT = {
    kind: EShortIdKind.LISTED,
    typed: "FTBEMHRX",
    matches: [],
    total: 0,
    truncated: false,
}

beforeEach(() => {
    mockLookup.mockReset()
    mockPolicy = {policy: POLICY, settled: true}
})

describe("the short ID lookup hook", () => {
    it("does nothing for a full ID, or an event that has no public policy", () => {
        const full = renderHook(() =>
            useShortIdMatches(`FTBEMHRX${"0".repeat(44)}`, "event", PUBLIC)
        )
        expect(full.result.current.status).toBe(EShortIdStatus.OFF)
        const hidden = renderHook(() => useShortIdMatches("FTBEMHRX", "event", PRIVATE))
        expect(hidden.result.current.status).toBe(EShortIdStatus.OFF)
        const none = renderHook(() => useShortIdMatches(undefined, "event", PUBLIC))
        expect(none.result.current.status).toBe(EShortIdStatus.OFF)
        expect(mockLookup).not.toHaveBeenCalled()
    })

    it("waits for the policy, then looks the ID up, with nothing kept on this device", async () => {
        mockPolicy = {settled: false}
        mockLookup.mockResolvedValue(RESULT)
        const {result, rerender} = renderHook(() => useShortIdMatches("ftbe-mhrx", "event", PUBLIC))
        expect(result.current.status).toBe(EShortIdStatus.LOADING)
        expect(mockLookup).not.toHaveBeenCalled()

        mockPolicy = {policy: POLICY, settled: true}
        rerender()
        await waitFor(() => expect(result.current.status).toBe(EShortIdStatus.DONE))
        expect(result.current.result).toBe(RESULT)
        expect(mockLookup).toHaveBeenCalledWith(POLICY, "FTBEMHRX", expect.anything())
    })

    it("is done without a result when the policy names no server, so the other routes answer", async () => {
        mockPolicy = {
            policy: {logs: new Map(), monitors: new Map()} as unknown as Policy,
            settled: true,
        }
        const {result} = renderHook(() => useShortIdMatches("FTBEMHRX", "event", PUBLIC))
        await waitFor(() => expect(result.current.status).toBe(EShortIdStatus.DONE))
        expect(result.current.result).toBeUndefined()
        expect(mockLookup).not.toHaveBeenCalled()
    })

    it("is done without a result when the policy did not load", async () => {
        mockPolicy = {settled: true}
        const {result} = renderHook(() => useShortIdMatches("FTBEMHRX", "event", PUBLIC))
        await waitFor(() => expect(result.current.status).toBe(EShortIdStatus.DONE))
        expect(result.current.result).toBeUndefined()
    })

    it("reports the servers as unavailable when the lookup fails", async () => {
        mockLookup.mockRejectedValue(new Error("down"))
        const {result} = renderHook(() => useShortIdMatches("FTBEMHRX", "event", PUBLIC))
        await waitFor(() => expect(result.current.status).toBe(EShortIdStatus.DONE))
        expect(result.current.result?.kind).toBe(EShortIdKind.UNAVAILABLE)
    })

    it("looks again when the typed ID changes", async () => {
        mockLookup.mockResolvedValue(RESULT)
        let typed = "FTBEMHRX"
        const {result, rerender} = renderHook(() => useShortIdMatches(typed, "event", PUBLIC))
        await waitFor(() => expect(result.current.status).toBe(EShortIdStatus.DONE))
        typed = "FTBEMHRX71J7"
        rerender()
        expect(result.current.status).toBe(EShortIdStatus.LOADING)
        await waitFor(() => expect(result.current.status).toBe(EShortIdStatus.DONE))
        expect(mockLookup.mock.calls.map((call) => call[1])).toEqual(["FTBEMHRX", "FTBEMHRX71J7"])
    })
})
