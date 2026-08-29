// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {ThemeProvider} from "@mui/material/styles"
import BreadCrumbSteps from "./BreadCrumbSteps"
import theme from "../../services/theme"

// BreadCrumbSteps calls `useTranslation()` to resolve the step labels; these
// tests assert on structure, so stub it to echo the key.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string): string => key}),
}))

const LABELS = ["steps.first", "steps.second", "steps.third"]

// The step colours come from the ui-essentials palette, which MUI's default
// theme does not carry.
const renderSteps = (element: React.ReactElement): string =>
    renderToStaticMarkup(<ThemeProvider theme={theme}>{element}</ThemeProvider>)

describe("BreadCrumbSteps", () => {
    it("exposes the steps as a named list", () => {
        const markup = renderSteps(
            <BreadCrumbSteps labels={LABELS} selected={1} ariaLabel="Voting progress" />
        )

        // The <ol> is styled list-style: none, which drops list semantics in
        // Safari and VoiceOver unless the role is stated explicitly.
        expect(markup).toContain('role="list"')
        expect(markup).toContain('aria-label="Voting progress"')
        expect(markup.match(/<li/g)).toHaveLength(LABELS.length)
    })

    it("marks the selected step as the current one", () => {
        const markup = renderSteps(<BreadCrumbSteps labels={LABELS} selected={1} />)

        expect(markup.match(/aria-current="step"/g)).toHaveLength(1)
    })

    it("hides the decorative separators from assistive technology", () => {
        const markup = renderSteps(<BreadCrumbSteps labels={LABELS} selected={0} />)

        // One separator per step except the last.
        expect(
            markup.match(
                /<div(?=[^>]*class="[^"]*step-separator)(?=[^>]*aria-hidden="true")[^>]*>/g
            )
        ).toHaveLength(LABELS.length - 1)
    })

    it("hides the decorative step numbers from assistive technology", () => {
        const markup = renderSteps(<BreadCrumbSteps labels={LABELS} selected={0} />)

        expect(
            markup.match(/<div(?=[^>]*class="[^"]*step-number)(?=[^>]*aria-hidden="true")[^>]*>/g)
        ).toHaveLength(LABELS.length)
    })

    it("omits the label when the consumer supplies none", () => {
        const markup = renderSteps(<BreadCrumbSteps labels={LABELS} selected={0} />)

        expect(markup).not.toContain("aria-label")
    })
})
