// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {getRandomUniqueItems} from "../../src/utils/getRandomUniqueItems"
import {getRandomNumberBetween} from "../../src/utils/getRandomNumberBetween"

// Matched on class rather than on element name: the contest title's heading level
// and the candidate row's element both follow the screen's document structure.
export const selectCandidatesForContest = (browser, contestItem) => {
    browser.elementIdText(Object.values(contestItem)[0] as string, function (contestTitle) {
        browser.elements(
            "xpath",
            `//*[contains(@class, 'contest-title')][normalize-space()='${contestTitle.value}']/..//*[contains(@class, 'candidate-item')]`,
            async function (candidateList) {
                const minVotes = await browser.getAttribute(
                    `//*[contains(@class, 'contest-title')][normalize-space()='${contestTitle.value}']`,
                    "data-min"
                )
                const maxVotes = await browser.getAttribute(
                    `//*[contains(@class, 'contest-title')][normalize-space()='${contestTitle.value}']`,
                    "data-max"
                )
                const numberOfChoices = getRandomNumberBetween(Number(minVotes), Number(maxVotes))

                const voterSelections = getRandomUniqueItems(
                    candidateList.value.map((_, i) => i + 1),
                    numberOfChoices
                )

                voterSelections.forEach(async (candidateIndex) => {
                    browser
                        .useXpath()
                        .click(
                            `//*[contains(@class, 'contest-title')][normalize-space()='${contestTitle.value}']/..//*[contains(@class, 'candidate-item')][${candidateIndex}]`
                        )
                })
            }
        )
    })
}
