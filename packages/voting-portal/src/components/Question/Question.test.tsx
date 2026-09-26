// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What a voter sees on one contest, pinned before it moves.
 *
 * These are characterisation tests: they describe what the ballot does **today**,
 * so that moving `Question` into `@sequentech/ui-essentials` — where the Election
 * Architect's preview will render the same component — can be shown not to have
 * changed it. They are not a specification. Where one looks like it is blessing
 * something odd, it is recording it, and the comment says so.
 *
 * This file had no predecessor, and that is the point. `Question` decides what
 * appears on a ballot and had no test of any kind: the package's jest config used
 * `testEnvironment: "node"` and matched only `*.test.ts`, so no React file could
 * be mounted or even collected. Verification was a person clicking through a
 * hand-built election.
 *
 * **Two things are deliberately pinned that look like implementation detail.**
 * The `candidates-*` class names, because `election_event_presentation.css` is
 * injected verbatim into the running portal — a client's own stylesheet targets
 * these, so renaming one silently breaks their ballot's appearance. And the
 * translation keys, because they are what a voter reads.
 *
 * **What no test here covers**, stated so a green run is not over-read: candidate
 * ordering, blank detection, the preferential predicate and the write-in budget
 * are WASM, stubbed in the harness, and covered by `cargo test -p sequent-core`
 * against the real encoder. See `../../testing/sequentCoreStub.ts`.
 *
 * All wiring lives in `../../testing/ballotHarness` so that the redux lift
 * changes that file and leaves the assertions below untouched.
 */

