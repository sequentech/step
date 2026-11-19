// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useMemo} from "react"
import Stack from "@mui/material/Stack"
import {styled} from "@mui/material/styles"
import {
    eventLogo as sharedEventLogo,
    hidesPlatformBranding,
    IElectionEventPresentation,
    PLATFORM_BRANDING_HIDDEN_CLASS,
    sanitizePresentationCss,
} from "@sequentech/ui-core"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"
import {SettingsContext} from "../../providers/SettingsContextProvider"

type Presentation = IElectionEventPresentation | null | undefined

export const EVENT_STYLES_CLASS = "voting-portal-wrapper"

// Every page of the portal, the error page included, sits inside this wrapper
// so the election event's CSS reaches all of them.
export const EventStyles = styled(Stack, {
    shouldForwardProp: (prop) => prop !== "customCss",
})<{customCss: string}>`
    &.${PLATFORM_BRANDING_HIDDEN_CLASS} .app-version {
        display: none;
    }

    ${({customCss}) => customCss}
`

export const eventStylesClassName = (presentation: Presentation): string =>
    hidesPlatformBranding(presentation)
        ? `${EVENT_STYLES_CLASS} ${PLATFORM_BRANDING_HIDDEN_CLASS}`
        : EVENT_STYLES_CLASS

export const useEventCss = (presentation: Presentation): string => {
    const {globalSettings} = useContext(SettingsContext)

    return useMemo(
        () =>
            sanitizePresentationCss(presentation?.css ?? "", {
                baseUrl: document.baseURI,
                publicBucketUrl: globalSettings.PUBLIC_BUCKET_URL,
            }),
        [presentation?.css, globalSettings.PUBLIC_BUCKET_URL]
    )
}

export const eventLogo = (presentation: Presentation): string =>
    sharedEventLogo(presentation, {platform: SequentLogo, blank: BlankLogoImg})
