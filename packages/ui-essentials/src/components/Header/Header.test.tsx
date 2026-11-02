/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {createInstance} from "i18next"
import {I18nextProvider} from "react-i18next"
import {ThemeProvider} from "@mui/material/styles"
import {EVoterAccessibilitySettingsPolicy} from "@sequentech/ui-core"
import english from "../../../../ui-core/src/translations/en"
import theme from "../../services/theme"
import Header, {HeaderProps} from "./Header"

const view = async (props: HeaderProps) => {
    const i18n = createInstance()
    await i18n.init({lng: "en", resources: {en: {translation: english.translations}}})
    return render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Header logoUrl="logo.svg" appVersion={{main: "10.0.0"}} {...props} />
            </ThemeProvider>
        </I18nextProvider>
    )
}

/** The width from which an element is displayed; jsdom does not apply the media queries. */
const shownFrom = (element: Element) => {
    const name = Array.from(element.classList).find((className) => className.startsWith("css-"))
    const rule = new RegExp(
        `^@media \\(min-width:(\\d+)px\\) \\{\\.${name} \\{display: flex;\\}\\}$`
    )
    const css = Array.from(document.styleSheets).flatMap((sheet) =>
        Array.from(sheet.cssRules, ({cssText}) => cssText)
    )
    return Number(css.map((text) => rule.exec(text)?.[1]).find(Boolean))
}

it("shows no hash without one", async () => {
    const {container} = await view({})
    expect(container.querySelectorAll(".app-version")).toHaveLength(1)
    expect(container.querySelector(".app-version")).toHaveTextContent("Version:10.0.0")
})

it.each([
    ["from the same width as the version", false, theme.breakpoints.values.sm],
    ["on wide screens only", true, theme.breakpoints.values.md],
])("shows the hash %s", async (_case, appHashDesktopOnly, width) => {
    const {container} = await view({appHash: {main: "DEAD-BEEF"}, appHashDesktopOnly})
    const [version, hash] = Array.from(container.querySelectorAll(".app-version"))
    expect(hash).toHaveTextContent("Hash:DEAD-BEEF")
    expect(shownFrom(version)).toBe(theme.breakpoints.values.sm)
    expect(shownFrom(hash)).toBe(width)
})

it("puts its actions between the accessibility button and the language menu", async () => {
    await view({
        accessibilitySettingsPolicy: EVoterAccessibilitySettingsPolicy.ENABLED,
        actions: <button>Listen to the instructions</button>,
    })
    expect(within(screen.getByRole("banner")).getAllByRole("button")).toEqual([
        screen.getByRole("button", {name: "Accessibility"}),
        screen.getByRole("button", {name: "Listen to the instructions"}),
        screen.getByRole("button", {name: "Language: English"}),
    ])
})
