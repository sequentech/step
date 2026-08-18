// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import Box from "@mui/material/Box"
import Typography from "@mui/material/Typography"
import {styled} from "@mui/material/styles"
import React from "react"

import PageLimit from "../components/PageLimit/PageLimit"
import {theme} from "../services/theme"

/**
 * The wording this screen reads, handed in rather than looked up.
 *
 * **No `t()` in this file, and that is the point.** These strings are
 * `startScreen.*`, which live in *voting-portal*'s catalogue and not in
 * `ui-essentials` — so a layout that translated for itself would render raw
 * `startScreen.instructionsTitle` everywhere it was used outside the portal. That is
 * not a hypothetical: the wizard's preview drew raw `selectElection.*` keys for
 * exactly this reason, and the fix was to stop guessing where a string lives.
 *
 * The portal passes its own `t(…)` values, so nothing about its behaviour changes and
 * its six locales keep the keys they already have. The preview passes what the client
 * configured, falling back to {@link START_WORDING_EN}.
 */
export interface IStartWording {
    instructionsTitle: string
    instructionsDescription: string
    steps: Array<{title: string; description: string}>
}

/**
 * The English the platform ships, for a caller with nothing configured.
 *
 * Here rather than in the preview, so the two do not drift: this is the same wording
 * the portal's `en` catalogue carries, and a client reviewing their event should see
 * what a voter sees before any override.
 */
export const START_WORDING_EN: IStartWording = {
    instructionsTitle: "Instructions",
    instructionsDescription: "Please follow these steps to cast your ballot:",
    steps: [
        {
            title: "1. Select your options",
            description:
                "Choose your preferred candidates and answer the Ballot questions one by one as they appear. You can edit your ballot until you are ready to proceed.",
        },
        {
            title: "2. Review your ballot",
            description:
                "Once you are satisfied with your selections, we will encrypt your ballot and show you a final review of your choices. You will also receive a unique tracker ID for your ballot.",
        },
        {
            title: "3. Cast your ballot",
            description:
                "Cast your ballot: Finally, you can cast your ballot so it is properly registered. Alternatively, you can opt to audit and confirm that your ballot was correctly captured and encrypted.",
        },
    ],
}

export interface IStartLayoutProps {
    /** The election's name, already translated by whoever owns the presentation. */
    title: string
    /** Its description, already translated and already HTML if it is HTML. */
    description?: React.ReactNode
    wording?: IStartWording
    /** The stepper, the action row — whatever the caller frames this with. */
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
    wording = START_WORDING_EN,
    above,
    below,
}: IStartLayoutProps): React.JSX.Element => (
    <PageLimit maxWidth="lg" className="start-screen screen">
        {above}
        <StyledTitle variant="h3" fontWeight="bold">
            <span>{title}</span>
        </StyledTitle>
        {description === undefined ? null : (
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {description}
            </Typography>
        )}
        <Typography variant="h5">{wording.instructionsTitle}</Typography>
        <Typography variant="body2">{wording.instructionsDescription}</Typography>
        <Box
            sx={{
                display: "flex",
                flexDirection: {xs: "column", md: "row"},
                gap: {sm: 0, md: "15px"},
            }}
        >
            {wording.steps.map((step) => (
                <Box
                    key={step.title}
                    sx={{width: {xs: "100%", md: "33.33333333%"}}}
                >
                    <Typography
                        variant="h5"
                        sx={{color: theme.palette.brandColor}}
                    >
                        {step.title}
                    </Typography>
                    <Typography variant="body2">{step.description}</Typography>
                </Box>
            ))}
        </Box>
        {below}
    </PageLimit>
)
