// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {faAngleLeft, faAngleRight} from "@fortawesome/free-solid-svg-icons"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"
import React from "react"

import Icon from "../components/Icon/Icon"
import {ActionsContainer, StyledButton} from "../components/ConfirmationActions/ConfirmationActions"

/**
 * What frames the Back control.
 *
 * The portal's is `styled(RouterLink)`; this is the same rule set on a `Box`, so the
 * caller says what to render it *as* — a router link in the portal, a plain element in
 * a preview that has no router. The two `outline: none` rules are the portal's own,
 * and its comment for them was: a link must contain a single tabbable element, the
 * button inside it.
 */
const BackFrame = styled(Box)<{
    /** What to render as. `Box` forwards this; the type is restated because
     * `styled()` drops the polymorphism from `Box`'s own props. */
    component?: React.ElementType
    /** Consumed by a router `Link` given as `component`, ignored by a `div`. */
    to?: string | object
}>`
    margin: auto 0;
    text-decoration: none;

    &:focus {
        outline: none;
    }

    & *[tabindex] {
        outline: none;
    }
`

export interface IBallotActionsWording {
    /** `votingScreen.backButton` — "Back". */
    back: string
    /** `votingScreen.clearButton` — "Clear choices". */
    clear: string
    /** `votingScreen.reviewButton` — "Next". */
    next: string
}

/**
 * The portal's English, for a host with no catalogue of its own for this row.
 *
 * Same purpose as `START_WORDING_EN`: the keys live in *voting-portal*, so a shared
 * component that translated for itself would draw raw keys anywhere else. A caller
 * with translations passes them; one without gets the words a voter really sees, which
 * is what a preview needs.
 */
export const BALLOT_ACTIONS_WORDING_EN: IBallotActionsWording = {
    back: "Back",
    clear: "Clear choices",
    next: "Next",
}

export interface IBallotActionsProps {
    /** The three labels. */
    wording?: IBallotActionsWording
    /**
     * What to render the Back control as, when it navigates by link. The portal
     * passes its router's `Link` and the `to` below; a preview passes neither and
     * gets a plain `div` around a button.
     */
    backComponent?: React.ElementType
    /** Where Back goes, for `backComponent`. */
    backTo?: string | object
    /** Called before Back navigates — the portal steps its contest pagination here. */
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
 * There are *two* Clear buttons, as in the portal: one above the row that only a
 * narrow screen shows, one inside it that only a wide screen shows. That is not a
 * mistake to tidy up — on a phone Clear is full-width above Back and Next rather than
 * squeezed between them.
 */
export const BallotActions = ({
    wording = BALLOT_ACTIONS_WORDING_EN,
    backComponent,
    backTo,
    onBack,
    onClear,
    onNext,
    disableNext,
    inert,
}: IBallotActionsProps): React.JSX.Element => (
    <>
        <StyledButton
            sx={{
                display: {sm: "none"},
                width: "100%",
            }}
            variant="secondary"
            disabled={inert}
            onClick={onClear}
        >
            <Box>{wording.clear}</Box>
        </StyledButton>

        <ActionsContainer sx={{marginBottom: "20px", marginTop: "10px"}}>
            <BackFrame
                component={backComponent ?? "div"}
                to={backComponent === undefined ? undefined : backTo}
                sx={{width: {xs: "100%", sm: "200px"}}}
                onClick={onBack}
            >
                <StyledButton sx={{width: {xs: "100%", sm: "200px"}}} disabled={inert}>
                    <Icon icon={faAngleLeft} size="sm" />
                    <Box>{wording.back}</Box>
                </StyledButton>
            </BackFrame>

            <StyledButton
                sx={{
                    display: {xs: "none", sm: "block"},
                    width: {xs: "100%", sm: "200px"},
                }}
                variant="secondary"
                disabled={inert}
                onClick={onClear}
            >
                <Box>{wording.clear}</Box>
            </StyledButton>

            <StyledButton
                className="next-button"
                sx={{width: {xs: "100%", sm: "200px"}}}
                onClick={onNext}
                disabled={inert === true || disableNext === true}
            >
                <Box>{wording.next}</Box>
                <Icon icon={faAngleRight} size="sm" />
            </StyledButton>
        </ActionsContainer>
    </>
)
