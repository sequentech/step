// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {getValueFromCookie, setCookie} from "../utils/cookies"

export const USER_ACCESSIBILITY_COOKIE_NAME = "USER_ACCESSIBILITY"

export enum EAccessibilityTextSize {
    DEFAULT = "default",
    LARGE = "large",
    LARGER = "larger",
}

export enum EAccessibilityContrast {
    DEFAULT = "default",
    HIGH = "high",
}

export enum EAccessibilityTextSpacing {
    DEFAULT = "default",
    WIDE = "wide",
}

export enum EAccessibilityMotion {
    DEFAULT = "default",
    REDUCED = "reduced",
}

export interface IAccessibilitySettings {
    textSize: EAccessibilityTextSize
    contrast: EAccessibilityContrast
    textSpacing: EAccessibilityTextSpacing
    motion: EAccessibilityMotion
}

export const DEFAULT_ACCESSIBILITY_SETTINGS: IAccessibilitySettings = {
    textSize: EAccessibilityTextSize.DEFAULT,
    contrast: EAccessibilityContrast.DEFAULT,
    textSpacing: EAccessibilityTextSpacing.DEFAULT,
    motion: EAccessibilityMotion.DEFAULT,
}

/** The values each setting accepts, and the root attribute it is exposed through. */
const SETTINGS: {
    [K in keyof IAccessibilitySettings]: {values: string[]; attribute: string}
} = {
    textSize: {values: Object.values(EAccessibilityTextSize), attribute: "data-a11y-text-size"},
    contrast: {values: Object.values(EAccessibilityContrast), attribute: "data-a11y-contrast"},
    textSpacing: {
        values: Object.values(EAccessibilityTextSpacing),
        attribute: "data-a11y-text-spacing",
    },
    motion: {values: Object.values(EAccessibilityMotion), attribute: "data-a11y-motion"},
}

const SETTING_NAMES = Object.keys(SETTINGS) as (keyof IAccessibilitySettings)[]

/** Reads what the voter chose. Anything unrecognised is dropped, never an error. */
export const parseAccessibilitySettings = (raw?: string): Partial<IAccessibilitySettings> => {
    if (!raw) {
        return {}
    }
    let parsed: unknown
    try {
        parsed = JSON.parse(raw)
    } catch {
        return {}
    }
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        return {}
    }
    const stored = parsed as Record<string, unknown>
    const settings: Record<string, unknown> = {}
    for (const name of SETTING_NAMES) {
        const value = stored[name]
        if (typeof value === "string" && SETTINGS[name].values.includes(value)) {
            settings[name] = value
        }
    }
    return settings as Partial<IAccessibilitySettings>
}

export const readStoredAccessibilitySettings = (): Partial<IAccessibilitySettings> =>
    parseAccessibilitySettings(getValueFromCookie(USER_ACCESSIBILITY_COOKIE_NAME))

/**
 * Remembers the voter's explicit choices for the browser session.
 *
 * A cookie rather than `localStorage`, scoped like the language cookie, so the login pages
 * read the same choices. Where cookies are blocked the settings still apply to the open page.
 */
export const storeAccessibilitySettings = (settings: Partial<IAccessibilitySettings>) => {
    const value = Object.keys(settings).length === 0 ? "" : JSON.stringify(settings)
    try {
        setCookie(USER_ACCESSIBILITY_COOKIE_NAME, value)
    } catch {
        // Storage is a convenience: the settings are already applied.
    }
}

const prefers = (query: string): boolean =>
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia(query).matches

/** What the operating system asks for, used until the voter chooses. */
export const getSystemAccessibilitySettings = (): Partial<IAccessibilitySettings> => {
    const settings: Partial<IAccessibilitySettings> = {}
    if (prefers("(prefers-contrast: more)")) {
        settings.contrast = EAccessibilityContrast.HIGH
    }
    if (prefers("(prefers-reduced-motion: reduce)")) {
        settings.motion = EAccessibilityMotion.REDUCED
    }
    return settings
}

export const resolveAccessibilitySettings = (
    chosen: Partial<IAccessibilitySettings>,
    system: Partial<IAccessibilitySettings>
): IAccessibilitySettings => ({
    ...DEFAULT_ACCESSIBILITY_SETTINGS,
    ...system,
    ...chosen,
})

/**
 * Exposes the settings as `data-a11y-*` attributes, which the shared stylesheet and any
 * event stylesheet select on. A default value removes its attribute.
 */
export const applyAccessibilitySettings = (
    settings: IAccessibilitySettings,
    root: HTMLElement = document.documentElement
) => {
    for (const name of SETTING_NAMES) {
        const {attribute} = SETTINGS[name]
        if (settings[name] === DEFAULT_ACCESSIBILITY_SETTINGS[name]) {
            root.removeAttribute(attribute)
        } else {
            root.setAttribute(attribute, settings[name])
        }
    }
}

export const clearAccessibilitySettings = (root: HTMLElement = document.documentElement) =>
    applyAccessibilitySettings(DEFAULT_ACCESSIBILITY_SETTINGS, root)
