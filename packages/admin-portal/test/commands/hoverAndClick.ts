// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightWatchHoverAndClick} from ".."

exports.command = async function (el: NightWatchHoverAndClick) {
    // if el is an object
    if (typeof el === "object") {
        if (el.hoverElement.startsWith("//")) {
            this.useXpath().moveToElement(el.hoverElement, 10, 10)
        } else {
            this.useCss().moveToElement(el.hoverElement, 10, 10)
        }

        if (el.clickElement.startsWith("//")) {
            //is xpath selector
            this.useXpath().click(el.clickElement)
        } else {
            this.useCss().moveToElement(el.clickElement)
        }
        return this
    }

    this.moveToElement(el, 10, 10).click(el)

    return this
}
