// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {faAngleLeft, faAngleRight} from "@fortawesome/free-solid-svg-icons"
import {Box} from "@mui/material"
import React from "react"

import {useTranslation} from "react-i18next"

import Icon from "../components/Icon/Icon"
import {ActionsContainer, StyledButton} from "../components/ActionsRow/ActionsRow"

export interface IBallotActionsProps {
    /**
     * What to render the Back control as, when it navigates by link, with `backTo`
     * as its destination. Back is then the link itself rather than a button inside
     * one, so it stays a single keyboard stop. Without it Back is a button and
     * `onBack` does the navigating, which is what the portal does.
     */
    backComponent?: React.ElementType
    /** Where Back goes, for `backComponent`. */
    backTo?: string | object
    /** Back was pressed. The portal steps its contest pagination and navigates here. */
    onBack?: () => void
    /** Clear every choice on this ballot. */
    onClear?: () => void
    /** On to the review screen. */
    onNext?: () => void
    /** Next is refused while a contest is over-voted. */
    disableNext?: boolean
    /**
     * Draw the row, refuse to work it.
     *
     * For a preview: these are the buttons a voter will meet, but there is no ballot
     * to clear and nowhere to go next, and a control that looks live and does nothing
     * is worse than one that is plainly disabled.
     */
    inert?: boolean
}

/**
 * The row of buttons under the ballot: Back, Clear choices, Next.
 *
 * Lifted out of the portal's `VotingScreen` so the Election Architect's Ballot Preview
 * draws this row rather than one invented for it — which is how the preview came to
 * show a "Back" with no chevron and no Clear button at all.
 *
 * Its three words are `votingScreen.*`, read here rather than handed in: this file
 * carried them in English for a while and the wizard read the copy, so a Spanish preview
 * said "Clear choices". `EA-F2-053`.
 *
 * There are *two* Clear buttons, as in the portal: one above the row that only a
 * narrow screen shows, one inside it that only a wide screen shows. That is not a
 * mistake to tidy up — on a phone Clear is full-width above Back and Next rather than
 * squeezed between them.
 */
export const BallotActions = ({
    backComponent,
    backTo,
    onBack,
    onClear,
    onNext,
    disableNext,
    inert,
}: IBallotActionsProps): React.JSX.Element => {
    const {t} = useTranslation()

    /*
     * `votingScreen.*`, the portal's own keys, translated here.
     *
     * These three had an English copy in this file — `BALLOT_ACTIONS_WORDING_EN` — and
     * the wizard's preview read that instead of the catalogue, so a Spanish preview said
     * "Clear choices". The strings live in `voting-portal/src/translations/<lng>.ts`,
     * where they always have and where clients override them.
     */
    const back = t("votingScreen.backButton")
    const clear = t("votingScreen.clearButton")
    const next = t("votingScreen.reviewButton")

    const backLink =
        backComponent === undefined ? {} : {component: backComponent, to: backTo}

    return (
        <>
            <StyledButton
                className="clear-selection-button"
                sx={{
                    display: {sm: "none"},
                    width: "100%",
                }}
                variant="secondary"
                disabled={inert}
                onClick={onClear}
            >
                <Box className="clear-selection-label">{clear}</Box>
            </StyledButton>

            <ActionsContainer
                className="actions-container"
                sx={{marginBottom: "20px", marginTop: "10px"}}
            >
                <StyledButton
                    {...backLink}
                    className="back-button"
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                    disabled={inert}
                    onClick={onBack}
                >
                    <Icon className="back-button-icon" icon={faAngleLeft} size="sm" />
                    <Box className="back-button-label">{back}</Box>
                </StyledButton>

                <StyledButton
                    className="clear-selection-button"
                    sx={{
                        display: {xs: "none", sm: "block"},
                        width: {xs: "100%", sm: "200px"},
                    }}
                    variant="secondary"
                    disabled={inert}
                    onClick={onClear}
                >
                    <Box className="clear-selection-label">{clear}</Box>
                </StyledButton>

                <StyledButton
                    className="next-button"
                    sx={{width: {xs: "100%", sm: "200px"}}}
                    onClick={onNext}
                    disabled={inert === true || disableNext === true}
                >
                    <Box className="next-button-label">{next}</Box>
                    <Icon className="next-button-icon" icon={faAngleRight} size="sm" />
                </StyledButton>
            </ActionsContainer>
        </>
    )
}
