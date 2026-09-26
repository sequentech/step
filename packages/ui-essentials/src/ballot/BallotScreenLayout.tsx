// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Typography, styled} from "@mui/material"
import React from "react"

import PageLimit from "../components/PageLimit/PageLimit"
import {theme} from "../services/theme"

/**
 * The election's name over the ballot.
 *
 * The portal's rule set, `margin-top: 25.5px` and all. Note it is a different one from
 * the ballot *list* screen's title in `ElectionListLayout`, which is 24px and
 * left-aligned; this one is 36px and centred.
 */
const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 36px;
    justify-content: center;
`

export interface IBallotScreenLayoutProps {
    /**
     * The breadcrumb, framed here 48px under the header — where the portal puts it,
     * in the `stepper-box` a client's stylesheet knows.
     */
    steps?: React.ReactNode
    /** The election's name. */
    title: React.ReactNode
    /**
     * Beside the name, inside the heading: the portal's help button and its dialog.
     */
    titleAdornment?: React.ReactNode
    /** The election's description, when it has one. Omitted entirely when it has not. */
    description?: React.ReactNode
    /** The contests. */
    children: React.ReactNode
    /** Back, Clear choices, Next — `BallotActions`. */
    actions?: React.ReactNode
}

/**
 * The screen where a voter marks their ballot.
 *
 * Lifted out of the portal's `VotingScreen` for the reason `ElectionListLayout` was
 * lifted out of `ElectionSelectionScreen`: the Election Architect's Ballot Preview has
 * to show *this* tree, with `voting-screen`, `stepper-box`, `title-container`,
 * `selected-election-title` and `description` where they are, because a client's
 * stylesheet is written against them. A preview that re-typed the markup would be
 * showing that CSS applied to something else.
 *
 * The contests themselves are `children`: the portal paginates them and a preview does
 * not, which is a difference in what is shown rather than in how the screen is built.
 */
export const BallotScreenLayout = ({
    steps,
    title,
    titleAdornment,
    description,
    children,
    actions,
}: IBallotScreenLayoutProps): React.JSX.Element => (
    <PageLimit maxWidth="lg" className="voting-screen screen">
        {steps === undefined ? null : (
            <Box marginTop="48px" className="stepper-box">
                {steps}
            </Box>
        )}

        <StyledTitle variant="h4" className="title-container">
            <Box className="selected-election-title">{title}</Box>
            {titleAdornment}
        </StyledTitle>

        {description === undefined ? null : (
            <Typography
                className="description"
                variant="body2"
                sx={{color: theme.palette.customGrey.main}}
            >
                {description}
            </Typography>
        )}

        {children}

        {actions}
    </PageLimit>
)
