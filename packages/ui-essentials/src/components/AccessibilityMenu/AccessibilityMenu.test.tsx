/**
 * @jest-environment jsdom
 * @jest-environment-options {"url":"https://voting.example.org/"}
 */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, waitFor, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {createInstance} from "i18next"
import {I18nextProvider} from "react-i18next"
import {ThemeProvider} from "@mui/material/styles"
import {
    EVoterAccessibilitySettingsPolicy,
    USER_ACCESSIBILITY_COOKIE_NAME,
    getValueFromCookie,
    setCookie,
} from "@sequentech/ui-core"
import english from "../../../../ui-core/src/translations/en"
import theme from "../../services/theme"
import AccessibilityMenu from "./AccessibilityMenu"
import Header from "../Header/Header"

const root = document.documentElement

const withProviders = async (children: React.ReactNode) => {
    const i18n = createInstance()
    await i18n.init({lng: "en", resources: {en: {translation: english.translations}}})
    return (
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>{children}</ThemeProvider>
        </I18nextProvider>
    )
}

const mockMatchMedia = (matching: string[]) => {
    window.matchMedia = ((query: string) => ({
        matches: matching.includes(query),
    })) as unknown as typeof window.matchMedia
}

afterEach(() => {
    setCookie(USER_ACCESSIBILITY_COOKIE_NAME, "")
    Reflect.deleteProperty(window, "matchMedia")
})

it("lets a keyboard user change a setting, hear it, and get focus back", async () => {
    const user = userEvent.setup()
    render(await withProviders(<AccessibilityMenu />))

    const button = screen.getByRole("button", {name: "Accessibility"})
    expect(button).toHaveAttribute("aria-haspopup", "dialog")
    button.focus()
    await user.keyboard("{Enter}")

    const dialog = await screen.findByRole("dialog", {name: "Accessibility settings"})
    const textSize = within(dialog).getByRole("radiogroup", {name: "Text size"})
    expect(within(textSize).getByRole("radio", {name: "Default"})).toBeChecked()

    await user.click(within(textSize).getByRole("radio", {name: "Larger"}))
    expect(root).toHaveAttribute("data-a11y-text-size", "larger")
    expect(within(dialog).getByRole("status")).toHaveTextContent("Text size: Larger")
    expect(JSON.parse(getValueFromCookie(USER_ACCESSIBILITY_COOKIE_NAME) ?? "{}")).toEqual({
        textSize: "larger",
    })

    await user.click(within(dialog).getByRole("button", {name: "Close"}))
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())
    expect(button).toHaveFocus()
    expect(root).toHaveAttribute("data-a11y-text-size", "larger")
})

it("offers every setting as a named group of choices", async () => {
    const user = userEvent.setup()
    render(await withProviders(<AccessibilityMenu />))
    await user.click(screen.getByRole("button", {name: "Accessibility"}))
    const dialog = await screen.findByRole("dialog")

    const choices = (group: string) =>
        within(within(dialog).getByRole("radiogroup", {name: group}))
            .getAllByRole("radio")
            .map((radio) => radio.getAttribute("value"))
    expect(choices("Text size")).toEqual(["default", "large", "larger"])
    expect(choices("Contrast")).toEqual(["default", "high"])
    expect(choices("Text spacing")).toEqual(["default", "wide"])
    expect(choices("Motion")).toEqual(["default", "reduced"])

    await user.click(within(dialog).getByRole("radio", {name: "High contrast"}))
    await user.click(within(dialog).getByRole("radio", {name: "Wide"}))
    await user.click(within(dialog).getByRole("radio", {name: "Reduced"}))
    expect(root).toHaveAttribute("data-a11y-contrast", "high")
    expect(root).toHaveAttribute("data-a11y-text-spacing", "wide")
    expect(root).toHaveAttribute("data-a11y-motion", "reduced")
})

it("applies what the voter chose earlier as soon as it mounts", async () => {
    setCookie(USER_ACCESSIBILITY_COOKIE_NAME, JSON.stringify({contrast: "high"}))
    const {unmount} = render(await withProviders(<AccessibilityMenu />))
    expect(root).toHaveAttribute("data-a11y-contrast", "high")

    unmount()
    expect(root).not.toHaveAttribute("data-a11y-contrast")
})

it("follows the operating system until the voter chooses, and returns to it on reset", async () => {
    mockMatchMedia(["(prefers-reduced-motion: reduce)"])
    const user = userEvent.setup()
    render(await withProviders(<AccessibilityMenu />))
    expect(root).toHaveAttribute("data-a11y-motion", "reduced")

    await user.click(screen.getByRole("button", {name: "Accessibility"}))
    const dialog = await screen.findByRole("dialog")
    const motion = within(dialog).getByRole("radiogroup", {name: "Motion"})
    expect(within(motion).getByRole("radio", {name: "Reduced"})).toBeChecked()

    await user.click(within(motion).getByRole("radio", {name: "Default"}))
    await user.click(within(dialog).getByRole("radio", {name: "Large"}))
    expect(root).not.toHaveAttribute("data-a11y-motion")
    expect(root).toHaveAttribute("data-a11y-text-size", "large")

    await user.click(within(dialog).getByRole("button", {name: "Reset settings"}))
    expect(root).toHaveAttribute("data-a11y-motion", "reduced")
    expect(root).not.toHaveAttribute("data-a11y-text-size")
    expect(within(dialog).getByRole("status")).toHaveTextContent("Settings reset")
    expect(getValueFromCookie(USER_ACCESSIBILITY_COOKIE_NAME)).toBeUndefined()
})

it("is in the header only where the event enables it", async () => {
    const {rerender} = render(await withProviders(<Header />))
    expect(screen.queryByRole("button", {name: "Accessibility"})).not.toBeInTheDocument()

    rerender(
        await withProviders(
            <Header accessibilitySettingsPolicy={EVoterAccessibilitySettingsPolicy.DISABLED} />
        )
    )
    expect(screen.queryByRole("button", {name: "Accessibility"})).not.toBeInTheDocument()

    rerender(
        await withProviders(
            <Header accessibilitySettingsPolicy={EVoterAccessibilitySettingsPolicy.ENABLED} />
        )
    )
    expect(screen.getByRole("button", {name: "Accessibility"})).toBeVisible()
})
