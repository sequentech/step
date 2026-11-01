// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Get email updates about this ballot" (design boards GetUpdates,
// ConfirmationUpdates and PhoneConfirmationUpdates): the hubs that the
// election's policy names, as equal tiles. Each opens the hub's own site in a
// new tab with the ballot's receipt in the link, and the hub's site asks for
// the address and what to watch there; nothing is sent from this page. The
// tiles come in a random order on each visit, and none is the default or
// stands out.

import React, {useId, useState} from "react"
import {Box} from "@mui/material"
import {useTranslation} from "react-i18next"
import {IOfferedHub, shuffled, watchLink} from "../../services/vitrine/hubUpdates"

const COLORS = {
    border: "#E3E7F1",
    tileBorder: "#CBD3E3",
    text: "#191D23",
    navy: "#0F054C",
    // The board's grey, a little darker: the band behind the footnote is not white.
    quiet: "#5B6B80",
    body: "#475569",
    icon: "#E9EAF6",
} as const

/** From this width the text goes left and the tiles right, in one band. */
const WIDE = 760

/** The width of a tile in the wide band, and the icon's indent under which
 * the footnote lines up with the title. */
const TILE_WIDTH = 220
const FOOT_INDENT = 54

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

const ExternalIcon: React.FC = () => (
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
                m: "4px 0 20px",
                border: `1px solid ${COLORS.border}`,
                borderRadius: "12px",
                background: "linear-gradient(180deg, #FAFBFF 0%, #F4F6FC 100%)",
                color: COLORS.text,
                containerType: "inline-size",
                [`@container (min-width: ${WIDE}px)`]: {
                    "& .get-updates-inner": {
                        gridTemplateColumns: "minmax(0, 1fr) auto",
                        alignItems: "center",
                        columnGap: "32px",
                        padding: "18px 20px",
                    },
                    "& .get-updates-hubs": {
                        gridTemplateColumns: "none",
                        gridAutoFlow: "column",
                        gridAutoColumns: `${TILE_WIDTH}px`,
                    },
                    "& .get-updates-foot": {
                        gridColumn: "1 / -1",
                        paddingLeft: `${FOOT_INDENT}px`,
                    },
                },
            }}
        >
            <Box className="get-updates-inner" sx={{display: "grid", gap: "14px", p: "18px"}}>
                <Box
                    className="get-updates-head"
                    sx={{display: "flex", gap: "14px", alignItems: "flex-start", minWidth: 0}}
                >
                    <Box
                        component="span"
                        className="get-updates-icon"
                        aria-hidden="true"
                        sx={{
                            flex: "none",
                            width: 40,
                            height: 40,
                            borderRadius: "50%",
                            backgroundColor: COLORS.icon,
                            color: COLORS.navy,
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                        }}
                    >
                        <MailIcon />
                    </Box>
                    <Box className="get-updates-text" sx={{minWidth: 0}}>
                        <Box
                            component="h2"
                            id={titleId}
                            className="get-updates-title"
                            sx={{m: 0, fontSize: "16px", lineHeight: "22px", fontWeight: 700}}
                        >
                            {t(`${KEYS}.title`)}
                        </Box>
                        <Box
                            component="p"
                            className="get-updates-description"
                            sx={{
                                m: "2px 0 0",
                                fontSize: "14px",
                                lineHeight: "20px",
                                color: COLORS.body,
                            }}
                        >
                            {t(`${KEYS}.description`)}
                        </Box>
                    </Box>
                </Box>
                <Box
                    component="ul"
                    className="get-updates-hubs"
                    aria-label={t(`${KEYS}.listLabel`)}
                    sx={{
                        display: "grid",
                        gridTemplateColumns: "minmax(0, 1fr)",
                        gap: "8px",
                        m: 0,
                        p: 0,
                        listStyle: "none",
                    }}
                >
                    {ordered.map((hub) => (
                        <Box
                            component="li"
                            key={hub.name}
                            className="get-updates-item"
                            sx={{display: "flex"}}
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
                                    "display": "flex",
                                    "alignItems": "center",
                                    "gap": "12px",
                                    "minHeight": 56,
                                    "p": "8px 14px",
                                    "boxSizing": "border-box",
                                    "border": `1px solid ${COLORS.tileBorder}`,
                                    "borderRadius": "8px",
                                    "backgroundColor": "#fff",
                                    "color": COLORS.navy,
                                    "textDecoration": "none",
                                    "transition": "border-color .15s, box-shadow .15s",
                                    "&:hover": {
                                        borderColor: COLORS.navy,
                                        boxShadow: "0 1px 0 rgba(15, 5, 76, 0.08)",
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
                    sx={{m: 0, fontSize: "12px", lineHeight: "17px", color: COLORS.quiet}}
                >
                    {t(receipt ? `${KEYS}.footnote` : `${KEYS}.footnoteNoReceipt`)}
                </Box>
            </Box>
        </Box>
    )
}
