// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {combineReducers, configureStore} from "@reduxjs/toolkit"
import {ThemeProvider} from "@mui/material/styles"
import {render} from "@testing-library/react"
import {theme} from "@sequentech/ui-essentials"
import type {IContest, IDecodedVoteContest} from "@sequentech/ui-core"
import {I18nextProvider} from "react-i18next"
import {Provider} from "react-redux"

import extraReducer from "../../store/extra/extraSlice"
import {aBallotStyle, aContest, i18n, marks} from "../../testing/ballotHarness"
import {InvalidErrorsList} from "./InvalidErrorsList"

const UNDER_VOTE = "errors.implicit.underVote"
const BLANK_VOTE = "errors.implicit.blankVote"

const contest = (presentation: Record<string, unknown>): IContest =>
    aContest({presentation} as Partial<IContest>)

const mountErrors = (
    question: IContest,
    message: string,
    {isReview = false, isTouched = isReview}: {isReview?: boolean; isTouched?: boolean}
) => {
    const state = {
        ...marks(question),
        invalid_alerts: [{message}],
    } as unknown as IDecodedVoteContest
    const store = configureStore({reducer: combineReducers({extra: extraReducer})})

    return render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Provider store={store}>
                    <InvalidErrorsList
                        ballotStyle={aBallotStyle(question)}
                        question={question}
                        hasWriteIns={false}
                        isInvalidWriteIns={false}
                        setIsInvalidWriteIns={() => undefined}
                        setDecodedContests={() => undefined}
                        isReview={isReview}
                        errorSelectionState={[state]}
                        isTouched={isTouched}
                        setIsTouched={() => undefined}
                    />
                </Provider>
            </ThemeProvider>
        </I18nextProvider>
    )
}

const shown = (): string[] =>
    Array.from(document.querySelectorAll('[class*="warn--"]')).flatMap((node) =>
        Array.from(node.classList).filter((name) => name.startsWith("warn--"))
    )

describe("warnings a policy defers to the review screen", () => {
    it.each([
        [{under_vote_policy: "warn-only-in-review"}, UNDER_VOTE],
        [{under_vote_policy: "warn-and-confirm-in-review"}, UNDER_VOTE],
        [{blank_vote_policy: "warn-only-in-review"}, BLANK_VOTE],
    ])("holds %o back while voting", (presentation, message) => {
        mountErrors(contest(presentation), message, {isTouched: true})
        expect(shown()).toEqual([])
    })

    it.each([
        [{under_vote_policy: "warn-only-in-review"}, UNDER_VOTE],
        [{under_vote_policy: "warn-and-confirm-in-review"}, UNDER_VOTE],
        [{blank_vote_policy: "warn-only-in-review"}, BLANK_VOTE],
    ])("shows %o on review", (presentation, message) => {
        mountErrors(contest(presentation), message, {isReview: true})
        expect(shown().length).toBe(1)
    })
})
