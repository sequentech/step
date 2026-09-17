/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {createInstance} from "i18next"
import {I18nextProvider} from "react-i18next"
import {ThemeProvider} from "@mui/material/styles"
import theme from "../../services/theme"
import LanguageMenu from "./LanguageMenu"
import LanguageSetter from "../LanguageSetter/LanguageSetter"

it("uses the theme text color and commits a language before notifying the caller", async () => {
    const user = userEvent.setup()
    const i18n = createInstance()
    await i18n.init({
        lng: "en",
        resources: {
            en: {translation: {language: "English"}},
            fr: {translation: {language: "Français"}},
        },
    })
    const onChange = jest.fn((language: string) => expect(i18n.language).toBe(language))
    render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <LanguageMenu languagesList={["en", "fr"]} onChange={onChange} />
            </ThemeProvider>
        </I18nextProvider>
    )
    const button = screen.getByRole("button", {name: "English"})
    expect(getComputedStyle(button).color).toBe("rgb(0, 0, 0)")
    await user.click(button)
    expect(button).toHaveAttribute("aria-expanded", "true")
    await user.click(screen.getByRole("menuitem", {name: "Français"}))
    await waitFor(() => expect(onChange).toHaveBeenCalledWith("fr"))
    expect(screen.getByRole("button", {name: "Français"})).not.toHaveAttribute("aria-expanded")
})

it("changes the inherited language when the configured language changes", async () => {
    const i18n = createInstance()
    await i18n.init({lng: "en", resources: {}})
    const content = (language: string) => (
        <I18nextProvider i18n={i18n}>
            <LanguageSetter language={language}>
                <span>Ballot</span>
            </LanguageSetter>
        </I18nextProvider>
    )
    const {rerender} = render(content("fr"))
    await waitFor(() => expect(i18n.language).toBe("fr"))
    expect(screen.getByText("Ballot")).toBeVisible()
    rerender(content("en"))
    await waitFor(() => expect(i18n.language).toBe("en"))
})
