// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {MenuItem, Stack, TextField} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    EScopeSelector,
    type MonitoringScope,
    type MonitoringScopeOption,
    type MonitoringScopeOptions,
    type MonitoringSettingsView,
} from "./types"
import {selectorWords} from "./lib/scopeLabel"

export interface MonitoringSelectorsProps {
    selectors: EScopeSelector[]
    scope: MonitoringScope
    onChange: (scope: MonitoringScope) => void
    options: MonitoringScopeOptions
    settings?: MonitoringSettingsView
    /** The viewer sees only the Posts their permission labels allow. */
    restricted: boolean
    /** On an election's page its Post is fixed, and not offered. */
    pinnedPost?: string | null
}

/** Region · Post · Country, applying to every widget that follows them. */
export function MonitoringSelectors({
    selectors,
    scope,
    onChange,
    options,
    settings,
    restricted,
    pinnedPost,
}: MonitoringSelectorsProps) {
    const {t} = useTranslation()
    const shown = selectors.filter((selector) => !(selector === EScopeSelector.POST && pinnedPost))
    if (!shown.length) return null

    const choicesOf = (selector: EScopeSelector): MonitoringScopeOption[] => {
        switch (selector) {
            case EScopeSelector.REGION:
                return options.regions
            case EScopeSelector.POST:
                // Once a region is chosen, only its Posts.
                return options.posts.filter(
                    (post) => !scope.region || !post.region || post.region === scope.region
                )
            case EScopeSelector.COUNTRY:
                return options.countries
        }
    }

    const change = (selector: EScopeSelector, key: string) => {
        const next: MonitoringScope = {...scope}
        if (key) next[selector] = key
        else delete next[selector]
        if (selector === EScopeSelector.REGION && next.post && key) {
            const post = options.posts.find((option) => option.key === next.post)
            if (post?.region && post.region !== key) delete next.post
        }
        onChange(next)
    }

    return (
        <Stack direction={{xs: "column", sm: "row"}} spacing={2} useFlexGap flexWrap="wrap">
            {shown.map((selector) => {
                const words = selectorWords(selector, settings, t, restricted)
                const choices = choicesOf(selector)
                const value = scope[selector] ?? ""
                return (
                    <TextField
                        key={selector}
                        id={`monitoring-scope-${selector}`}
                        select
                        size="small"
                        label={words.label}
                        value={choices.some((choice) => choice.key === value) ? value : ""}
                        onChange={(event) => change(selector, event.target.value)}
                        // "All" is the empty value, and is shown as a choice.
                        slotProps={{select: {displayEmpty: true}, inputLabel: {shrink: true}}}
                        sx={{minWidth: 200}}
                    >
                        <MenuItem value="">{words.all}</MenuItem>
                        {choices.map((choice) => (
                            <MenuItem key={choice.key} value={choice.key}>
                                {choice.label}
                            </MenuItem>
                        ))}
                    </TextField>
                )
            })}
        </Stack>
    )
}
