/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {renderHook} from "@testing-library/react"
import {ballotBoxSealPolicyOf, useBallotBoxesReadiness} from "./useBallotBoxesReadiness"
import {EBallotBoxesReadiness} from "@/services/tallyEligibility"

let mockReply: {data?: unknown; error?: Error} = {}
const mockUseQuery = jest.fn((_query: unknown, _options: unknown) => mockReply)

jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useQuery: (query: unknown, options: unknown) => mockUseQuery(query, options),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...require("../../../../ui-core/src/types/CoreTypes"),
    ...require("../../../../ui-core/src/types/ElectionEventPresentation"),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {QUERY_POLL_INTERVAL_MS: 1}}),
}))

const SEALING = {ballot_box_seal_policy: "seal-at-close"}

beforeEach(() => {
    mockReply = {}
    mockUseQuery.mockClear()
})

it("reads the policy from the presentation object or its JSON", () => {
    expect(ballotBoxSealPolicyOf(SEALING)).toBe("seal-at-close")
    expect(ballotBoxSealPolicyOf(JSON.stringify(SEALING))).toBe("seal-at-close")
    expect(ballotBoxSealPolicyOf({})).toBe("do-not-seal")
    expect(ballotBoxSealPolicyOf("not json")).toBe("do-not-seal")
})

it("asks nothing and changes nothing when the event doesn't seal", () => {
    const {result} = renderHook(() => useBallotBoxesReadiness("event", ["e1"], {}))
    expect(result.current).toBeUndefined()
    expect(mockUseQuery.mock.calls[0][1]).toMatchObject({skip: true})
})

it("asks with tally-read and is loading until the seals arrive", () => {
    const {result} = renderHook(() => useBallotBoxesReadiness("event", ["e1"], SEALING))
    expect(mockUseQuery.mock.calls[0][1]).toMatchObject({
        skip: false,
        variables: {electionEventId: "event", electionIds: ["e1"]},
        context: {headers: {"x-hasura-role": "tally-read"}},
    })
    expect(result.current?.e1.readiness).toBe(EBallotBoxesReadiness.LOADING)
})

it("is unavailable, not 'not sealed', when the seals can't be read", () => {
    mockReply = {error: new Error("denied")}
    const {result} = renderHook(() => useBallotBoxesReadiness("event", ["e1"], SEALING))
    expect(result.current?.e1.readiness).toBe(EBallotBoxesReadiness.UNAVAILABLE)
})

it("summarizes the seals of each election", () => {
    mockReply = {
        data: {
            sequent_backend_ballot_box_seal: [
                {election_id: "e1", area_id: "a", status: "published"},
                {election_id: "e2", area_id: "a", status: "sealed"},
            ],
        },
    }
    const {result} = renderHook(() => useBallotBoxesReadiness("event", ["e1", "e2"], SEALING))
    expect(result.current?.e1.readiness).toBe(EBallotBoxesReadiness.READY)
    expect(result.current?.e2.readiness).toBe(EBallotBoxesReadiness.PUBLISHING)
})
