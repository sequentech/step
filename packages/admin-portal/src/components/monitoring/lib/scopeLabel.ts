// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EScopeSelector,
    type MonitoringScope,
    type MonitoringScopeOption,
    type MonitoringScopeOptions,
    type MonitoringSelectorWords,
    type MonitoringSettingsView,
} from "../types"

export type Translate = (key: string, options?: Record<string, unknown>) => string

const PORTAL_WORDS: Record<EScopeSelector, {label: string; all: string}> = {
    [EScopeSelector.REGION]: {
        label: "monitoring.selectors.region",
        all: "monitoring.selectors.allRegions",
    },
    [EScopeSelector.POST]: {
        label: "monitoring.selectors.post",
        all: "monitoring.selectors.allPosts",
    },
    [EScopeSelector.COUNTRY]: {
        label: "monitoring.selectors.country",
        all: "monitoring.selectors.allCountries",
    },
}

/**
 * What a dashboard selector is called: the settings' words (a university may
 * say "Faculty"), else the portal's. A viewer limited by permission labels sees
 * only some Posts, so their "All" says so.
 */
export function selectorWords(
    selector: EScopeSelector,
    settings: MonitoringSettingsView | undefined,
    t: Translate,
    restricted: boolean
): MonitoringSelectorWords {
    const configured = settings?.selectors?.[selector]
    const words = configured ?? {
        label: t(PORTAL_WORDS[selector].label),
        all: t(PORTAL_WORDS[selector].all),
    }
    if (selector !== EScopeSelector.POST || !restricted) return words
    return {
        label: words.label,
        all: configured
            ? t("monitoring.selectors.authorizedOnly", {all: configured.all})
            : t("monitoring.selectors.allAuthorizedPosts"),
    }
}

const optionsOf = (
    options: MonitoringScopeOptions,
    selector: EScopeSelector
): MonitoringScopeOption[] => {
    switch (selector) {
        case EScopeSelector.REGION:
            return options.regions
        case EScopeSelector.POST:
            return options.posts
        case EScopeSelector.COUNTRY:
            return options.countries
    }
}

export function optionLabel(
    options: MonitoringScopeOptions,
    selector: EScopeSelector,
    key: string
): string {
    return optionsOf(options, selector).find((option) => option.key === key)?.label ?? key
}

/** `All regions · Madrid · Spain`: the scope a dashboard or export is shown at. */
export function scopeLabel({
    scope,
    selectors,
    options,
    settings,
    t,
    restricted,
    pinnedPost,
}: {
    scope: MonitoringScope
    selectors: EScopeSelector[]
    options: MonitoringScopeOptions
    settings?: MonitoringSettingsView
    t: Translate
    restricted: boolean
    pinnedPost?: string | null
}): string {
    return selectors
        .map((selector) => {
            const key =
                selector === EScopeSelector.POST && pinnedPost ? pinnedPost : scope[selector]
            return key
                ? optionLabel(options, selector, key)
                : selectorWords(selector, settings, t, restricted).all
        })
        .join(" · ")
}
