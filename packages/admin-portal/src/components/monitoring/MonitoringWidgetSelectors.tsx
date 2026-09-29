// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {MenuItem, Stack, TextField, ToggleButton, ToggleButtonGroup} from "@mui/material"
import {ESelectorControl} from "./types"
import {ESelectorState, type ResolvedSelector} from "./lib/selectors"

export interface MonitoringWidgetSelectorsProps {
    widgetId: string
    selectors: ResolvedSelector[]
    onChange: (name: string, value: string) => void
}

/** The widget's own selectors, in declared order; one whose `when` does not hold is not shown. */
export function MonitoringWidgetSelectors({
    widgetId,
    selectors,
    onChange,
}: MonitoringWidgetSelectorsProps) {
    const shown = selectors.filter(({state}) => state.kind === ESelectorState.VALUE)
    if (!shown.length) return null
    return (
        <Stack direction="row" flexWrap="wrap" useFlexGap spacing={1}>
            {shown.map(({name, selector, options, state}) => {
                const value = state.kind === ESelectorState.VALUE ? state.value : ""
                if (selector.control === ESelectorControl.TOGGLE) {
                    return (
                        <ToggleButtonGroup
                            key={name}
                            size="small"
                            exclusive
                            value={value}
                            aria-label={selector.label}
                            onChange={(_event, selected: string | null) => {
                                if (selected !== null) onChange(name, selected)
                            }}
                        >
                            {options.map((option) => (
                                <ToggleButton key={option.value} value={option.value}>
                                    {option.label}
                                </ToggleButton>
                            ))}
                        </ToggleButtonGroup>
                    )
                }
                return (
                    <TextField
                        key={name}
                        id={`monitoring-${widgetId}-${name}`}
                        select
                        size="small"
                        label={selector.label}
                        value={value}
                        onChange={(event) => onChange(name, event.target.value)}
                        sx={{minWidth: 140}}
                    >
                        {options.map((option) => (
                            <MenuItem key={option.value} value={option.value}>
                                {option.label}
                            </MenuItem>
                        ))}
                    </TextField>
                )
            })}
        </Stack>
    )
}
