// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Typography, styled} from "@mui/material"
import React from "react"

import {stringToHtml} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

import PageLimit from "../components/PageLimit/PageLimit"
import {theme} from "../services/theme"

/**
 * The heading of the ballot list screen.
 *
 * Both `margin-top`s are the portal's: the second wins, and the first is left in so
 * this is the same declaration block a client's stylesheet has been overriding.
 */
const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 24px;
    font-weight: 500;
    line-height: 27px;
    margin-top: 20px;
    margin-bottom: 16px;
`

const ElectionContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 30px;
    margin-bottom: 30px;
`

const TitleSection = styled(Box)`
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
    gap: 32px;
    min-height: 100px;

    @media (max-width: ${({theme}) => theme.breakpoints.values.sm}px) {
        flex-direction: column;
        align-items: stretch;
        gap: 16px;
        min-height: unset;
        padding: 24px 0;
    }
`

const PageActions = styled(Box)`
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: 16px;

    .election-event-results-button {
        min-width: 150px;
        padding: 10px 24px;
        justify-content: center;
        font-weight: 500;
        line-height: 24px;
        white-space: nowrap;
    }

    @media (max-width: ${({theme}) => theme.breakpoints.values.sm}px) {
        width: 100%;

        > .MuiButton-root {
            flex: 1;
        }
    }
`

export interface IElectionListLayoutProps {
    /**
     * The breadcrumb, built by whoever knows how many steps there are.
     *
     * Framed here rather than by the caller: 48px under the header is where the
     * portal puts it, and a preview that framed it itself got that wrong.
     */
    steps?: React.ReactNode
    /**
     * Beside the heading, inside it. The portal puts a help button and its dialog
     * here; a preview puts nothing.
     */
    titleAdornment?: React.ReactNode
    /**
     * Shown *instead of* the description. The portal replaces the description with a
     * warning when an election is misconfigured or closed, rather than stacking both.
     */
    alert?: React.ReactNode
    /** The buttons on the right: results, support materials. */
    actions?: React.ReactNode
    /** The elections themselves — one card each — or a line saying there are none. */
    children: React.ReactNode
}

/**
 * The screen a voter meets first: which ballot do you want to vote?
 *
 * Lifted out of the voting portal's `ElectionSelectionScreen`, which still renders
 * it — for the same reason as `StartLayout`, `ReviewLayout` and `ConfirmationLayout`
 * before it, and with one addition. Those three were lifted so the Election
 * Architect's Ballot Preview could draw the real screen; this one *also* carries four
 * class names a client's stylesheet targets — `election-selection-screen`,
 * `title-section`, `election-selection-heading`, `elections-list` — and a preview
 * that re-typed the markup around them would be showing that CSS applied to a
 * different tree. Elections have to be almost pixel-identical to what a voter sees,
 * so the tree is the contract, not just the components in it.
 *
 * `PageActions` is rendered whether or not there are actions, because the portal
 * renders it whether or not its two buttons are there.
 */
export const ElectionListLayout = ({
    steps,
    titleAdornment,
    alert,
    actions,
    children,
}: IElectionListLayoutProps): React.JSX.Element => {
    const {t} = useTranslation()

    return (
        <PageLimit maxWidth="lg" className="election-selection-screen screen">
            {steps === undefined ? null : <Box marginTop="48px">{steps}</Box>}

            <TitleSection className="title-section">
                <Box sx={{flex: 1, minWidth: 0}} className="election-selection-heading">
                    <StyledTitle variant="h1">
                        {/* `electionSelectionScreen.title`, translated here rather than
                        copied: this file carried an English pair for a while and the
                        wizard's preview read it instead of the catalogue. */}
                        <Box>{t("electionSelectionScreen.title")}</Box>
                        {titleAdornment}
                    </StyledTitle>
                    {alert ?? (
                        <Typography
                            variant="body1"
                            sx={{color: theme.palette.customGrey.contrastText}}
                        >
                            {stringToHtml(t("electionSelectionScreen.description"))}
                        </Typography>
                    )}
                </Box>
                <PageActions className="election-event-actions">{actions}</PageActions>
            </TitleSection>

            <ElectionContainer className="elections-list">{children}</ElectionContainer>
        </PageLimit>
    )
}
