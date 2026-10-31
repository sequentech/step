// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {render} from "@testing-library/react"
import {GroupedBallotId} from "./GroupedBallotId"

const MINE = "a41c7e095b3d28f6c0e19d743a58b2f06e1d84c90b7f5a23d816e4c279b013af"
const DECODED = "a41c7e09e2c8913f07ba6d51f4a2c83915e79b0d62fa3c48a91e0d75be364f02"

const texts = (nodes: ArrayLike<Element>): (string | null)[] =>
    Array.from(nodes).map((node) => node.textContent)

describe("GroupedBallotId", () => {
    it("shows the ID in groups of four, the first two in bold", () => {
        const {container} = render(<GroupedBallotId id={MINE} data-testid="id" />)
        expect(container.textContent).toBe(
            "a41c 7e09 5b3d 28f6 c0e1 9d74 3a58 b2f0 6e1d 84c9 0b7f 5a23 d816 e4c2 79b0 13af"
        )
        expect(texts(container.querySelectorAll("b"))).toEqual(["a41c", "7e09"])
        expect(container.querySelector("mark")).toBeNull()
    })

    it("highlights the groups that differ from the ID it is compared with", () => {
        const {container} = render(<GroupedBallotId id={MINE} compareTo={DECODED} />)
        expect(texts(container.querySelectorAll("mark"))).toEqual([
            "5b3d",
            "28f6",
            "c0e1",
            "9d74",
            "3a58",
            "b2f0",
            "6e1d",
            "84c9",
            "0b7f",
            "5a23",
            "d816",
            "e4c2",
            "79b0",
            "13af",
        ])
        expect(texts(container.querySelectorAll("b"))).toEqual(["a41c", "7e09"])
    })

    it("highlights a single group that differs, and one in the short ID too", () => {
        const one = `${MINE.slice(0, 16)}ffff${MINE.slice(20)}`
        const {container, rerender} = render(<GroupedBallotId id={MINE} compareTo={one} />)
        expect(texts(container.querySelectorAll("mark"))).toEqual(["c0e1"])

        const short = `ffff${MINE.slice(4)}`
        rerender(<GroupedBallotId id={MINE} compareTo={short} />)
        expect(texts(container.querySelectorAll("mark"))).toEqual(["a41c"])
        expect(texts(container.querySelectorAll("b"))).toEqual(["a41c", "7e09"])
    })

    it("highlights what the other ID lacks, and compares without spacing or case", () => {
        const {container, rerender} = render(
            <GroupedBallotId id={MINE} compareTo={MINE.slice(0, 60)} />
        )
        expect(texts(container.querySelectorAll("mark"))).toEqual(["13af"])

        rerender(<GroupedBallotId id={MINE} compareTo={MINE.toUpperCase()} />)
        expect(container.querySelector("mark")).toBeNull()
    })
})
