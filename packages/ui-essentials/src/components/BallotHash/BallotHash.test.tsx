/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import BallotHash, {BallotHashCopyButton, copyBallotHash, CopyBallotHashStatus} from "./BallotHash"
import theme from "../../services/theme"

jest.mock("../LinkBehavior/LinkBehavior", () => "a")

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: {ballotId: string}): string =>
            key === "ballotHash"
                ? `Your Tracking ID: ${values?.ballotId}`
                : "About your Tracking ID",
    }),
}))

const copyLabels = {
    copy: "Copy tracking ID",
    copied: "Tracking ID copied",
    error: "Could not copy tracking ID",
}

describe("copyBallotHash", () => {
    it("writes the complete ballot hash to the clipboard", async () => {
        const writeText = jest.fn().mockResolvedValue(undefined)

        await expect(copyBallotHash("abc123", {writeText})).resolves.toBe(
            CopyBallotHashStatus.Copied
        )
        expect(writeText).toHaveBeenCalledWith("abc123")
    })

    describe("when the Clipboard API is refused or missing", () => {
        const setExecCommand = (execCommand: unknown) =>
            Object.defineProperty(document, "execCommand", {configurable: true, value: execCommand})

        afterEach(() => {
            Reflect.deleteProperty(document, "execCommand")
        })

        it("copies through a temporary text area", async () => {
            let selected = ""
            setExecCommand(
                jest.fn(() => {
                    selected = document.querySelector("textarea")?.value ?? ""
                    return true
                })
            )
            const writeText = jest.fn().mockRejectedValue(new Error("denied"))

            await expect(copyBallotHash("abc123", {writeText})).resolves.toBe(
                CopyBallotHashStatus.Copied
            )
            expect(selected).toBe("abc123")
            expect(document.querySelector("textarea")).toBeNull()
            await expect(copyBallotHash("abc123", undefined)).resolves.toBe(
                CopyBallotHashStatus.Copied
            )
        })

        it("asks for a manual copy when nothing works, and never reports an error", async () => {
            setExecCommand(jest.fn().mockReturnValue(false))
            await expect(copyBallotHash("abc123", undefined)).resolves.toBe(
                CopyBallotHashStatus.Manual
            )
            setExecCommand(
                jest.fn(() => {
                    throw new Error("unsupported")
                })
            )
            await expect(
                copyBallotHash("abc123", {
                    writeText: jest.fn().mockRejectedValue(new Error("denied")),
                })
            ).resolves.toBe(CopyBallotHashStatus.Manual)
            Reflect.deleteProperty(document, "execCommand")
            await expect(copyBallotHash("abc123", undefined)).resolves.toBe(
                CopyBallotHashStatus.Manual
            )
            expect(document.querySelector("textarea")).toBeNull()
        })
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
            const button = screen.getByRole("button", {name: copyLabels.error})
            // Nothing went wrong with the ballot: no warning sign.
            expect(button.querySelector("svg[data-icon='triangle-exclamation']")).toBeNull()
            expect(button.querySelector("svg[data-icon='copy']")).not.toBeNull()
        }
    )

    it("renders the optional copy control with an accessible name", () => {
        render(
            <ThemeProvider theme={theme}>
                <BallotHash
                    hash="abc123"
                    copyLabels={copyLabels}
                    helpButtonLabel="About tracking ID"
                />
            </ThemeProvider>
        )

        expect(screen.getByRole("button", {name: "Copy tracking ID"})).toBeInTheDocument()
        expect(screen.getByRole("button", {name: "About tracking ID"})).toBeInTheDocument()
        expect(screen.getByRole("status")).toBeInTheDocument()
    })

    it("uses the translated fallback when no help label is supplied", () => {
        render(
            <ThemeProvider theme={theme}>
                <BallotHash hash="abc123" />
            </ThemeProvider>
        )

        expect(screen.getByRole("button", {name: "About your Tracking ID"})).toBeInTheDocument()
    })
})

describe("BallotHashCopyButton", () => {
    const GREEN = "rgb(4, 120, 87)"

    afterEach(() => {
        Reflect.deleteProperty(navigator, "clipboard")
    })

    const copied = async (copiedColor?: string) => {
        const writeText = jest.fn().mockResolvedValue(undefined)
        Object.defineProperty(navigator, "clipboard", {configurable: true, value: {writeText}})
        render(
            <ThemeProvider theme={theme}>
                <BallotHashCopyButton
                    hash="abc123"
                    copyLabels={copyLabels}
                    copiedColor={copiedColor}
                />
            </ThemeProvider>
        )
        const resting = getComputedStyle(screen.getByRole("button", {name: copyLabels.copy})).color
        await act(async () => fireEvent.click(screen.getByRole("button", {name: copyLabels.copy})))
        const done = getComputedStyle(screen.getByRole("button", {name: copyLabels.copied})).color
        return {resting, done}
    }

    it("turns the check green once the ID is copied, when it is given that colour", async () => {
        const {resting, done} = await copied("#047857")
        expect(resting).not.toBe(GREEN)
        expect(done).toBe(GREEN)
    })

    it("takes the styles it is given over its own, and keeps its own otherwise", () => {
        const render1 = (sx?: Record<string, unknown>) => {
            const {unmount} = render(
                <ThemeProvider theme={theme}>
                    <BallotHashCopyButton hash="abc123" copyLabels={copyLabels} sx={sx} />
                </ThemeProvider>
            )
            const style = getComputedStyle(screen.getByRole("button", {name: copyLabels.copy}))
            const size = [style.width, style.height, style.paddingBottom]
            unmount()
            return size
        }
        expect(render1()).toEqual(["", "", "2px"])
        expect(render1({width: 34, height: 34, paddingBottom: 0})).toEqual(["34px", "34px", "0px"])
    })

    it("keeps its colour when it is given none", async () => {
        const {resting, done} = await copied()
        expect(done).toBe(resting)
    })

    it("keeps its colour while the copy failed, even when it is given a colour", async () => {
        Object.defineProperty(navigator, "clipboard", {
            configurable: true,
            value: {writeText: jest.fn().mockRejectedValue(new Error("denied"))},
        })
        render(
            <ThemeProvider theme={theme}>
                <BallotHashCopyButton hash="abc123" copyLabels={copyLabels} copiedColor="#047857" />
            </ThemeProvider>
        )
        await act(async () => fireEvent.click(screen.getByRole("button", {name: copyLabels.copy})))
        expect(
            getComputedStyle(screen.getByRole("button", {name: copyLabels.error})).color
        ).not.toBe(GREEN)
    })
})
