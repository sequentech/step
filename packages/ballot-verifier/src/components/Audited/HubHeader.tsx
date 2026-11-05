// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The header of a transparency hub's page (design board HubHeader), which
// the audited ballot screen shows instead of the election's: a strip that
// says whose site this is and what it never asks for, then the hub's mark,
// its name and who runs it. The name and the operator come from the
// election's policy, or the verifier's settings; without them the header
// says only that this is a hub.

import React from "react"
import Box from "@mui/material/Box"
import {useTranslation} from "react-i18next"
import type {IHubIdentity} from "../../services/vitrine/auditedBallot"
import {FONT, HUB_COLORS, PHONE} from "./hubStyle"

const HubMark: React.FC = () => (
    <Box
        component="span"
        className="hub-header-mark"
        sx={{
            width: "40px",
            height: "40px",
            borderRadius: "10px",
            backgroundColor: HUB_COLORS.key,
            color: "#fff",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            flex: "none",
            [PHONE]: {width: "36px", height: "36px"},
        }}
    >
        <svg
            width="22"
            height="22"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            style={{overflow: "visible"}}
        >
            <circle cx="10.5" cy="10.5" r="7" />
            <path d="M15.5 15.5l5 5" />
            <path d="M7.2 10.8l2.3 2.3 4.4-4.6" />
        </svg>
    </Box>
)

export interface IHubHeaderProps {
    hub: IHubIdentity
}

export const HubHeader: React.FC<IHubHeaderProps> = ({hub}) => {
    const {t} = useTranslation()
    const run = hub.operator ? "withOperator" : "noOperator"
    return (
        <Box
            component="header"
            className="hub-header"
            sx={{
                fontFamily: FONT,
                color: "#191D23",
                backgroundColor: "#fff",
                borderBottom: "1px solid #E7EAEE",
            }}
        >
            <Box
                className="hub-header-strip"
                sx={{
                    display: "flex",
                    justifyContent: "center",
                    p: "6px 16px",
                    backgroundColor: HUB_COLORS.deep,
                    color: HUB_COLORS.stripText,
                    fontSize: "12px",
                    lineHeight: "16px",
                    textAlign: "center",
                }}
            >
                {t(`auditedScreen.header.strip.${run}`, {operator: hub.operator})}
            </Box>
            <Box
                className="hub-header-bar"
                sx={{
                    boxSizing: "border-box",
                    maxWidth: "1184px",
                    margin: "0 auto",
                    p: "14px 24px",
                    display: "flex",
                    alignItems: "center",
                    gap: "10px",
                    [PHONE]: {maxWidth: "none", p: "10px 16px"},
                }}
            >
                <HubMark />
                <Box component="span" sx={{minWidth: 0}}>
                    <Box
                        component="span"
                        className="hub-header-name"
                        sx={{
                            display: "block",
                            fontSize: "18px",
                            lineHeight: "22px",
                            fontWeight: 700,
                            color: HUB_COLORS.deep,
                            overflowWrap: "anywhere",
                        }}
                    >
                        {hub.name ?? t("auditedScreen.header.unnamed")}
                    </Box>
                    <Box
                        component="span"
                        className="hub-header-operator"
                        sx={{
                            display: "block",
                            fontSize: "12px",
                            lineHeight: "16px",
                            color: "#64748B",
                        }}
                    >
                        {t(`auditedScreen.header.subline.${run}`, {operator: hub.operator})}
                    </Box>
                </Box>
            </Box>
        </Box>
    )
}
