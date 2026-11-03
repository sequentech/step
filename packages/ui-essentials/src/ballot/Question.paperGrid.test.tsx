// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {ECandidatesLayout, EOverVotePolicy, ICandidate} from "@sequentech/ui-core"

import {aCandidate, aContest, anEngine, anOption, marks, mountContest} from "../testing/ballot"

const senators = (count: number): Array<ICandidate> =>
    Array.from({length: count}, (_, index) =>
        aCandidate(`s${index + 1}`, `Senator ${String(index + 1).padStart(3, "0")}`)
    )

const paperContest = (over: Record<string, unknown> = {}) =>
    aContest({
        name: "Senator",
        max_votes: 12,
        candidates: senators(66),
        presentation: {candidates_layout: ECandidatesLayout.PAPER_GRID},
        ...over,
    } as never)

const grid = () => document.querySelector(".candidates-paper-grid") as HTMLElement

describe("a contest drawn as a paper ballot", () => {
    beforeEach(() => sessionStorage.clear())

    it("keeps randomized candidate numbers on review and when returning to vote", async () => {
        const contest = paperContest({
            candidates: senators(3),
            presentation: {
                candidates_layout: ECandidatesLayout.PAPER_GRID,
                candidates_order: "random",
            },
        })
        const sort = jest.fn((candidates: ICandidate[]) => [...candidates].reverse())
        const engine = {...anEngine(), sortCandidatesInContest: sort}
        const mounted = mountContest(contest, {engine})
        await userEvent.click(screen.getByRole("checkbox", {name: "1. Senator 003"}))
        const selection = mounted.held()
        mounted.unmount()
        sort.mockImplementation((candidates) => candidates)
        const review = mountContest(contest, {engine, selection, isReview: true})
        expect(document.querySelector(".paper-candidate-name")).toHaveTextContent("1. Senator 003")
        review.unmount()
        mountContest(contest, {engine, selection})
        expect(screen.getByRole("checkbox", {name: "1. Senator 003"})).toBeChecked()
    })

    it("makes authored details available without changing the candidate's mark", async () => {
        const contest = paperContest({
            candidates: [
                aCandidate("a", "Alice", {
                    description: "A candidate statement",
                    presentation: {
                        urls: [
                            {title: "Photo", url: "alice.png", is_image: true},
                            {title: "URL", url: "https://example.com/alice", is_image: false},
                        ],
                    },
                }),
            ],
        })
        const mounted = mountContest(contest)
        const details = document.querySelector("details")!
        const summary = details.querySelector("summary")!
        await userEvent.click(summary)
        expect(details).toHaveAttribute("open")
        expect(screen.getByText("A candidate statement")).toBeVisible()
        expect(details.querySelector("img")).toHaveAttribute(
            "src",
            "https://assets.example/alice.png"
        )
        expect(screen.getByRole("link")).toHaveAttribute("href", "https://example.com/alice")
        expect(mounted.held().choices[0].selected).toBe(-1)
    })

    it.each([
        [2, 2, "Select 2 options"],
        [1, 3, "Select between 1 and 3 options"],
    ])("shows the %i..%i requirement in the visible heading", (min, max, instruction) => {
        mountContest(paperContest({min_votes: min, max_votes: max}))
        expect(document.querySelector(".contest-paper-subtitle")).toHaveTextContent(instruction)
    })

    it.each([{isReview: true}, {is_acclaimed: true}])(
        "does not instruct a voter to mark a read-only contest: %j",
        (mode) => {
            mountContest(paperContest(mode), {isReview: "isReview" in mode})
            expect(document.querySelector(".contest-paper-subtitle")).toBeNull()
        }
    )

    it("keeps the card list when no layout is configured", () => {
        mountContest(aContest())
        expect(grid()).toBeNull()
        expect(document.querySelector(".contest-paper-heading")).toBeNull()
        expect(document.querySelectorAll(".paper-candidate")).toHaveLength(0)
    })

    it("draws the heading bar with the selection limit, keeping the heading's name", () => {
        mountContest(paperContest())
        const heading = screen.getByRole("heading", {name: "Senator"})
        expect(heading.closest(".contest-paper-heading")).not.toBeNull()
        expect(screen.getByText("Vote for no more than 12")).toBeInTheDocument()
    })

    it("numbers every candidate in ballot order and names its checkbox", () => {
        mountContest(paperContest())
        const boxes = screen.getAllByRole("checkbox")
        expect(boxes).toHaveLength(66)
        expect(screen.getByRole("checkbox", {name: "1. Senator 001"})).toBeInTheDocument()
        expect(screen.getByRole("checkbox", {name: "66. Senator 066"})).toBeInTheDocument()
        expect(screen.getAllByRole("listitem")).toHaveLength(66)
    })

    it("marks a candidate on click and clears it on a second click", async () => {
        const user = userEvent.setup()
        const mounted = mountContest(paperContest())
        const box = screen.getByRole("checkbox", {name: "18. Senator 018"})

        await user.click(box)
        expect(box).toBeChecked()
        expect(box.closest("li")).toHaveClass("paper-candidate-checked")
        expect(mounted.held().choices.find((choice) => choice.id === "s18")?.selected).toBe(0)

        await user.click(box)
        expect(box).not.toBeChecked()
        expect(mounted.held().choices.find((choice) => choice.id === "s18")?.selected).toBe(-1)
    })

    it("can be marked from the keyboard", async () => {
        const user = userEvent.setup()
        mountContest(paperContest())
        await user.tab()
        expect(screen.getByRole("checkbox", {name: "1. Senator 001"})).toHaveFocus()
        await user.keyboard(" ")
        expect(screen.getByRole("checkbox", {name: "1. Senator 001"})).toBeChecked()
    })

    it("disables the rest once the limit is reached when the policy says so", async () => {
        const user = userEvent.setup()
        mountContest(
            paperContest({
                max_votes: 2,
                presentation: {
                    candidates_layout: ECandidatesLayout.PAPER_GRID,
                    over_vote_policy: EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE,
                },
            })
        )
        await user.click(screen.getByRole("checkbox", {name: "1. Senator 001"}))
        await user.click(screen.getByRole("checkbox", {name: "2. Senator 002"}))
        expect(screen.getByRole("checkbox", {name: "3. Senator 003"})).toBeDisabled()
        expect(screen.getByRole("checkbox", {name: "1. Senator 001"})).toBeEnabled()
    })

    it("fills column by column, with rows and density from the candidate count", () => {
        mountContest(paperContest())
        const style = getComputedStyle(grid())
        expect(style.gridAutoFlow).toBe("column")
        expect(style.gridTemplateRows).toBe("repeat(33, auto)")
    })

    it("lists only the marked candidates on review, with their ballot numbers", () => {
        const contest = paperContest()
        mountContest(contest, {isReview: true, selection: marks(contest, {s5: 0, s39: 0})})
        expect(screen.queryAllByRole("checkbox")).toHaveLength(0)
        const cells = Array.from(document.querySelectorAll(".paper-candidate"))
        expect(cells.map((cell) => cell.textContent)).toEqual(["5. Senator 005", "39. Senator 039"])
        expect(cells.every((cell) => cell.classList.contains("paper-candidate-checked"))).toBe(true)
    })

    it("keeps write-ins writable inside the grid", async () => {
        const user = userEvent.setup()
        const contest = paperContest({
            candidates: [...senators(3), anOption("w", "", "is_write_in")],
            presentation: {
                candidates_layout: ECandidatesLayout.PAPER_GRID,
                allow_writeins: true,
            },
        })
        const mounted = mountContest(contest)
        const field = screen.getByRole("textbox")
        await user.type(field, "Ana")
        expect(mounted.held().choices.find((choice) => choice.id === "w")?.write_in_text).toBe(
            "ANA"
        )
    })

    it("falls back to the card list for a ranked contest", () => {
        mountContest(paperContest({counting_algorithm: "instant-runoff", candidates: senators(3)}))
        expect(grid()).toBeNull()
        expect(document.querySelector(".contest-paper-heading")).toBeNull()
    })
})
