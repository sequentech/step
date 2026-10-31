// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * One contest, through the two ports a host supplies.
 *
 * The portal keeps its own copy of these behaviours in
 * `voting-portal/src/components/Question/`, mounted over its redux adapter. These
 * run in the package the ballot now lives in, over a plain port, so a change to
 * the ballot is checked where it is made and by the Election Architect's preview
 * as much as by the portal.
 */

import {act, fireEvent, render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"

import {
    aBallotStyle,
    aCandidate,
    aContest,
    anEngine,
    anOption,
    marks,
    mountContest,
} from "../testing/ballot"
import {Question} from "./Question"

const checkboxes = () => screen.getAllByRole("checkbox")

describe("a contest, as a voter meets it", () => {
    it("names itself, and says how many may be chosen", () => {
        mountContest(aContest({min_votes: 1, max_votes: 3}))

        const title = screen.getByRole("heading", {name: "President"})
        expect(title).toHaveAttribute("data-min", "1")
        expect(title).toHaveAttribute("data-max", "3")
        expect(screen.getByRole("region", {name: "President"})).toBeInTheDocument()
    })

    it.each([
        [1, 3, "President Select between 1 and 3 options"],
        [2, 2, "President Select 2 options"],
        [0, 2, "President Select up to 2 options"],
    ])("announces a %i..%i limit in the group's legend", (min, max, legend) => {
        mountContest(aContest({min_votes: min, max_votes: max}))
        expect(document.querySelector(".candidates-legend")).toHaveTextContent(legend)
    })

    it("draws every candidate in the order the engine returns", () => {
        const engine = anEngine()
        mountContest(
            aContest({
                candidates: [
                    aCandidate("a", "Alice Okonjo"),
                    aCandidate("b", "Bob Iyer"),
                    aCandidate("c", "Cara Bianchi"),
                ],
            }),
            {engine: {...engine, sortCandidatesInContest: (list) => [...list].reverse()}}
        )

        const rows = checkboxes().map((box) => box.closest("li")?.textContent ?? "")
        expect(rows[0]).toContain("Cara Bianchi")
        expect(rows[2]).toContain("Alice Okonjo")
    })

    it("shows the contest's and each candidate's description", () => {
        mountContest(
            aContest({
                description: "One seat, three years.",
                candidates: [aCandidate("a", "Alice Okonjo", {description: "Steward"} as never)],
            } as never)
        )
        expect(screen.getByText("One seat, three years.")).toBeInTheDocument()
        expect(screen.getByText("Steward")).toBeInTheDocument()
    })

    it("draws a candidate's image from the host's asset base", () => {
        mountContest(
            aContest({
                candidates: [
                    aCandidate("a", "Alice Okonjo", {
                        presentation: {urls: [{title: "p", url: "alice.png", is_image: true}]},
                    } as never),
                ],
            })
        )
        expect(document.querySelector(".candidate-image img, img.candidate-image")).toHaveAttribute(
            "src",
            "https://assets.example/alice.png"
        )
    })
})

describe("marking a ballot", () => {
    it("reflects a mark that arrived with the ballot", () => {
        const contest = aContest()
        mountContest(contest, {selection: marks(contest, {a: 0})})

        expect(checkboxes()[0]).toBeChecked()
        expect(checkboxes()[1]).not.toBeChecked()
    })

    it("takes a click, and a second click takes the mark away", async () => {
        const mounted = mountContest(aContest({max_votes: 2}))

        await userEvent.click(checkboxes()[0])
        expect(checkboxes()[0]).toBeChecked()
        expect(mounted.held().choices[0].selected).toBe(0)

        await userEvent.click(checkboxes()[0])
        expect(checkboxes()[0]).not.toBeChecked()
        expect(mounted.held().choices[0].selected).toBe(-1)
    })

    it("clears the contest before a radio-style choice", async () => {
        const contest = aContest({presentation: {candidates_selection_policy: "radio"}} as never)
        const mounted = mountContest(contest, {selection: marks(contest, {a: 0})})

        await userEvent.click(checkboxes()[1])

        expect(mounted.calls.map(([name]) => name)).toEqual(["reset", "setChoice"])
        expect(mounted.held().choices.map((choice) => choice.selected)).toEqual([-1, 0])
    })

    it("disables the rest once the limit is reached, and releases them again", async () => {
        const contest = aContest({
            presentation: {over_vote_policy: "not-allowed-with-msg-and-disable"},
        } as never)
        mountContest(contest, {selection: marks(contest, {a: 0})})

        expect(checkboxes()[1]).toBeDisabled()
        await userEvent.click(checkboxes()[0])
        expect(checkboxes()[1]).not.toBeDisabled()
    })

    it("leaves the others alone when the policy does not disable", () => {
        const contest = aContest({presentation: {over_vote_policy: "allowed"}} as never)
        mountContest(contest, {selection: marks(contest, {a: 0})})

        expect(checkboxes()[1]).not.toBeDisabled()
    })
})

describe("the options that are not candidates", () => {
    const withBlank = (position?: string) =>
        aContest({
            max_votes: 2,
            candidates: [
                aCandidate("a", "Alice Okonjo"),
                anOption("blank", "Blank vote", "is_explicit_blank", {
                    invalid_vote_position: position,
                }),
            ],
        })

    it("draws a blank option above the candidates when it says top", () => {
        mountContest(withBlank("top"))
        expect(document.querySelector(".candidates-top-blank-invalid")).toHaveTextContent(
            "Blank vote"
        )
        expect(document.querySelector(".candidates-bottom-blank-invalid")).toBeNull()
    })

    it("draws it below the candidates by default", () => {
        mountContest(withBlank())
        expect(document.querySelector(".candidates-top-blank-invalid")).toBeNull()
        expect(document.querySelector(".candidates-bottom-blank-invalid")).toHaveTextContent(
            "Blank vote"
        )
    })

    it("records an explicit blank as the only mark, and unmarks it", async () => {
        const contest = withBlank()
        const mounted = mountContest(contest, {selection: marks(contest, {a: 0})})
        const blank = () =>
            within(screen.getByText("Blank vote").closest("li")!).getByRole("checkbox")

        await userEvent.click(blank())
        expect(mounted.calls.at(-1)?.[0]).toBe("setBlank")
        expect(mounted.held().choices.map((choice) => choice.selected)).toEqual([-1, 0])

        await userEvent.click(blank())
        expect(mounted.held().choices.map((choice) => choice.selected)).toEqual([-1, -1])

        // A candidate chosen after a blank clears the blank's hold on the contest.
        await userEvent.click(blank())
        await userEvent.click(checkboxes()[0])
        expect(mounted.held().choices[0].selected).toBe(0)
    })

    it("marks the contest explicitly invalid through the port", async () => {
        const contest = aContest({
            candidates: [
                aCandidate("a", "Alice Okonjo"),
                anOption("spoil", "Spoil my ballot", "is_explicit_invalid", {
                    invalid_vote_position: "top",
                }),
            ],
        })
        const mounted = mountContest(contest)
        const spoil = () =>
            within(screen.getByText("Spoil my ballot").closest("li")!).getByRole("checkbox")

        await userEvent.click(spoil())
        expect(mounted.held().is_explicit_invalid).toBe(true)
        expect(spoil()).toBeChecked()

        await userEvent.click(spoil())
        expect(mounted.held().is_explicit_invalid).toBe(false)
    })

    const withWriteIn = () =>
        aContest({
            max_votes: 2,
            presentation: {allow_writeins: true},
            candidates: [aCandidate("a", "Alice Okonjo"), anOption("w1", "", "is_write_in")],
        } as never)

    it("takes a write-in, normalised the way the encoder stores it", () => {
        const mounted = mountContest(withWriteIn())

        fireEvent.change(screen.getByRole("textbox"), {target: {value: "Zoë"}})
        expect(mounted.held().choices[1]).toMatchObject({selected: -1, write_in_text: "ZOE"})
    })

    it("holds Next back while the encoder reports too many write-in characters", () => {
        const contest = withWriteIn()
        const typed = marks(contest, {w1: 0})
        typed.choices[1].write_in_text = "ZOE KIM"
        const mounted = mountContest(contest, {engine: anEngine(3), errors: [typed]})

        expect(mounted.nextDisabled()).toBe(true)
        expect(document.querySelector(".write-in-error")).not.toBeNull()
    })

    it("offers no field for a write-in slot where the contest takes none", () => {
        mountContest(
            aContest({candidates: [aCandidate("a", "Alice"), anOption("w1", "", "is_write_in")]})
        )
        expect(screen.queryByRole("textbox")).toBeNull()
    })
})

describe("ranked contests", () => {
    const ranked = () =>
        aContest({
            counting_algorithm: "instant-runoff",
            max_votes: 2,
            candidates: [aCandidate("a", "Alice Okonjo"), aCandidate("b", "Bob Iyer")],
        } as never)

    it("offers a rank picker instead of a checkbox, and records the rank", async () => {
        const mounted = mountContest(ranked())

        expect(screen.queryAllByRole("checkbox")).toHaveLength(0)
        await userEvent.click(screen.getAllByRole("combobox")[1])
        // The first entry is "no preference"; the next is first place.
        await userEvent.click((await screen.findAllByRole("option"))[1])

        expect(mounted.held().choices.map((choice) => choice.selected)).toEqual([-1, 0])
    })

    it("lists ranked choices in rank order on review, unranked last", () => {
        const contest = aContest({
            counting_algorithm: "instant-runoff",
            max_votes: 3,
            candidates: [
                aCandidate("a", "Alice Okonjo"),
                aCandidate("b", "Bob Iyer"),
                aCandidate("c", "Cara Bianchi"),
                aCandidate("d", "Dan Wu"),
            ],
        } as never)
        mountContest(contest, {isReview: true, selection: marks(contest, {c: 0, a: 1})})

        const text = document.querySelector(".candidates-singles-container")?.textContent ?? ""
        expect(text.indexOf("Cara Bianchi")).toBeLessThan(text.indexOf("Alice Okonjo"))
        expect(text).not.toContain("Bob Iyer")
    })
})

describe("grouped candidates", () => {
    const grouped = (presentation: Record<string, unknown> = {}) =>
        aContest({
            max_votes: 3,
            candidates: [
                aCandidate("blue", "Blue Slate", {
                    candidate_type: "Blue Slate",
                    presentation: {is_category_list: true},
                } as never),
                aCandidate("a", "Alice Okonjo", {
                    candidate_type: "Blue Slate",
                    presentation: {subtype: "Board"},
                } as never),
                aCandidate("b", "Bob Iyer", {candidate_type: "Blue Slate"} as never),
                aCandidate("c", "Cara Bianchi", {candidate_type: "Green Slate"} as never),
            ],
            presentation: {
                types_presentation: {
                    "Blue Slate": {subtypes_presentation: {Board: {sort_order: 1}}},
                },
                ...presentation,
            },
        } as never)

    it("puts each group in its own list, subtypes under their own heading", () => {
        mountContest(grouped())

        const lists = document.querySelector(".candidates-lists-container") as HTMLElement
        expect(within(lists).getByText("Blue Slate")).toBeInTheDocument()
        expect(within(lists).getByText("Green Slate")).toBeInTheDocument()
        expect(within(lists).getByRole("heading", {name: "Board"})).toBeInTheDocument()
        expect(document.querySelector(".candidates-singles-container")).toBeNull()
    })

    it("marks a whole list through its header", async () => {
        const mounted = mountContest(grouped())

        await userEvent.click(document.querySelector(".candidates-list-checkbox input")!)
        expect(mounted.calls.at(-1)?.[1]).toMatchObject({voteChoice: {id: "blue", selected: 0}})
        expect(document.querySelector(".candidates-list-checkbox input")).toBeChecked()
    })

    it("clears the contest before a radio-style list choice", async () => {
        const contest = {
            ...grouped({candidates_selection_policy: "radio"}),
            max_votes: 1,
        }
        const mounted = mountContest(contest)

        await userEvent.click(document.querySelector(".candidates-list-checkbox input")!)
        expect(mounted.calls.map(([name]) => name)).toEqual(["reset", "setChoice"])
    })

    it("counts a collapsed list's selections in its header", () => {
        const contest = grouped({collapsible_lists: "enabled-collapsed"})
        mountContest(contest, {selection: marks(contest, {a: 0, b: 0, c: 0})})

        expect(screen.getByText("2 candidates selected")).toBeInTheDocument()
        expect(screen.getByText("1 candidate selected")).toBeInTheDocument()
    })

    it("opens and closes every group from one control, when they collapse", async () => {
        mountContest(grouped({collapsible_lists: "enabled-collapsed"}))

        const toggle = screen.getByRole("button", {name: "Expand all"})
        expect(toggle).toHaveAttribute("aria-expanded", "false")
        await userEvent.click(toggle)
        expect(screen.getByRole("button", {name: "Collapse all"})).toHaveAttribute(
            "aria-expanded",
            "true"
        )
        await userEvent.click(screen.getByRole("button", {name: "Toggle list Green Slate"}))
        await userEvent.click(screen.getByRole("button", {name: "Toggle list Blue Slate"}))
        expect(screen.getByRole("button", {name: "Expand all"})).toBeInTheDocument()
    })

    it("offers no such control where the groups do not collapse", () => {
        mountContest(grouped())
        expect(screen.queryByRole("button", {name: /(collapse|expand) all/i})).toBeNull()
    })

    it("shows on review only the lists something was chosen from", () => {
        const contest = grouped({collapsible_lists: "enabled"})
        mountContest(contest, {isReview: true, selection: marks(contest, {c: 0})})

        expect(screen.queryByRole("button", {name: /collapse all/i})).toBeNull()
        expect(screen.getByText("Cara Bianchi")).toBeInTheDocument()
        expect(screen.queryByText("Alice Okonjo")).toBeNull()
    })

    it("shows a whole list on review when the list itself was chosen", () => {
        const contest = grouped()
        mountContest(contest, {isReview: true, selection: marks(contest, {blue: 0})})

        expect(screen.getByText("Alice Okonjo")).toBeInTheDocument()
        expect(screen.getByText("Bob Iyer")).toBeInTheDocument()
    })
})

describe("the review screen's rendering of the same contest", () => {
    it("says the contest is blank when nothing was marked", () => {
        const contest = aContest()
        mountContest(contest, {isReview: true})
        expect(document.querySelector(".candidates-review-blank")).not.toBeNull()
        expect(document.querySelector(".candidates-legend")).toHaveTextContent(/^President$/)
    })

    it("lists only what was marked", () => {
        const contest = aContest()
        mountContest(contest, {isReview: true, selection: marks(contest, {a: 0})})

        expect(document.querySelector(".candidates-review-blank")).toBeNull()
        expect(screen.getByText("Alice Okonjo")).toBeInTheDocument()
        expect(screen.queryByText("Bob Iyer")).toBeNull()
    })

    it("ignores clicks on review", async () => {
        const contest = aContest()
        const mounted = mountContest(contest, {isReview: true, selection: marks(contest, {a: 0})})
        for (const box of screen.queryAllByRole("checkbox")) {
            await userEvent.click(box)
        }
        expect(mounted.calls).toEqual([])
    })

    it("shows a declined ballot as declined, and draws no options", () => {
        mountContest(aContest(), {isReview: true, isDeclineToVote: true})
        expect(document.querySelector(".candidates-review-decline")).toHaveTextContent(
            "Declined to vote"
        )
        expect(screen.queryAllByRole("checkbox")).toHaveLength(0)
    })

    it("shows a blank ballot as blank", () => {
        mountContest(aContest(), {isReview: true, isBlankBallot: true})
        expect(document.querySelector(".candidates-review-blank-ballot")).toHaveTextContent(
            "Blank ballot"
        )
    })

    it("hands the encoder's report for this contest up to the screen", () => {
        const contest = aContest()
        const report = marks(contest, {a: 0})
        const mounted = mountContest(contest, {isReview: true, errors: [report]})
        expect(mounted.decoded()).toBe(report)
    })
})

describe("an acclaimed contest", () => {
    const acclaimed = (over: Record<string, unknown> = {}) =>
        aContest({
            is_acclaimed: true,
            max_votes: 2,
            candidates: [
                aCandidate("a", "Alice Okonjo"),
                anOption("w", "Write-in", "is_write_in"),
                anOption("x", "None of the above", "is_explicit_blank"),
            ],
            ...over,
        } as never)

    it("asks the engine which candidates it shows, and makes none selectable", async () => {
        const mounted = mountContest(acclaimed())

        expect(screen.getByText("Alice Okonjo")).toBeInTheDocument()
        expect(screen.queryByText("Write-in")).toBeNull()
        expect(screen.queryByText("None of the above")).toBeNull()
        expect(screen.getByText("Decided by acclamation")).toBeInTheDocument()
        for (const box of screen.queryAllByRole("checkbox")) {
            expect(box).toBeDisabled()
        }
        expect(mounted.calls).toEqual([])
    })

    it("uses the contest's own acclamation text, and is never declined", () => {
        mountContest(
            acclaimed({
                presentation: {i18n: {en: {acclamation_description: "Unopposed this year"}}},
            }),
            {isReview: true, isBlankBallot: true}
        )
        expect(screen.getByText("Unopposed this year")).toBeInTheDocument()
        expect(document.querySelector(".candidates-review-blank-ballot")).toBeNull()
        expect(document.querySelector(".candidates-review-blank")).toBeNull()
        expect(screen.getByText("Alice Okonjo")).toBeInTheDocument()
    })
})

describe("a host with no ports", () => {
    it("names the missing engine instead of drawing a ballot on guesses", () => {
        const contest = aContest()
        const spy = jest.spyOn(console, "error").mockImplementation(() => undefined)
        try {
            expect(() =>
                act(() => {
                    render(
                        <Question
                            ballotStyle={aBallotStyle(contest)}
                            question={contest}
                            isReview={false}
                            errorSelectionState={[]}
                            setDecodedContests={() => undefined}
                        />
                    )
                })
            ).toThrow(/No BallotEngine above this ballot, so isPreferential/)
        } finally {
            spy.mockRestore()
        }
    })
})
