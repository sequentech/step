// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The portal's words, for the stories in this directory.
 *
 * These components translate `votingScreen.*`, `reviewScreen.*` and
 * `confirmationScreen.*`, which live in `voting-portal/src/translations/<lng>.ts` — so a
 * story outside that application has to supply them or it draws keys. Story data, and the
 * shortest set that makes each picture readable: the strings themselves belong to the
 * portal, and a full copy here would be `EA-F2-053` again.
 */

import i18next from "i18next"
import React from "react"
import {I18nextProvider} from "react-i18next"

const instance = i18next.createInstance()
void instance.init({
    lng: "en",
    fallbackLng: "en",
    resources: {
        en: {
            translation: {
                votingScreen: {
                    backButton: "Back",
                    clearButton: "Clear choices",
                    reviewButton: "Next",
                },
                reviewScreen: {
                    backButton: "Edit ballot",
                    auditButton: "Audit ballot",
                    castBallotButton: "Cast ballot",
                },
                confirmationScreen: {
                    printButton: "Print",
                    finishButton: "Finish",
                },
                candidate: {
                    moreInformationLink: "More information",
                },
                contest: {
                    paperBallot: {
                        voteForNoMoreThan_one: "Vote for no more than {{count}}",
                        voteForNoMoreThan_other: "Vote for no more than {{count}}",
                    },
                },
                a11y: {
                    selectUpTo_one: "Select up to {{count}} option",
                    selectUpTo_other: "Select up to {{count}} options",
                    selectExactly_one: "Select {{count}} option",
                    selectExactly_other: "Select {{count}} options",
                    selectBetween: "Select between {{min}} and {{max}} options",
                },
                breadcrumbSteps: {
                    electionList: "Ballots",
                    ballot: "Ballot",
                    review: "Review",
                    confirmation: "Confirm",
                },
            },
        },
    },
    interpolation: {escapeValue: false},
})

/** Wraps a story in that catalogue. */
export const withCatalogue = (Story: React.ComponentType): React.JSX.Element => (
    <I18nextProvider i18n={instance}>
        <Story />
    </I18nextProvider>
)
