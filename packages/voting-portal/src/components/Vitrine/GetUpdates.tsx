// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Get email updates about this ballot" (design boards HubTilesV42 and
// ConfirmationV42B): an optional section under the receipt, with no box of its
// own, offering the hubs that the election's policy names as equal tiles. Each opens the hub's own site in a
// new tab with the ballot's receipt in the link, and the hub's site asks for
// the address and what to watch there; nothing is sent from this page. The
// tiles come in a random order on each visit, and none is the default or
// stands out.

import React, {useId, useState} from "react"
import {Box} from "@mui/material"
import {useTranslation} from "react-i18next"
import {IOfferedHub, shuffled, watchLink} from "../../services/vitrine/hubUpdates"

const COLORS = {
    tileBorder: "#D5DBE8",
    text: "#191D23",
    navy: "#0F054C",
    quiet: "#5B6474",
    body: "#4A5262",
    hover: "#FAFBFF",
} as const

/** Narrower than this, the tiles stack and the words get a little smaller. */
const NARROW = 480

/** The narrowest a tile gets before the row wraps. */
const TILE_MIN = 232

const KEYS = "vitrine.updates"

const MailIcon: React.FC = () => (
    <svg
        className="get-updates-mail"
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
    >
        <rect className="get-updates-mail-body" x="3" y="5" width="18" height="14" rx="2" />
        <path className="get-updates-mail-flap" d="M3.5 6.5l8.5 6.5 8.5-6.5" />
    </svg>
)

export const ExternalIcon: React.FC = () => (
    <svg
        className="get-updates-external"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        style={{flex: "none"}}
    >
        <path className="get-updates-external-path" d="M14 4h6v6" />
        <path className="get-updates-external-path" d="M20 4l-9 9" />
        <path
            className="get-updates-external-path"
            d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"
        />
    </svg>
)

export interface IGetUpdatesProps {
    /** The hubs to offer; nothing is shown without them. */
    hubs: readonly IOfferedHub[]
    /** The policy ID, `sha256:<hex>`, which the hub's site opens the election by. */
    policyId: string
    /** The ballot's receipt chain, once there is one. */
    receipt?: string
}

/** The order is drawn when the callout appears, and kept while the page
 * updates: tiles that moved when a status changed would be moving under the
 * voter's pointer. Give the component a `key` of the policy ID so that a
 * different policy draws again. */
export const GetUpdates: React.FC<IGetUpdatesProps> = ({hubs, policyId, receipt}) => {
    const {t} = useTranslation()
    const titleId = useId()
    const [order] = useState(() => shuffled(hubs.map((hub) => hub.name)))
    const ordered = order.flatMap((name) => hubs.filter((hub) => hub.name === name))
    if (ordered.length === 0) return null

    return (
        <Box
            component="section"
            className="get-updates"
            aria-labelledby={titleId}
            sx={{
                width: "100%",
                boxSizing: "border-box",
                color: COLORS.text,
                containerType: "inline-size",
                [`@container (max-width: ${NARROW}px)`]: {
                    "& .get-updates-hubs": {gridTemplateColumns: "minmax(0, 1fr)"},
                    "& .get-updates-title": {fontSize: "17px"},
                    "& .get-updates-description": {fontSize: "14px", lineHeight: "21px"},
                },
            }}
        >
            <Box
                component="span"
                className="get-updates-optional"
                sx={{
                    display: "inline-block",
                    m: "0 0 6px",
                    fontSize: "12px",
                    lineHeight: "16px",
                    fontWeight: 700,
                    letterSpacing: "0.06em",
                    textTransform: "uppercase",
                    color: COLORS.quiet,
                }}
            >
                {t(`${KEYS}.optional`)}
            </Box>
            <Box
                component="h2"
                id={titleId}
                className="get-updates-title"
                sx={{
                    "display": "flex",
                    "alignItems": "center",
                    "gap": "10px",
                    "m": 0,
                    "fontSize": "18px",
                    "lineHeight": "24px",
                    "fontWeight": 700,
                    "& .get-updates-mail": {flex: "none", color: COLORS.navy},
                }}
            >
                <MailIcon />
                <span className="get-updates-title-text">{t(`${KEYS}.title`)}</span>
            </Box>
            <Box
                component="p"
                className="get-updates-description"
                sx={{
                    m: "6px 0 0",
                    maxWidth: "62ch",
                    fontSize: "15px",
                    lineHeight: "22px",
                    color: COLORS.body,
                }}
            >
                {t(`${KEYS}.description`)}
            </Box>
            <Box
                component="ul"
                className="get-updates-hubs"
                aria-label={t(`${KEYS}.listLabel`)}
                sx={{
                    display: "grid",
                    gridTemplateColumns: `repeat(auto-fill, minmax(${TILE_MIN}px, 1fr))`,
                    gap: "10px",
                    m: "16px 0 0",
                    p: 0,
                    listStyle: "none",
                }}
            >
                {ordered.map((hub) => (
                    <Box
                        component="li"
                        key={hub.name}
                        className="get-updates-item"
                        sx={{display: "flex", minWidth: 0}}
                    >
                        <Box
                            component="a"
                            className="get-updates-hub"
                            href={watchLink(hub.site, policyId, receipt)}
                            target="_blank"
                            rel="noopener noreferrer"
                            aria-label={t(`${KEYS}.hubLabel`, {
                                name: hub.name,
                                party: hub.party,
                            })}
                            sx={{
                                "flex": 1,
                                "minWidth": 0,
                                "display": "flex",
                                "alignItems": "center",
                                "gap": "12px",
                                "minHeight": 60,
                                "p": "10px 14px 10px 16px",
                                "boxSizing": "border-box",
                                "border": `1px solid ${COLORS.tileBorder}`,
                                "borderRadius": "10px",
                                "backgroundColor": "#fff",
                                "color": COLORS.navy,
                                "textDecoration": "none",
                                "transition": "border-color .15s, background-color .15s",
                                "&:hover": {
                                    borderColor: COLORS.navy,
                                    backgroundColor: COLORS.hover,
                                },
                                "&:focus-visible": {
                                    outline: `2px solid ${COLORS.navy}`,
                                    outlineOffset: "2px",
                                },
                            }}
                        >
                            <Box
                                component="span"
                                className="get-updates-hub-text"
                                sx={{flex: 1, minWidth: 0}}
                            >
                                <Box
                                    component="span"
                                    className="get-updates-hub-name"
                                    sx={{
                                        display: "block",
                                        fontSize: "15px",
                                        lineHeight: "20px",
                                        fontWeight: 600,
                                        overflowWrap: "anywhere",
                                    }}
                                >
                                    {hub.name}
                                </Box>
                                <Box
                                    component="span"
                                    className="get-updates-hub-who"
                                    sx={{
                                        display: "block",
                                        mt: "1px",
                                        fontSize: "13px",
                                        lineHeight: "18px",
                                        color: COLORS.quiet,
                                        overflowWrap: "anywhere",
                                    }}
                                >
                                    {t(`${KEYS}.runBy`, {party: hub.party})}
                                </Box>
                            </Box>
                            <ExternalIcon />
                        </Box>
                    </Box>
                ))}
            </Box>
            <Box
                component="p"
                className="get-updates-foot"
                sx={{
                    m: "12px 0 0",
                    maxWidth: "80ch",
                    fontSize: "13px",
                    lineHeight: "19px",
                    color: COLORS.quiet,
                }}
            >
                {t(receipt ? `${KEYS}.footnote` : `${KEYS}.footnoteNoReceipt`)}
            </Box>
        </Box>
    )
}