import {screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"

import {aCandidate, aContest, marks, mountContest} from "../../testing/ballotHarness"

/** A blank/invalid/write-in option is one flag on `presentation`. */
const option = (
    id: string,
    name: string,
    flag: "is_explicit_blank" | "is_explicit_invalid" | "is_write_in",
    extra: Record<string, unknown> = {}
) => aCandidate(id, name, {presentation: {[flag]: true, ...extra}} as never)

describe("a contest, as a voter meets it", () => {
    it("names itself, and says how many may be chosen", () => {
        // `data-min`/`data-max` are read by nothing in the app. They are there
        // for tests and for support staff reading a live page, and they are the
        // cheapest way to see the limits a ballot is enforcing.
        mountContest(aContest({min_votes: 1, max_votes: 3}))

        const title = screen.getByRole("heading", {name: "President"})
        expect(title).toHaveAttribute("data-min", "1")
        expect(title).toHaveAttribute("data-max", "3")
        expect(title).toHaveClass("contest-title")
    })

    it("labels the contest region by its own title", () => {
        // `aria-labelledby` on the section is how a screen reader announces which
        // contest it has entered. Easy to break by renaming an id, invisible when
        // broken.
        mountContest(aContest())
        expect(screen.getByRole("region", {name: "President"})).toBeInTheDocument()
    })

    it("wraps the options in a fieldset legend naming the contest", () => {
        // The legend is visually hidden and exists for assistive technology. A
        // group of checkboxes with no accessible group name is the commonest
        // ballot accessibility failure.
        mountContest(aContest())
        const legend = document.querySelector(".candidates-legend")
        expect(legend).not.toBeNull()
        expect(legend).toHaveTextContent("President")
    })

    it("draws every candidate, in the order the list arrives", () => {
        mountContest(
            aContest({
                candidates: [
                    aCandidate("a", "Alice Okonjo"),
                    aCandidate("b", "Bob Iyer"),
                    aCandidate("c", "Cara Bianchi"),
                ],
            })
        )

        // Asserted as it arrives, not as a sort: the ordering rule is Rust, and
        // the harness's stub is order-preserving precisely so this observes the
        // component's arrangement rather than the stub's opinion.
        const rows = screen
            .getAllByRole("checkbox")
            .map((box) => box.closest("li")?.textContent ?? "")
        expect(rows[0]).toContain("Alice Okonjo")
        expect(rows[1]).toContain("Bob Iyer")
        expect(rows[2]).toContain("Cara Bianchi")
    })

    it("shows a candidate's description under their name", () => {
        mountContest(
            aContest({
                candidates: [
                    aCandidate("a", "Alice Okonjo", {
                        description: "Steward, Local 12",
                    } as never),
                ],
            })
        )
        expect(screen.getByText("Steward, Local 12")).toBeInTheDocument()
    })

    it("shows the contest's own description when it has one", () => {
        mountContest(aContest({description: "One seat, three years."} as never))
        expect(screen.getByText("One seat, three years.")).toBeInTheDocument()
    })

    it("puts the plain candidates in the singles container", () => {
        mountContest(aContest())
        expect(document.querySelector(".candidates-singles-container")).not.toBeNull()
        expect(document.querySelector(".candidates-lists-container")).toBeNull()
    })
})

describe("marking a ballot", () => {
    it("starts with nothing marked", () => {
        mountContest(aContest())
        for (const box of screen.getAllByRole("checkbox")) {
            expect(box).not.toBeChecked()
        }
    })

    it("reflects a mark that arrived with the ballot", () => {
        // The review screen and a resumed ballot both arrive pre-marked.
        // `selected: 0` is the first preference; `-1` means unmarked — getting
        // that backwards produces a ballot that looks blank with every box
        // ticked, which is why the harness's `marks()` spells it out.
        const contest = aContest()
        mountContest(contest, {selection: marks(contest, {a: 0})})

        const boxes = screen.getAllByRole("checkbox")
        expect(boxes[0]).toBeChecked()
        expect(boxes[1]).not.toBeChecked()
    })

    it("takes a click on an option", async () => {
        const contest = aContest({max_votes: 2})
        mountContest(contest)

        await userEvent.click(screen.getAllByRole("checkbox")[0])
        expect(screen.getAllByRole("checkbox")[0]).toBeChecked()
    })

    it("disables the rest once the limit is reached, under the disabling policy", async () => {
        // `over_vote_policy: not-allowed-with-msg-and-disable` is the one policy
        // that reaches into the other rows: at `max_votes`, everything unselected
        // goes disabled. Pinned because it is cross-row state — `selectedChoicesSum`
        // lifted into `Question` — and the easiest thing to lose in a refactor.
        const contest = aContest({
            max_votes: 1,
            candidates: [aCandidate("a", "Alice Okonjo"), aCandidate("b", "Bob Iyer")],
            presentation: {over_vote_policy: "not-allowed-with-msg-and-disable"},
        } as never)
        mountContest(contest, {selection: marks(contest, {a: 0})})

        expect(screen.getAllByRole("checkbox")[1]).toBeDisabled()
    })

    it("leaves the others alone when the policy does not disable", () => {
        const contest = aContest({
            max_votes: 1,
            presentation: {over_vote_policy: "allowed"},
        } as never)
        mountContest(contest, {selection: marks(contest, {a: 0})})

        expect(screen.getAllByRole("checkbox")[1]).not.toBeDisabled()
    })
})

describe("the options that are not candidates", () => {
    it("draws a blank option above the candidates when it says top", () => {
        // Position is per candidate, not per contest, and the top block is a
        // separate container from the singles. Both are pinned: a blank option
        // that silently moves to the bottom changes what a hurried voter picks.
        mountContest(
            aContest({
                candidates: [
                    aCandidate("a", "Alice Okonjo"),
                    option("blank", "Blank vote", "is_explicit_blank", {
                        invalid_vote_position: "top",
                    }),
                ],
            })
        )

        const top = document.querySelector(".candidates-top-blank-invalid")
        expect(top).not.toBeNull()
        expect(top).toHaveTextContent("Blank vote")
    })

    it("draws a blank option below the candidates by default", () => {
        mountContest(
            aContest({
                candidates: [
                    aCandidate("a", "Alice Okonjo"),
                    option("blank", "Blank vote", "is_explicit_blank"),
                ],
            })
        )

        expect(document.querySelector(".candidates-top-blank-invalid")).toBeNull()
        expect(document.querySelector(".candidates-bottom-blank-invalid")).not.toBeNull()
    })

    it("offers a decline-to-vote option as its own row", () => {
        mountContest(
            aContest({
                candidates: [
                    aCandidate("a", "Alice Okonjo"),
                    option("spoil", "Decline to vote", "is_explicit_invalid"),
                ],
            })
        )
        expect(screen.getByText("Decline to vote")).toBeInTheDocument()
    })

    it("gives a write-in somewhere to type", () => {
        mountContest(
            aContest({
                presentation: {allow_writeins: true},
                candidates: [aCandidate("a", "Alice Okonjo"), option("w1", "", "is_write_in")],
            } as never)
        )
        expect(screen.getByRole("textbox")).toBeInTheDocument()
    })
})

describe("ranked contests", () => {
    it("offers a rank picker instead of a checkbox", () => {
        // The whole shape of the row changes on the counting algorithm: a
        // `Select` of ordinals rather than a checkbox. A preview that got this
        // wrong would show a voter the wrong ballot entirely.
        mountContest(
            aContest({
                counting_algorithm: "instant-runoff",
                max_votes: 2,
            } as never)
        )

        expect(screen.queryAllByRole("checkbox")).toHaveLength(0)
        expect(screen.getAllByRole("combobox").length).toBeGreaterThan(0)
    })
})

describe("grouped candidates", () => {
    const grouped = () =>
        aContest({
            candidates: [
                aCandidate("a", "Alice Okonjo", {candidate_type: "Blue Slate"} as never),
                aCandidate("b", "Bob Iyer", {candidate_type: "Blue Slate"} as never),
                aCandidate("c", "Cara Bianchi", {
                    candidate_type: "Green Slate",
                } as never),
            ],
        })

    it("puts each group in its own list, and names it", () => {
        mountContest(grouped())

        const lists = document.querySelector(".candidates-lists-container")
        expect(lists).not.toBeNull()
        expect(within(lists as HTMLElement).getByText("Blue Slate")).toBeInTheDocument()
        expect(within(lists as HTMLElement).getByText("Green Slate")).toBeInTheDocument()
    })

    it("offers one control to open or close every group, when they collapse", async () => {
        // `collapsible_lists: enabled-collapsed` is the case worth pinning: the
        // groups arrive shut, so the control reads "expand all" rather than
        // "collapse all". Getting that inverted is a ballot that looks empty.
        mountContest(
            aContest({
                ...grouped(),
                presentation: {collapsible_lists: "enabled-collapsed"},
            } as never)
        )

        expect(screen.getByRole("button", {name: /expand all/i})).toBeInTheDocument()
    })

    it("reads collapse-all when the groups arrive open", () => {
        mountContest(
            aContest({
                ...grouped(),
                presentation: {collapsible_lists: "enabled"},
            } as never)
        )

        expect(screen.getByRole("button", {name: /collapse all/i})).toBeInTheDocument()
    })

    it("offers no such control where the groups do not collapse", () => {
        mountContest(grouped())
        expect(screen.queryByRole("button", {name: /collapse all/i})).toBeNull()
        expect(screen.queryByRole("button", {name: /expand all/i})).toBeNull()
    })
})

describe("the review screen's rendering of the same contest", () => {
    it("offers no expand-all control, because nothing is being decided", () => {
        mountContest(
            aContest({
                candidates: [aCandidate("a", "Alice", {candidate_type: "Slate"} as never)],
                presentation: {collapsible_lists: "enabled"},
            } as never),
            {isReview: true}
        )

        expect(screen.queryByRole("button", {name: /collapse all/i})).toBeNull()
    })

    it("says the ballot is blank when nothing was marked", () => {
        // `checkIsBlank` is WASM; the harness's stub answers "blank when nothing
        // is selected", which is the real rule. What is pinned here is that the
        // screen *says so* — a blank ballot that looks identical to a marked one
        // is the failure this catches.
        const contest = aContest()
        mountContest(contest, {isReview: true, selection: marks(contest)})

        expect(document.querySelector(".candidates-review-blank")).not.toBeNull()
    })

    it("does not say blank when something was marked", () => {
        const contest = aContest()
        mountContest(contest, {
            isReview: true,
            selection: marks(contest, {a: 0}),
        })

        expect(document.querySelector(".candidates-review-blank")).toBeNull()
    })

    it("shows a declined ballot as declined, and draws no options", () => {
        const contest = aContest()
        mountContest(contest, {isReview: true, isDeclineToVote: true})

        expect(document.querySelector(".candidates-review-decline")).not.toBeNull()
        expect(screen.queryAllByRole("checkbox")).toHaveLength(0)
    })
})
