// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Which of the encoder's complaints a voter is shown, and when.
 *
 * `filterErrorList` is a matrix of policy predicates; mounting the list on its own
 * makes every cell reachable without provoking real encoder errors. The portal
 * checks the same matrix through its redux adapter in
 * `voting-portal/src/components/InvalidErrorsList/`.
 */

import {aContest, anEngine, anError, marks, mountErrors} from "../testing/ballot"
import {contestErrorsId, writeInErrorId} from "./InvalidErrorsList"

const UNDER_VOTE = "errors.implicit.underVote"
const BLANK_VOTE = "errors.implicit.blankVote"
const OVER_VOTE_DISABLED = "errors.implicit.overVoteDisabled"
const SELECTED_MAX = "errors.implicit.selectedMax"

/** The rendered warnings, by the class the component derives from each message. */
const shown = (): string[] =>
    Array.from(document.querySelectorAll('[class*="warn--"]')).flatMap((node) =>
        Array.from(node.classList).filter((name) => name.startsWith("warn--"))
    )

const contest = (presentation: Record<string, unknown> = {}) => aContest({presentation} as never)

describe("before a voter has touched the contest", () => {
    it("says nothing at all, even when the encoder has complaints", () => {
        mountErrors(contest(), {
            alerts: [anError(UNDER_VOTE)],
            errors: [anError(SELECTED_MAX)],
            isTouched: false,
        })
        expect(shown()).toEqual([])
    })

    it("speaks once the contest has been touched", () => {
        mountErrors(contest(), {alerts: [anError(UNDER_VOTE)], isTouched: true})
        expect(shown().join(" ")).toContain("underVote")
    })

    it("counts a mark that arrived with the ballot as a touch", () => {
        const plain = contest()
        const mounted = mountErrors(plain, {selection: marks(plain, {a: 0})})
        expect(mounted.touched()).toBe(true)
    })

    it("does not call an unmarked contest touched", () => {
        expect(mountErrors(contest()).touched()).toBe(false)
    })
})

describe("warnings a policy defers to the review screen", () => {
    it.each([
        [{under_vote_policy: "warn-only-in-review"}, UNDER_VOTE],
        [{under_vote_policy: "warn-and-confirm-in-review"}, UNDER_VOTE],
        [{blank_vote_policy: "warn-only-in-review"}, BLANK_VOTE],
    ])("holds %o back while voting", (presentation, message) => {
        mountErrors(contest(presentation), {alerts: [anError(message)], isTouched: true})
        expect(shown()).toEqual([])
    })

    it.each([
        [{under_vote_policy: "warn-only-in-review"}, UNDER_VOTE],
        [{under_vote_policy: "warn-and-confirm-in-review"}, UNDER_VOTE],
        [{blank_vote_policy: "warn-only-in-review"}, BLANK_VOTE],
    ])("shows %o on review", (presentation, message) => {
        mountErrors(contest(presentation), {alerts: [anError(message)], isReview: true})
        expect(shown().length).toBe(1)
    })

    it("shows an under-vote warning while voting when the policy does not defer", () => {
        mountErrors(contest({under_vote_policy: "warn"}), {
            alerts: [anError(UNDER_VOTE)],
            isTouched: true,
        })
        expect(shown().length).toBe(1)
    })
})

describe("warnings that belong to one screen only", () => {
    it("drops the over-vote-disabled note on review, where it cannot be acted on", () => {
        mountErrors(contest(), {alerts: [anError(OVER_VOTE_DISABLED)], isReview: true})
        expect(shown()).toEqual([])
    })

    it("keeps it while voting, where it explains the greyed-out rows", () => {
        mountErrors(contest(), {alerts: [anError(OVER_VOTE_DISABLED)], isTouched: true})
        expect(shown().length).toBe(1)
    })
})

describe("two warnings that would say the same thing twice", () => {
    it("keeps blank and drops under-vote when both fire", () => {
        mountErrors(contest(), {
            alerts: [anError(UNDER_VOTE), anError(BLANK_VOTE)],
            isReview: true,
        })
        const classes = shown().join(" ")
        expect(classes).toContain("blankVote")
        expect(classes).not.toContain("underVote")
    })

    it("drops the selected-max note when it is already an alert", () => {
        mountErrors(contest(), {alerts: [anError(SELECTED_MAX)], isReview: true})
        expect(shown()).toEqual([])
    })
})

describe("errors, which are firmer than warnings", () => {
    it.each(["allowed", "allowed-with-exclusive-explicit"])(
        "suppresses an error where invalid ballots are %s",
        (policy) => {
            mountErrors(contest({invalid_vote_policy: policy}), {
                errors: [anError("errors.implicit.somethingElse")],
                isReview: true,
            })
            expect(shown()).toEqual([])
        }
    )

    it("still reports going over the limit, unless over-voting is allowed", () => {
        mountErrors(contest({invalid_vote_policy: "allowed", over_vote_policy: "not-allowed"}), {
            errors: [anError(SELECTED_MAX)],
            isReview: true,
        })
        expect(shown().join(" ")).toContain("selectedMax")
    })

    it("suppresses going over the limit where over-voting is allowed", () => {
        mountErrors(contest({invalid_vote_policy: "allowed", over_vote_policy: "allowed"}), {
            errors: [anError(SELECTED_MAX)],
            isReview: true,
        })
        expect(shown()).toEqual([])
    })

    it("still reports a blank ballot where blank is not allowed", () => {
        mountErrors(contest({invalid_vote_policy: "allowed", blank_vote_policy: "not-allowed"}), {
            errors: [anError(BLANK_VOTE)],
            isReview: true,
        })
        expect(shown().join(" ")).toContain("blankVote")
    })

    it("reports an error normally when invalid ballots are not allowed", () => {
        mountErrors(contest({invalid_vote_policy: "not-allowed"}), {
            errors: [anError(SELECTED_MAX)],
            isReview: true,
        })
        expect(shown().join(" ")).toContain("selectedMax")
        // The encoder's key is translated rather than shown raw.
        expect(document.body).toHaveTextContent("more than the maximum")
    })
})

describe("the list as a whole", () => {
    it("is the status region the contest's options point at", () => {
        mountErrors(contest())
        const region = document.getElementById(contestErrorsId("contest-1"))
        expect(region).toHaveAttribute("role", "status")
    })

    it("hands this contest's report up to the screen", () => {
        const plain = contest()
        const mounted = mountErrors(plain, {selection: marks(plain, {b: 0})})
        expect(mounted.decoded()).toMatchObject({contest_id: "contest-1"})
    })

    it("flags write-ins past the engine's character budget, with an id to point at", () => {
        const plain = contest()
        const typed = marks(plain, {a: 0})
        typed.choices[0].write_in_text = "TOO LONG"
        const mounted = mountErrors(plain, {
            selection: typed,
            hasWriteIns: true,
            engine: anEngine(2),
        })
        expect(mounted.invalidWriteIns()).toBe(true)
        expect(document.getElementById(writeInErrorId("contest-1"))).toHaveClass("write-in-error")
    })

    it("asks the engine nothing about write-ins in a contest without them", () => {
        const engine = anEngine()
        const spy = jest.spyOn(engine, "getWriteInAvailableCharacters")
        const mounted = mountErrors(contest(), {engine})
        expect(spy).not.toHaveBeenCalled()
        expect(mounted.invalidWriteIns()).toBeUndefined()
        expect(document.querySelector(".write-in-error")).toBeNull()
    })
})
