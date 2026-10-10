// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The two ports a host fills: selection state and the WebAssembly engine.
 *
 * Without a provider, reads answer "nothing selected" and every write or engine
 * question fails naming what is missing, rather than a ballot silently drawing
 * from a guess.
 */

import {renderHook} from "@testing-library/react"
import type {ICandidate, IDecodedVoteContest} from "@sequentech/ui-core"
import React from "react"

import {anEngine} from "../testing/ballot"
import {BallotEngineProvider, useBallotEngine} from "./engine"
import {BallotSelectionProvider, type BallotSelectionPort, useBallotSelection} from "./selection"
import type {IBallotStyle} from "./types"

const style = {} as IBallotStyle

describe("without a selection provider", () => {
    const port = () => renderHook(() => useBallotSelection()).result.current

    it("reads as an empty ballot that nobody has voted", () => {
        expect(port().contest(style, "c")).toBeUndefined()
        expect(port().choice(style, "c", "a")).toBeUndefined()
        expect(port().isVoted("e")).toBe(false)
        expect(port().imageBaseUrl).toBe("")
    })

    it.each(["setChoice", "setBlank", "setInvalid", "reset"] as const)(
        "refuses %s, naming the provider a host must supply",
        (write) => {
            expect(() => (port()[write] as (input: never) => void)({} as never)).toThrow(
                /No BallotSelectionProvider above this ballot/
            )
        }
    )
})

describe("without an engine provider", () => {
    it.each([
        "sortCandidatesInContest",
        "isPreferential",
        "checkIsBlank",
        "getWriteInAvailableCharacters",
        "isEligibleAcclaimedCandidate",
    ] as const)("refuses %s by name", (question) => {
        const {result} = renderHook(() => useBallotEngine())
        expect(() => (result.current[question] as () => unknown)()).toThrow(
            new RegExp(`No BallotEngine above this ballot, so ${question} cannot be answered`)
        )
    })
})

describe("with providers", () => {
    it("hands the host's own port and engine to the ballot", () => {
        const port = {isVoted: () => true, imageBaseUrl: "/img/"} as unknown as BallotSelectionPort
        const engine = anEngine(5)
        const {result} = renderHook(
            () => ({selection: useBallotSelection(), engine: useBallotEngine()}),
            {
                wrapper: ({children}: {children: React.ReactNode}) => (
                    <BallotEngineProvider engine={engine}>
                        <BallotSelectionProvider port={port}>{children}</BallotSelectionProvider>
                    </BallotEngineProvider>
                ),
            }
        )
        expect(result.current.selection).toBe(port)
        expect(result.current.engine).toBe(engine)
        expect(
            result.current.engine.getWriteInAvailableCharacters(
                {choices: [{id: "w", selected: 0, write_in_text: "AB"}]} as IDecodedVoteContest,
                {} as IBallotStyle["ballot_eml"]
            )
        ).toBe(3)
        expect(
            result.current.engine.isEligibleAcclaimedCandidate({
                presentation: {is_write_in: true},
            } as ICandidate)
        ).toBe(false)
    })
})
