/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import {createInstance} from "i18next"
import {I18nextProvider} from "react-i18next"
import SelectElection from "./SelectElection/SelectElection"
import Footer from "./Footer/Footer"
import Version from "./Version/Version"
import theme from "../services/theme"

jest.mock("./LinkBehavior/LinkBehavior", () => "a")
jest.mock("@sequentech/ui-core", () => ({isUndefined: (value: unknown) => value === undefined}), {
    virtual: true,
})

const i18n = createInstance()

beforeAll(async () => {
    await i18n.init({
        lng: "en",
        resources: {
            en: {
                translation: {
                    "footer.poweredBy": "Powered by <1></1>",
                    "version.header": "Version:",
                    "hash.header": "Hash:",
                },
            },
            invalid: {translation: {"footer.poweredBy": "Invalid footer translation"}},
        },
    })
})

const renderWithTheme = (children: React.ReactNode) =>
    render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>{children}</ThemeProvider>
        </I18nextProvider>
    )

describe("static information semantics", () => {
    it("makes each ballot title a level-two heading within its list item", () => {
        renderWithTheme(
            <main>
                <h1>Ballot list</h1>
                <div role="list">
                    <SelectElection
                        title="Executive Board"
                        isActive
                        isOpen
                        isStarted
                        hasVoted={false}
                    />
                </div>
            </main>
        )

        const title = screen.getByRole("heading", {name: "Executive Board", level: 2})
        expect(title).toHaveClass("election-title")
        expect(within(screen.getByRole("listitem")).getByRole("heading")).toBe(title)
    })

    it("keeps footer attribution and its link without adding a heading", async () => {
        await i18n.changeLanguage("en")
        renderWithTheme(<Footer />)

        expect(screen.getByRole("contentinfo")).toHaveTextContent("Powered by Sequent Tech Inc")
        expect(screen.getByRole("link", {name: "Sequent Tech Inc"})).toHaveAttribute(
            "href",
            "//sequentech.io"
        )
        expect(screen.queryByRole("heading")).toBeNull()
    })

    it("does not turn invalid footer translation feedback into a heading", async () => {
        await i18n.changeLanguage("invalid")
        renderWithTheme(<Footer />)

        expect(screen.getByRole("contentinfo")).toHaveTextContent("Error: Invalid translation")
        expect(screen.queryByRole("heading")).toBeNull()
    })

    it("renders version and hash as static text without disabled controls or tabindex", async () => {
        await i18n.changeLanguage("en")
        const {container} = renderWithTheme(
            <>
                <Version version={{main: "10.0.0"}} />
                <Version header="hash.header" version={{main: "abcdef"}} />
            </>
        )

        const versions = container.querySelectorAll(".app-version")
        expect(versions).toHaveLength(2)
        expect(versions[0]).toHaveTextContent("Version:10.0.0")
        expect(versions[1]).toHaveTextContent("Hash:abcdef")
        expect(container.querySelector("button, [role=button], [tabindex], [disabled]")).toBeNull()
    })
})
