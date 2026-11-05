// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {expect} from "storybook/test"

// The buttons that move the voter on stay at the bottom of the viewport while
// the rest of the page scrolls, so long pages never hide them.
export async function expectStickyActions(button: HTMLElement): Promise<void> {
    const actions = button.closest(".auth-actions")
    await expect(actions).not.toBeNull()
    const style = getComputedStyle(actions as Element)
    await expect(style.position).toBe("sticky")
    await expect(style.bottom).toBe("0px")
}
