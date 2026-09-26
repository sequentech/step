// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {faAngleLeft, faAngleRight, faFire} from "@fortawesome/free-solid-svg-icons"
import {Box, CircularProgress} from "@mui/material"
import {styled} from "@mui/material/styles"
import React from "react"
import {useTranslation} from "react-i18next"

import Icon from "../components/Icon/Icon"
import {ActionsContainer, StyledButton} from "../components/ActionsRow/ActionsRow"

/** The chevron on *Cast ballot*, at the size the portal draws it. */
const StyledIcon = styled(Icon)`
    min-width: 14px;
    padding: 5px;
`

/** While a ballot is being cast, in place of that chevron. */
const StyledCircularProgress = styled(CircularProgress)`
    width: 14px !important;
    height: 14px !important;
`

/** What frames the Back control. The portal's is a router link; see `BallotActions`. */
const BackFrame = styled(Box)<{
    component?: React.ElementType
    to?: string | object
}>`
    margin: auto 0;
    text-decoration: none;
`

export interface IReviewActionsProps {
    /**
     * Whether *Audit ballot* is shown.
     *
     * `EVotingPortalAuditButtonCfg` decides in the portal: shown beside the others, shown
     * inside the ballot-identifier help, or not at all. That is a policy read from the
     * event, so it stays with the caller; this only draws what it is told.
     */
    withAudit?: boolean
    /** Casting is under way: the button waits with a spinner rather than a chevron. */
    casting?: boolean
    /** What to render Back as when it navigates by link. */
    backComponent?: React.ElementType
    backTo?: string | object
    onBack?: () => void
    onAudit?: () => void
    onCast?: () => void
    /**
     * Draw the row, refuse to work it.
     *
     * For a preview: these are the buttons a voter will meet, and there is no ballot to
     * cast. The Election Architect passes this and wires Back and Cast to move between
     * the screens it draws.
     */
    inert?: boolean
}

/**
 * The row under the review screen: `‹ Edit ballot`, *Audit ballot*, *Cast ballot ›*.
 *
 * Lifted out of the portal's `ReviewScreen`, where it was `ActionButtons` — a hundred
 * lines of casting logic with this row at the bottom of it. Only the row is here: what
 * casts stays in the route, because it needs a mutation, a store and a session.
 *
 * The Election Architect drew its own three buttons before this: plain MUI, evenly
 * spaced, no icons and no warning colour, from a table in its `PortalFrame`. A client
 * checking that *Audit ballot* is the one that stands out was looking at a picture that
 * did not have that property.
 *
 * The two icons are the portal's own — a flame on Audit, a chevron on Cast — and *Audit
 * ballot* is `variant="warning"`, which is what makes it read as the unusual choice.
 */
export const ReviewActions = ({
    withAudit = false,
    casting = false,
    backComponent,
    backTo,
    onBack,
    onAudit,
    onCast,
    inert = false,
}: IReviewActionsProps): React.JSX.Element => {
    const {t} = useTranslation()

    return (
        <Box sx={{marginBottom: "10px", marginTop: "10px"}}>
            <ActionsContainer className="actions-container">
                <BackFrame
                    component={backComponent ?? "div"}
                    to={backComponent === undefined ? undefined : backTo}
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                    onClick={onBack}
                >
                    <StyledButton
                        sx={{width: {xs: "100%", sm: "200px"}}}
                        disabled={inert && onBack === undefined}
                    >
                        <Icon icon={faAngleLeft} size="sm" />
                        <Box>{t("reviewScreen.backButton")}</Box>
                    </StyledButton>
                </BackFrame>

                {withAudit ? (
                    <StyledButton
                        className="audit-button"
                        sx={{width: {xs: "100%", sm: "200px"}}}
                        variant="warning"
                        disabled={onAudit === undefined}
                        onClick={onAudit}
                    >
                        <Icon icon={faFire} size="sm" />
                        <Box>{t("reviewScreen.auditButton")}</Box>
                    </StyledButton>
                ) : null}

                <StyledButton
                    className="cast-ballot-button"
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                    disabled={casting || onCast === undefined}
                    onClick={onCast}
                >
                    <Box>{t("reviewScreen.castBallotButton")}</Box>
                    {casting ? (
                        <StyledCircularProgress color="inherit" />
                    ) : (
                        <StyledIcon icon={faAngleRight} size="sm" />
                    )}
                </StyledButton>
            </ActionsContainer>
        </Box>
    )
}
