// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import Box from "@mui/material/Box"
import Typography from "@mui/material/Typography"
import {styled} from "@mui/material/styles"
import React from "react"

import {stringToHtml} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

import PageLimit from "../components/PageLimit/PageLimit"
import {theme} from "../services/theme"

/**
 * The screen's own arrangement. Its words come from `startScreen.*`, read here.
 *
 * **This file used to hold those eight strings in English**, on the argument that they
 * live in *voting-portal*'s catalogue, so a layout translating for itself would draw raw
 * keys anywhere else. The argument was wrong twice over: every host of this component
 * carries that catalogue — the Election Architect vendors it and hands it to the preview
 * through `PreviewLocale` — and the copy meant the wizard showed English to a client
 * previewing in Spanish, from strings that had *drifted* from the portal's own
 * ("Instructions" where the portal says "How to vote").
 *
 * So: `t()` on the paths clients override, and the strings stay in
 * `voting-portal/src/translations/<lng>.ts` where they have always been. `EA-F2-053`.
 */
export interface IStartLayoutProps {
    /** The election's name, already translated by whoever owns the presentation. */
    title: string
    /** Its description, already translated and already HTML if it is HTML. */
    description?: React.ReactNode
    /**
     * The breadcrumb, framed here 48px under the header — as `ReviewLayout`,
     * `ConfirmationLayout` and `ElectionListLayout` frame theirs.
     *
     * It used to arrive through `above` with the portal's route doing the framing,
     * which left that one measurement written down in every caller. The wizard's
     * preview had it as 16px, outside the screen entirely.
     */
    steps?: React.ReactNode
    /** Anything else above the title, framed by the caller. */
    above?: React.ReactNode
    below?: React.ReactNode
    /** The host's dialogs, which belong with the state that opens them. */
    children?: React.ReactNode
}

const StyledTitle = styled(Typography)<{component?: React.ElementType}>`
    margin-top: 25.5px;
    display: flex;
    justify-content: center;
    text-align: center;
`

/**
 * The screen a voter meets before the ballot: what this election is, and what is
 * about to happen in three steps.
 *
 * Lifted out of the portal's `StartScreen` route, which cannot be reused as it
 * stands: it reads five slices of redux, three router params and a ballot-encryption
 * hook, none of which exist in a preview. The arrangement — a centred title, the
 * description, then the instructions in three columns that stack on a phone — is the
 * part worth sharing, and it is now the only copy of it. `ReviewLayout` and
 * `ConfirmationLayout` beside this file were lifted the same way and for the same
 * reason.
 *
 * Everything conditional stays with the caller. The security checkbox, *Decline to
 * Vote*, the demo dialog and the navigation all belong to the route: they act, and a
 * layout that acts is a layout that needs a store.
 */
export const StartLayout = ({
    title,
    description,
    steps,
    above,
    below,
    children,
}: IStartLayoutProps): React.JSX.Element => {
    const {t} = useTranslation()

    /*
     * `startScreen.*`, translated here.
     *
     * This file carried the eight strings as `START_WORDING_EN`, and the wizard's
     * preview read them instead of the catalogue — so a Spanish preview of a Spanish
     * election showed English instructions. They live in
     * `voting-portal/src/translations/<lng>.ts`, on the same paths as ever.
     */
    const instructions = (["select", "review", "cast"] as const).map((kind, index) => ({
        kind,
        title: t(`startScreen.step${index + 1}Title`),
        description: t(`startScreen.step${index + 1}Description`),
    }))

    return (
        <PageLimit maxWidth="lg" className="start-screen screen">
            {steps === undefined ? null : (
                <Box className="stepper-box" marginTop="48px">
                    {steps}
                </Box>
            )}
            {above}
            <StyledTitle className="screen-title" variant="h3" component="h1" fontWeight="bold">
                <span className="screen-title-text">{title}</span>
            </StyledTitle>
            {description === undefined ? null : (
                <Typography
                    className="screen-description"
                    variant="body2"
                    component="div"
                    sx={{color: theme.palette.customGrey.main}}
                >
                    {description}
                </Typography>
            )}
            <Typography className="instructions-title" variant="h5" component="h2">
                {t("startScreen.instructionsTitle")}
            </Typography>
            <Typography className="instructions-description" variant="body2" component="div">
                {stringToHtml(t("startScreen.instructionsDescription"))}
            </Typography>
            <Box
                className="instructions-steps"
                sx={{
                    display: "flex",
                    flexDirection: {xs: "column", md: "row"},
                    gap: {sm: 0, md: "15px"},
                }}
            >
                {instructions.map((step) => (
                    <Box
                        key={step.kind}
                        className={`instructions-step instructions-${step.kind}-step`}
                        sx={{width: {xs: "100%", md: "33.33333333%"}}}
                    >
                        <Typography
                            className="instructions-step-title"
                            variant="h5"
                            component="h3"
                            sx={{color: theme.palette.brandColor}}
                        >
                            {step.title}
                        </Typography>
                        <Typography
                            className="instructions-step-description"
                            variant="body2"
                            component="div"
                        >
                            {stringToHtml(step.description)}
                        </Typography>
                    </Box>
                ))}
            </Box>
            {below}
            {children}
        </PageLimit>
    )
}
