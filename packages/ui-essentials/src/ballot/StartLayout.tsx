// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import Box from "@mui/material/Box"
import Typography from "@mui/material/Typography"
import {styled} from "@mui/material/styles"
import React from "react"

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
}

const StyledTitle = styled(Typography)`
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
    const instructions = [1, 2, 3].map((at) => ({
        title: t(`startScreen.step${at}Title`),
        description: t(`startScreen.step${at}Description`),
    }))

    return (
        <PageLimit maxWidth="lg" className="start-screen screen">
            {steps === undefined ? null : <Box marginTop="48px">{steps}</Box>}
            {above}
            <StyledTitle variant="h3" fontWeight="bold">
                <span>{title}</span>
            </StyledTitle>
            {description === undefined ? null : (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {description}
                </Typography>
            )}
            <Typography variant="h5">{t("startScreen.instructionsTitle")}</Typography>
            <Typography variant="body2">{t("startScreen.instructionsDescription")}</Typography>
            <Box
                sx={{
                    display: "flex",
                    flexDirection: {xs: "column", md: "row"},
                    gap: {sm: 0, md: "15px"},
                }}
            >
                {instructions.map((step) => (
                    <Box key={step.title} sx={{width: {xs: "100%", md: "33.33333333%"}}}>
                        <Typography variant="h5" sx={{color: theme.palette.brandColor}}>
                            {step.title}
                        </Typography>
                        <Typography variant="body2">{step.description}</Typography>
                    </Box>
                ))}
            </Box>
            {below}
        </PageLimit>
    )
}
