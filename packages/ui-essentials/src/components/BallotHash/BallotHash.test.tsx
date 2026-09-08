/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import BallotHash, {copyBallotHash, CopyBallotHashStatus} from "./BallotHash"
import theme from "../../services/theme"

jest.mock("../LinkBehavior/LinkBehavior", () => "a")

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
    afterEach(() => {
        jest.useRealTimers()
        Reflect.deleteProperty(navigator, "clipboard")
    })

    it.each([":hover", ":active"])("keeps the copy button's box model unchanged on %s", (state) => {
        render(
            <ThemeProvider theme={theme}>
                <BallotHash hash="abc123" copyLabels={copyLabels} />
            </ThemeProvider>
        )
        const button = screen.getByRole("button", {name: copyLabels.copy})
        const restingStyle = getComputedStyle(button)
        const interactionStyle = document.createElement("button").style
        interactionStyle.padding = restingStyle.padding
        interactionStyle.border = restingStyle.border
        let matchedRules = 0

        // jsdom does not apply pointer pseudo-classes; resolve the matching
        // Emotion rules against the button's resting box model instead.
        for (const sheet of Array.from(document.styleSheets)) {
            for (const cssRule of Array.from(sheet.cssRules)) {
                const rule = cssRule as CSSStyleRule
                if (
                    rule.type === CSSRule.STYLE_RULE &&
                    rule.selectorText.includes(state) &&
                    button.matches(rule.selectorText.replaceAll(state, ""))
                ) {
                    matchedRules += 1
                    for (let index = 0; index < rule.style.length; index++) {
                        const property = rule.style[index]
                        interactionStyle.setProperty(
                            property,
                            rule.style.getPropertyValue(property)
                        )
                    }
                }
            }
        }

        expect(matchedRules).toBeGreaterThan(0)
        expect(interactionStyle.paddingBottom).toBe(restingStyle.paddingBottom)
        expect(interactionStyle.borderWidth).toBe(restingStyle.borderWidth)
    })

    it.each([undefined, copyLabels])("has no copy control for an empty hash (%j)", (labels) => {
        render(
            <ThemeProvider theme={theme}>
                <BallotHash hash="" copyLabels={labels} />
            </ThemeProvider>
        )
        expect(screen.queryByRole("button", {name: copyLabels.copy})).toBeNull()
    })

    it("keeps copying opt-in even when there is a hash", () => {
        render(
            <ThemeProvider theme={theme}>
                <BallotHash hash="abc123" />
            </ThemeProvider>
        )
        expect(screen.queryByRole("button", {name: copyLabels.copy})).toBeNull()
    })

    it("announces a complete copy and resets feedback after two seconds or a hash change", async () => {
        jest.useFakeTimers()
        const writeText = jest.fn().mockResolvedValue(undefined)
        Object.defineProperty(navigator, "clipboard", {configurable: true, value: {writeText}})
        const view = render(
            <ThemeProvider theme={theme}>
                <BallotHash hash="abc123" copyLabels={copyLabels} />
            </ThemeProvider>
        )

        await act(async () => fireEvent.click(screen.getByRole("button", {name: copyLabels.copy})))
        expect(writeText).toHaveBeenCalledWith("abc123")
        expect(screen.getByRole("status")).toHaveTextContent(copyLabels.copied)
        expect(screen.getByRole("button", {name: copyLabels.copied})).toBeInTheDocument()
        act(() => jest.advanceTimersByTime(2000))
        expect(screen.getByRole("status")).toBeEmptyDOMElement()

        await act(async () => fireEvent.click(screen.getByRole("button", {name: copyLabels.copy})))
        view.rerender(
            <ThemeProvider theme={theme}>
                <BallotHash hash="different-hash" copyLabels={copyLabels} />
            </ThemeProvider>
        )
        expect(screen.getByRole("status")).toBeEmptyDOMElement()
        expect(screen.getByRole("button", {name: copyLabels.copy})).toBeInTheDocument()
    })

    it.each([undefined, {writeText: jest.fn().mockRejectedValue(new Error("denied"))}])(
        "announces clipboard failures (%j)",
        async (clipboard) => {
            Object.defineProperty(navigator, "clipboard", {configurable: true, value: clipboard})
            render(
                <ThemeProvider theme={theme}>
                    <BallotHash hash="abc123" copyLabels={copyLabels} />
                </ThemeProvider>
            )
            await act(async () =>
                fireEvent.click(screen.getByRole("button", {name: copyLabels.copy}))
            )
            expect(screen.getByRole("status")).toHaveTextContent(copyLabels.error)
            expect(screen.getByRole("button", {name: copyLabels.error})).toBeInTheDocument()
        }
    )

    it("renders the optional copy control with an accessible name", () => {
        render(
            <ThemeProvider theme={theme}>
                <BallotHash
                    hash="abc123"
                    copyLabels={copyLabels}
                    helpButtonLabel="About ballot ID"
                />
            </ThemeProvider>
        )

        expect(screen.getByRole("button", {name: "Copy ballot ID"})).toBeInTheDocument()
        expect(screen.getByRole("button", {name: "About ballot ID"})).toBeInTheDocument()
        expect(screen.getByRole("status")).toBeInTheDocument()
    })

    it("uses the translated fallback when no help label is supplied", () => {
        render(
            <ThemeProvider theme={theme}>
                <BallotHash hash="abc123" />
            </ThemeProvider>
        )

        expect(screen.getByRole("button", {name: "About your Ballot ID"})).toBeInTheDocument()
    })
})
