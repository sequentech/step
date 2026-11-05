// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Link, Stack, ToggleButton, ToggleButtonGroup} from "@mui/material"
import type {PreviewScreen} from "voting-portal/src/preview/screens"

export interface ScreenLinks {
    /** Hash route of the workbench that reopens the bundled scenario at this screen. */
    deepLink: string
    storyId: string
    storyUrl: string
}

export interface ScreenNavProps {
    screens: readonly {screen: PreviewScreen; available: boolean}[]
    current?: PreviewScreen
    onOpen: (screen: PreviewScreen) => void
    links?: ScreenLinks
}

const title = (screen: PreviewScreen) => screen.charAt(0).toUpperCase() + screen.slice(1)

export const ScreenNav: React.FC<ScreenNavProps> = ({screens, current, onOpen, links}) => (
    <Stack
        component="nav"
        aria-label="Voter screens"
        direction="row"
        spacing={2}
        alignItems="center"
        flexWrap="wrap"
        useFlexGap
        className="screen-nav"
    >
        <ToggleButtonGroup exclusive size="small" value={current ?? null}>
            {screens.map(({screen, available}) => (
                <ToggleButton
                    key={screen}
                    value={screen}
                    disabled={!available}
                    onClick={() => onOpen(screen)}
                >
                    {title(screen)}
                </ToggleButton>
            ))}
        </ToggleButtonGroup>
        {links ? (
            <Box sx={{fontSize: 12, color: "text.secondary"}}>
                Link <code>#{links.deepLink}</code> · Story{" "}
                <Link href={links.storyUrl} target="_blank" rel="noreferrer">
                    {links.storyId}
                </Link>
            </Box>
        ) : null}
    </Stack>
)
