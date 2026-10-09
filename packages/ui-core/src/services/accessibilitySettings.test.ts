/**
 * @jest-environment jsdom
 * @jest-environment-options {"url":"https://voting.example.org/"}
 */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {afterEach, expect, it, jest} from "@jest/globals"
import {getValueFromCookie, setCookie} from "../utils/cookies"
import {
    DEFAULT_ACCESSIBILITY_SETTINGS,
    EAccessibilityContrast,
    EAccessibilityMotion,
    EAccessibilityTextSize,
    EAccessibilityTextSpacing,
    USER_ACCESSIBILITY_COOKIE_NAME,
    applyAccessibilitySettings,
    clearAccessibilitySettings,
    getSystemAccessibilitySettings,
    parseAccessibilitySettings,
    readStoredAccessibilitySettings,
    resolveAccessibilitySettings,
    storeAccessibilitySettings,
} from "./accessibilitySettings"

const mockMatchMedia = (matching: string[]) => {
    window.matchMedia = ((query: string) => ({
        matches: matching.includes(query),
    })) as unknown as typeof window.matchMedia
}

afterEach(() => {
    jest.restoreAllMocks()
    setCookie(USER_ACCESSIBILITY_COOKIE_NAME, "")
    clearAccessibilitySettings()
    Reflect.deleteProperty(window, "matchMedia")
})

it("parses the stored choices and drops what it does not recognise", () => {
    expect(
        parseAccessibilitySettings(
            JSON.stringify({textSize: "larger", contrast: "inverted", motion: "reduced", zoom: 3})
        )
    ).toEqual({
        textSize: EAccessibilityTextSize.LARGER,
        motion: EAccessibilityMotion.REDUCED,
    })
})

it.each([undefined, "", "not json", "null", "[]", '"large"'])(
    "treats %p as no stored choice",
    (raw) => {
        expect(parseAccessibilitySettings(raw)).toEqual({})
    }
)

it("round-trips the voter's choices through the cookie", () => {
    storeAccessibilitySettings({
        textSize: EAccessibilityTextSize.LARGE,
        textSpacing: EAccessibilityTextSpacing.WIDE,
    })
    expect(readStoredAccessibilitySettings()).toEqual({
        textSize: EAccessibilityTextSize.LARGE,
        textSpacing: EAccessibilityTextSpacing.WIDE,
    })
})

it("forgets the cookie when no choice is left", () => {
    storeAccessibilitySettings({contrast: EAccessibilityContrast.HIGH})
    storeAccessibilitySettings({})
    expect(getValueFromCookie(USER_ACCESSIBILITY_COOKIE_NAME)).toBeUndefined()
    expect(readStoredAccessibilitySettings()).toEqual({})
})

it("keeps working when the cookie cannot be written", () => {
    jest.spyOn(Document.prototype, "cookie", "set").mockImplementation(() => {
        throw new Error("cookies are blocked")
    })
    expect(() => storeAccessibilitySettings({contrast: EAccessibilityContrast.HIGH})).not.toThrow()
})

it("reads the operating system's contrast and motion preferences", () => {
    mockMatchMedia(["(prefers-contrast: more)", "(prefers-reduced-motion: reduce)"])
    expect(getSystemAccessibilitySettings()).toEqual({
        contrast: EAccessibilityContrast.HIGH,
        motion: EAccessibilityMotion.REDUCED,
    })
    mockMatchMedia([])
    expect(getSystemAccessibilitySettings()).toEqual({})
})

it("reports no system preference where matchMedia is missing", () => {
    expect(getSystemAccessibilitySettings()).toEqual({})
})

it("prefers the voter's choice over the system's, and the system's over the default", () => {
    expect(
        resolveAccessibilitySettings(
            {contrast: EAccessibilityContrast.DEFAULT, textSize: EAccessibilityTextSize.LARGE},
            {contrast: EAccessibilityContrast.HIGH, motion: EAccessibilityMotion.REDUCED}
        )
    ).toEqual({
        textSize: EAccessibilityTextSize.LARGE,
        contrast: EAccessibilityContrast.DEFAULT,
        textSpacing: EAccessibilityTextSpacing.DEFAULT,
        motion: EAccessibilityMotion.REDUCED,
    })
    expect(resolveAccessibilitySettings({}, {})).toEqual(DEFAULT_ACCESSIBILITY_SETTINGS)
})

it("exposes each non-default setting as an attribute on the root element", () => {
    const root = document.documentElement
    applyAccessibilitySettings({
        textSize: EAccessibilityTextSize.LARGER,
        contrast: EAccessibilityContrast.HIGH,
        textSpacing: EAccessibilityTextSpacing.WIDE,
        motion: EAccessibilityMotion.REDUCED,
    })
    expect(root.dataset.a11yTextSize).toBe("larger")
    expect(root.dataset.a11yContrast).toBe("high")
    expect(root.dataset.a11yTextSpacing).toBe("wide")
    expect(root.dataset.a11yMotion).toBe("reduced")

    applyAccessibilitySettings({
        ...DEFAULT_ACCESSIBILITY_SETTINGS,
        textSize: EAccessibilityTextSize.LARGE,
    })
    expect(root.dataset.a11yTextSize).toBe("large")
    expect(root.hasAttribute("data-a11y-contrast")).toBe(false)
    expect(root.hasAttribute("data-a11y-text-spacing")).toBe(false)
    expect(root.hasAttribute("data-a11y-motion")).toBe(false)

    clearAccessibilitySettings()
    expect(root.hasAttribute("data-a11y-text-size")).toBe(false)
})
