// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {ThemeProvider} from "@mui/material/styles"
import BallotHash, {copyBallotHash, CopyBallotHashStatus} from "./BallotHash"
import theme from "../../services/theme"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: {ballotId: string}): string =>
            key === "ballotHash" ? `Your Ballot ID: ${values?.ballotId}` : "About your Ballot ID",
    }),
}))

const copyLabels = {
    copy: "Copy ballot ID",
    copied: "Ballot ID copied",
    error: "Could not copy ballot ID",
}

describe("copyBallotHash", () => {
    it("writes the complete ballot hash to the clipboard", async () => {
        const writeText = jest.fn().mockResolvedValue(undefined)

        await expect(copyBallotHash("abc123", {writeText})).resolves.toBe(
            CopyBallotHashStatus.Copied
        )
        expect(writeText).toHaveBeenCalledWith("abc123")
    })

    it("returns an error when clipboard access is unavailable or rejected", async () => {
        await expect(copyBallotHash("abc123", undefined)).resolves.toBe(CopyBallotHashStatus.Error)
        await expect(
            copyBallotHash("abc123", {writeText: jest.fn().mockRejectedValue(new Error("denied"))})
        ).resolves.toBe(CopyBallotHashStatus.Error)
    })
})

describe("BallotHash", () => {
    it("renders the optional copy control with an accessible name", () => {
        const markup = renderToStaticMarkup(
            <ThemeProvider theme={theme}>
                <BallotHash
                    hash="abc123"
                    copyLabels={copyLabels}
                    helpButtonLabel="About ballot ID"
                />
            </ThemeProvider>
        )

        expect(markup).toContain('aria-label="Copy ballot ID"')
        expect(markup).toContain('aria-label="About ballot ID"')
        expect(markup).toContain('role="status"')
    })

    it("uses the translated fallback when no help label is supplied", () => {
        const markup = renderToStaticMarkup(
            <ThemeProvider theme={theme}>
                <BallotHash hash="abc123" />
            </ThemeProvider>
        )

        expect(markup).toContain('aria-label="About your Ballot ID"')
    })
})
