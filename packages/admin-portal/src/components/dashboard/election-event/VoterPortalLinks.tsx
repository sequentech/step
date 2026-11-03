// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {Box} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {IVotingChannelsConfig} from "@sequentech/ui-core"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {getAuthUrl} from "@/services/UrlGeneration"

export interface VoterPortalLinksProps {
    electionEvent?: {id?: string | null; voting_channels?: unknown} | null
}

interface IVoterPortalLink {
    key: string
    label: string
    href: string
}

/** The event's voter login and enrollment links, and the kiosk's when it votes by kiosk. */
export const VoterPortalLinks: React.FC<VoterPortalLinksProps> = ({electionEvent}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const eventId = electionEvent?.id
    if (!tenantId || !eventId) {
        return null
    }
    const portalUrl = globalSettings.VOTING_PORTAL_URL
    const kioskPortalUrl = globalSettings.KIOSK_VOTING_PORTAL_URL?.trim() || portalUrl
    const channels = electionEvent?.voting_channels as IVotingChannelsConfig | null | undefined
    const links: IVoterPortalLink[] = [
        {
            key: "login",
            label: t("dashboard.voterLoginURL"),
            href: getAuthUrl(portalUrl, tenantId, eventId, "login") ?? "",
        },
        {
            key: "enroll",
            label: t("dashboard.voterEnrollURL"),
            href: getAuthUrl(portalUrl, tenantId, eventId, "enroll") ?? "",
        },
    ]
    if (channels?.kiosk === true) {
        const kioskUrl = getAuthUrl(kioskPortalUrl, tenantId, eventId, "login")
        links.push({
            key: "kiosk",
            label: t("dashboard.voterEnrollKioskURL"),
            href: kioskUrl ? `${kioskUrl}?kiosk` : "",
        })
    }
    return (
        <Box
            sx={{
                display: "flex",
                flexWrap: "wrap",
                justifyContent: "center",
                alignItems: "center",
                columnGap: "20px",
                rowGap: "4px",
                paddingTop: "10px",
            }}
        >
            {links.map((link, index) => (
                <React.Fragment key={link.key}>
                    {index > 0 && <p aria-hidden="true">|</p>}
                    <a href={link.href} target="_blank" rel="noopener noreferrer">
                        {link.label}
                    </a>
                </React.Fragment>
            ))}
        </Box>
    )
}
