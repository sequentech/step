// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {ThemeProvider} from "@mui/material/styles"
import SelectElection from "./SelectElection"
import theme from "../../services/theme"

// ui-core resolves to its built `dist` via the workspace symlink, which isn't
// available when this package's tests run in isolation. SelectElection only needs
// `isUndefined` from it at runtime (`IElectionDates` is a type, erased at compile time).
// `virtual: true` lets the mock register even though the module can't be resolved;
// jest.mock is hoisted above the imports by babel-jest, so it applies before the import.
jest.mock(
    "@sequentech/ui-core",
    () => ({
        isUndefined: (value: unknown): boolean => value === undefined,
    }),
    {virtual: true}
)

// SelectElection calls `useTranslation()` for labels only; the test asserts on the
// injected date, not translated copy. Without a configured i18next instance the
// hook logs a NO_I18NEXT_INSTANCE warning, so stub it to echo the key.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string): string => key}),
}))

const ELECTION_DATES = {
    first_started_at: "2025-10-29T14:00:00.000Z",
}

const legacyFormat = (input: string): string =>
    new Intl.DateTimeFormat("en-GB", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
    }).format(new Date(input))

describe("SelectElection", () => {
    it("uses the injected formatDateTime when provided", () => {
        const markup = renderToStaticMarkup(
            <ThemeProvider theme={theme}>
                <SelectElection
                    isActive
                    isOpen
                    title="Executive Board"
                    hasVoted={false}
                    electionDates={ELECTION_DATES}
                    isStarted
                    formatDateTime={() => "CUSTOM_DATE"}
                />
            </ThemeProvider>
        )

        expect(markup).toContain("CUSTOM_DATE")
        expect(markup).not.toContain(legacyFormat(ELECTION_DATES.first_started_at))
    })

    it("falls back to the legacy GB format when formatDateTime is absent", () => {
        const markup = renderToStaticMarkup(
            <ThemeProvider theme={theme}>
                <SelectElection
                    isActive
                    isOpen
                    title="Executive Board"
                    hasVoted={false}
                    electionDates={ELECTION_DATES}
                    isStarted
                />
            </ThemeProvider>
        )

        expect(markup).toContain(legacyFormat(ELECTION_DATES.first_started_at))
        expect(markup).not.toContain("CUSTOM_DATE")
    })
})
