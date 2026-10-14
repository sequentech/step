// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import AxeBuilder from "@axe-core/playwright"
import type {Page} from "@playwright/test"

export async function scanPage(page: Page) {
    const result = await new AxeBuilder({page})
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze()
    return result.violations.map(({id, impact, nodes}) => ({
        id,
        impact,
        targets: nodes.map(({target}) => target),
    }))
}
