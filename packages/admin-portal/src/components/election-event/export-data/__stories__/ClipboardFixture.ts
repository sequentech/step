// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {spyOn} from "storybook/test"

/** Answers the page's clipboard writes, which succeed or are refused by the browser. */
export function storyClipboard(outcome: "copied" | "refused") {
    const writeText = spyOn(navigator.clipboard, "writeText").mockImplementation(async () => {
        if (outcome === "refused")
            throw new DOMException("Write permission denied", "NotAllowedError")
    })
    return {writeText, restore: () => writeText.mockRestore()}
}
