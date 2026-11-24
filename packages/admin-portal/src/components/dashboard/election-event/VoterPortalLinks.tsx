// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useId, useState} from "react"
import {Box, IconButton, Typography} from "@mui/material"
import QrCode2Icon from "@mui/icons-material/QrCode2"
import {useTranslation} from "react-i18next"
import type {IVotingChannelsConfig} from "@sequentech/ui-core"
import {Dialog, QRCode} from "@sequentech/ui-essentials"
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

/** A link's address as a QR code, so a kiosk or phone opens it without typing it. */
const VoterPortalLinkQrCode: React.FC<{link: IVoterPortalLink; onClose: () => void}> = ({
    link,
    onClose,
}) => {
    const {t} = useTranslation()
    const urlId = useId()
    return (
        <Dialog
            open
            title={link.label}
            cancel={t("dashboard.closeQrCode")}
            handleClose={onClose}
            variant="info"
            fullWidth
        >
            <Box sx={{display: "flex", flexDirection: "column", alignItems: "center", gap: 2}}>
                {/* A light quiet zone keeps the code scannable whatever the page background. */}
                <Box
                    sx={{
                        "backgroundColor": "#fff",
                        "padding": 2,
                        "width": "100%",
                        "maxWidth": 320,
                        "& .qr-code-svg": {display: "block", width: "100%", height: "auto"},
                    }}
                >
                    <QRCode value={link.href} ariaLabelledby={urlId} />
                </Box>
                <Typography
                    id={urlId}
                    variant="body2"
                    sx={{wordBreak: "break-all", textAlign: "center", userSelect: "all"}}
                >
                    {link.href}
                </Typography>
            </Box>
        </Dialog>
    )
}

/** The event's voter login and enrollment links, and the kiosk's when it votes by kiosk. */
export const VoterPortalLinks: React.FC<VoterPortalLinksProps> = ({electionEvent}) => {
    const {t} = useTranslation()
    const [qrLinkKey, setQrLinkKey] = useState<string | null>(null)
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
    const qrLink = links.find((link) => link.key === qrLinkKey)
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
                    <Box component="span" sx={{display: "inline-flex", alignItems: "center"}}>
                        <a href={link.href} target="_blank" rel="noopener noreferrer">
                            {link.label}
                        </a>
                        <IconButton
                            size="small"
                            aria-label={t("dashboard.showQrCode", {link: link.label})}
                            title={t("dashboard.showQrCode", {link: link.label})}
                            onClick={() => setQrLinkKey(link.key)}
                            disabled={!link.href}
                        >
                            <QrCode2Icon fontSize="small" />
                        </IconButton>
                    </Box>
                </React.Fragment>
            ))}
            {qrLink?.href ? (
                <VoterPortalLinkQrCode link={qrLink} onClose={() => setQrLinkKey(null)} />
            ) : null}
        </Box>
    )
}
