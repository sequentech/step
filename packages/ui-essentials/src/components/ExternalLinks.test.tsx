/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import "@testing-library/jest-dom"
import Candidate from "./Candidate/Candidate"
import SelectElection from "./SelectElection/SelectElection"
import Header from "./Header/Header"
import theme from "../services/theme"

jest.mock("./LinkBehavior/LinkBehavior", () => "a")

jest.mock(
    "@sequentech/ui-core",
    () => ({
        isUndefined: (value: unknown): boolean => value === undefined,
        ECandidatesIconCheckboxPolicy: {
            SQUARE_CHECKBOX: "square-checkbox",
            ROUND_CHECKBOX: "round-checkbox",
        },
        EVotingPortalCountdownPolicy: {
            NO_COUNTDOWN: "NO_COUNTDOWN",
            COUNTDOWN: "COUNTDOWN",
            COUNTDOWN_WITH_ALERT: "COUNTDOWN_WITH_ALERT",
        },
    }),
    {virtual: true}
)

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => key,
        i18n: {language: "en", changeLanguage: jest.fn()},
    }),
}))

const renderWithTheme = (ui: React.ReactElement) =>
    render(<ThemeProvider theme={theme}>{ui}</ThemeProvider>)

const expectSafeNewTabLinks = (container: HTMLElement, expectedCount: number) => {
    const links = Array.from(container.querySelectorAll<HTMLAnchorElement>("a[target='_blank']"))
    expect(links).toHaveLength(expectedCount)
    for (const link of links) {
        const rel = (link.getAttribute("rel") ?? "").split(/\s+/)
        expect(rel).toEqual(expect.arrayContaining(["noopener", "noreferrer"]))
    }
}

it("opens the candidate more-info url without giving the new tab access to the voter's page", () => {
    const {container} = renderWithTheme(<Candidate title="Alice" url="https://example.com/alice" />)
    expectSafeNewTabLinks(container, 1)
})

it("opens the election website and results without giving the new tab access to the page", () => {
    const {container} = renderWithTheme(
        <SelectElection
            isActive
            isOpen
            isStarted
            hasVoted={false}
            title="Election"
            electionHomeUrl="https://example.com/election"
            resultsUrl="https://example.com/results"
        />
    )
    expectSafeNewTabLinks(container, 3)
})

it("opens the header logo link without giving the new tab access to the page", () => {
    const {container} = renderWithTheme(
        <Header logoLink="https://example.com" logoUrl="https://example.com/logo.png" />
    )
    expectSafeNewTabLinks(container, 1)
})
