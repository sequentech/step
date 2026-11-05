// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {matchedCharacters, parseBallotLogSearch, searchesAnything} from "./BallotLogSearch"

const parse = (input: string) => parseBallotLogSearch(input)!

test("a colon searches pseudonyms only", () => {
    expect(parse("k7qm:2x")).toEqual({query: "k7qm:2x", pseudonym: "K7QM2X", trackingId: null})
    expect(parse("K7QM:")).toMatchObject({pseudonym: "K7QM", trackingId: null})
    expect(parse("K7QM:2XWD")).toMatchObject({pseudonym: "K7QM2XWD", trackingId: null})
})

test("a hyphen searches Tracking IDs only, from the 8 characters of the short ID", () => {
    expect(parse("ftbe-mhrx")).toMatchObject({pseudonym: null, trackingId: "FTBEMHRX"})
    expect(parse(" FTBE-MHRX-1 ")).toMatchObject({pseudonym: null, trackingId: "FTBEMHRX1"})
    expect(parse("FTBE-MH")).toMatchObject({pseudonym: null, trackingId: null})
})

test("neither separator searches both", () => {
    expect(parse("FTBEMHRX")).toMatchObject({pseudonym: "FTBEMHRX", trackingId: "FTBEMHRX"})
    expect(parse("FTBE")).toMatchObject({pseudonym: "FTBE", trackingId: null})
    expect(parse("FTBEMHRXA")).toMatchObject({pseudonym: null, trackingId: "FTBEMHRXA"})
})

test("O is 0, and I and L are 1", () => {
    expect(parse("oil:").pseudonym).toBe("011")
    expect(parse("OIL1-OIL1").trackingId).toBe("01110111")
})

test("an empty search is the whole log, and an unreadable one is never sent", () => {
    expect(parse("  ")).toEqual({query: "", pseudonym: null, trackingId: null})
    expect(searchesAnything(parse(""))).toBe(true)
    expect(searchesAnything(parse("FTBE-MH"))).toBe(false)
    for (const bad of ["U", "%_", "FTBE;", "A".repeat(81)]) {
        expect(parseBallotLogSearch(bad)).toBeNull()
    }
})

test("the matched start of a displayed code counts characters, not separators", () => {
    expect(matchedCharacters("FTBE-MHRX", "FTBEM")).toBe(5)
    expect(matchedCharacters("K7QM:2XWD", "K7QM2")).toBe(5)
    expect(matchedCharacters("K7QM:2XWD", "FTBE")).toBe(0)
    expect(matchedCharacters("FTBE-MHRX", null)).toBe(0)
    expect(matchedCharacters("FTBE-MHRX", "FTBEMHRXA")).toBe(8)
})
