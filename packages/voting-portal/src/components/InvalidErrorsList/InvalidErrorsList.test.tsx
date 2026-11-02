// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Which of the encoder's complaints a voter is actually shown.
 *
 * `filterErrorList` is eighty lines of interacting policy predicates written as
 * negations — its own comment explains that `!()` is used so the function reads
 * as "hide the error when this happens" rather than "show it when". It decides
 * whether somebody is told their ballot is blank, or under-voted, or over the
 * limit, and when. There was no test of it.
 *
 * Characterisation, like `Question.test.tsx`: this records today's answers so the
 * move into `@sequentech/ui-essentials` can be shown not to have changed them.
 * The combinations here are the ones the policies actually interact in, not every
 * product of the enums — a matrix of 4×4×4×5 would be a table nobody reads and
 * would pin the same three code paths forty times over.
 *
 * A warning's `warnId` becomes a class through `warnIdToClassName`, and per-event
 * CSS targets it, so the class is asserted rather than the sentence wherever the
 * sentence is long.
 */

import {screen} from "@testing-library/react"

import {anError, aContest, mountErrors} from "../../testing/ballotHarness"

const UNDER_VOTE = "errors.implicit.underVote"
const BLANK_VOTE = "errors.implicit.blankVote"
const OVER_VOTE_DISABLED = "errors.implicit.overVoteDisabled"
const SELECTED_MAX = "errors.implicit.selectedMax"

/** The rendered warnings, by the class the component derives from each message. */
const shown = (): string[] =>
    Array.from(document.querySelectorAll('[class*="warn--"]')).flatMap((node) =>
        Array.from(node.classList).filter((name) => name.startsWith("warn--"))
    )

describe("before a voter has touched the contest", () => {
    it("says nothing at all, even when the encoder has complaints", () => {
        // `!isReview && !isTouched` clears both lists wholesale. This is the rule
        // that stops a fresh ballot shouting "you have not voted" at somebody who
        // has just arrived, and it is worth pinning because it is the one branch
        // that discards *everything* rather than filtering.
        mountErrors(aContest(), {
            alerts: [anError(UNDER_VOTE)],
            errors: [anError(SELECTED_MAX)],
            isTouched: false,
        })

        expect(shown()).toEqual([])
    })

    it("speaks once the contest has been touched", () => {
        mountErrors(aContest(), {
            alerts: [anError(UNDER_VOTE)],
            isTouched: true,
        })

        expect(shown().length).toBeGreaterThan(0)
    })
})

describe("warnings a policy defers to the review screen", () => {
    it("holds back an under-vote warning while voting, under warn-only-in-review", () => {
        mountErrors(
            aContest({
                presentation: {under_vote_policy: "warn-only-in-review"},
            } as never),
            {alerts: [anError(UNDER_VOTE)], isTouched: true}
        )

        expect(shown()).toEqual([])
    })

    it("shows that same warning on the review screen", () => {
        mountErrors(
            aContest({
                presentation: {under_vote_policy: "warn-only-in-review"},
            } as never),
            {alerts: [anError(UNDER_VOTE)], isReview: true}
        )

        expect(shown().length).toBeGreaterThan(0)
    })

    it("holds back a blank-vote warning while voting, under warn-only-in-review", () => {
        mountErrors(
            aContest({
                presentation: {blank_vote_policy: "warn-only-in-review"},
            } as never),
            {alerts: [anError(BLANK_VOTE)], isTouched: true}
        )

        expect(shown()).toEqual([])
    })

    it("shows an under-vote warning while voting when the policy does not defer", () => {
        mountErrors(aContest({presentation: {under_vote_policy: "warn"}} as never), {
            alerts: [anError(UNDER_VOTE)],
            isTouched: true,
        })

        expect(shown().length).toBeGreaterThan(0)
    })
})

describe("warnings that belong to one screen only", () => {
    it("drops the over-vote-disabled note on review, where it cannot be acted on", () => {
        // The note exists to explain why the other options went grey while
        // voting. On the review screen there is nothing to click, so it is
        // filtered — regardless of policy.
        mountErrors(aContest(), {
            alerts: [anError(OVER_VOTE_DISABLED)],
            isReview: true,
        })

        expect(shown()).toEqual([])
    })

    it("keeps it while voting, where it explains the greyed-out rows", () => {
        mountErrors(aContest(), {
            alerts: [anError(OVER_VOTE_DISABLED)],
            isTouched: true,
        })

        expect(shown().length).toBeGreaterThan(0)
    })
})

describe("two warnings that would say the same thing twice", () => {
    it("keeps blank and drops under-vote when both fire", () => {
        // A blank ballot is also an under-voted one, so the encoder reports both.
        // Telling somebody twice is worse than telling them once: the specific
        // message survives and the general one goes.
        mountErrors(aContest(), {
            alerts: [anError(UNDER_VOTE), anError(BLANK_VOTE)],
            isReview: true,
        })

        const classes = shown().join(" ")
        expect(classes).toContain("blankVote")
        expect(classes).not.toContain("underVote")
    })

    it("keeps under-vote when it is the only complaint", () => {
        mountErrors(aContest(), {
            alerts: [anError(UNDER_VOTE)],
            isReview: true,
        })

        expect(shown().join(" ")).toContain("underVote")
    })
})

describe("errors, which are firmer than warnings", () => {
    it("suppresses an error where invalid ballots are allowed", () => {
        // `invalid_vote_policy: allowed` means a ballot the encoder dislikes may
        // still be cast, so most errors stop being errors. Recorded as behaviour,
        // not endorsed: the predicate is a triple negation and the two carve-outs
        // below are the reason it cannot simply be deleted.
        mountErrors(
            aContest({
                presentation: {invalid_vote_policy: "allowed"},
            } as never),
            {errors: [anError("errors.implicit.somethingElse")], isReview: true}
        )

        expect(shown()).toEqual([])
    })

    it("still reports going over the limit, even then", () => {
        // First carve-out: an over-vote survives `invalid_vote_policy: allowed`
        // whenever the over-vote policy itself is not `allowed`. Without this the
        // voter could cast a ballot the count would reject.
        mountErrors(
            aContest({
                presentation: {
                    invalid_vote_policy: "allowed",
                    over_vote_policy: "not-allowed",
                },
            } as never),
            {errors: [anError(SELECTED_MAX)], isReview: true}
        )

        expect(shown().join(" ")).toContain("selectedMax")
    })

    it("still reports a blank ballot where blank is not allowed", () => {
        // Second carve-out, same reasoning.
        mountErrors(
            aContest({
                presentation: {
                    invalid_vote_policy: "allowed",
                    blank_vote_policy: "not-allowed",
                },
            } as never),
            {errors: [anError(BLANK_VOTE)], isReview: true}
        )

        expect(shown().join(" ")).toContain("blankVote")
    })

    it("reports an error normally when invalid ballots are not allowed", () => {
        mountErrors(
            aContest({
                presentation: {invalid_vote_policy: "not-allowed"},
            } as never),
            {errors: [anError(SELECTED_MAX)], isReview: true}
        )

        expect(shown().join(" ")).toContain("selectedMax")
    })
})
