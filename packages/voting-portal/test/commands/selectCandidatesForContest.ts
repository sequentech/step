// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {getRandomUniqueItems} from "../../src/utils/getRandomUniqueItems"
import {getRandomNumberBetween} from "../../src/utils/getRandomNumberBetween"

export const selectCandidatesForContest = (browser, contestItem) => {
    browser.elementIdText(Object.values(contestItem)[0] as string, function (contestTitle) {
        browser.elements(
            "xpath",
            `//h5[normalize-space()='${contestTitle.value}']/..//div[contains(@class, 'candidate-item')]`,
            async function (candidateList) {
                const minVotes = await browser.getAttribute(
                    `//h5[normalize-space()='${contestTitle.value}']`,
                    "data-min"
                )
                const maxVotes = await browser.getAttribute(
                    `//h5[normalize-space()='${contestTitle.value}']`,
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
                            `//h5[normalize-space()='${contestTitle.value}']/..//div[contains(@class, 'candidate-item')][${candidateIndex}]`
                        )
                })
            }
        )
    })
}
