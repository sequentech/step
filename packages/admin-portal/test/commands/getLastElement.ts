// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"

export const getLastElement = async function ({browser, el}: {browser: NightwatchAPI; el: string}) {
    const resultElement = await browser.element.findAll(el)

    return resultElement[resultElement.length - 1]
}
