// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Look it up elsewhere" (design board FindElsewhere): a collapsed section that
// lists the explorers the election's policy names, each opening this ballot, or
// this log entry, in a record that someone else reads. Those run by others come
// first and are tagged Independent; the election authority's own come last.

import React, {useId, useState} from "react"
import {Box, ButtonBase} from "@mui/material"
import {useTranslation} from "react-i18next"
import {groupBallotId} from "../../services/vitrine/ballotId"
import {explorerLink, IExplorer} from "./explorers"
import {EStatusTone, StatusTag} from "./StatusTag"

const COLORS = {
    border: "#E7EAEE",
    text: "#191D23",
    navy: "#0F054C",
    quiet: "#64748B",
} as const

export enum EFindWhat {
    BALLOT = "ballot",
    ENTRY = "entry",
}

export interface IFindElsewhereProps {
    explorers: IExplorer[]
    /** The policy's ID, `sha256:...`. */
    policyId: string | undefined
    /** The name of the log in the policy. */
    log: string
    /** Whether the explorers open a ballot, or a log entry. */
    what?: EFindWhat
    /** The full Tracking ID, to find the ballot by it. */
    ballotId?: string
    /** The entry's index in the log, to open the entry by it. */
    entry?: number
}

export const FindElsewhere: React.FC<IFindElsewhereProps> = ({
    explorers,
    policyId,
    log,
    what = EFindWhat.BALLOT,
    ballotId,
    entry,
}) => {
    const {t} = useTranslation()
    const [open, setOpen] = useState(false)
    const listId = useId()
    if (!policyId || explorers.length === 0) return null

    const independent = explorers.filter((explorer) => explorer.independent).length
    const target = {
        policyId,
        log,
        ...(entry !== undefined ? {entry} : {}),
        ...(entry === undefined && ballotId ? {find: groupBallotId(ballotId)} : {}),
    }

    return (
        <Box
            component="section"
            className="find-elsewhere"
            sx={{
                boxSizing: "border-box",
                width: "100%",
                border: `1px solid ${COLORS.border}`,
                borderRadius: "4px",
                color: COLORS.text,
            }}
        >
            <Box component="h3" className="find-elsewhere-heading" sx={{m: 0}}>
                <ButtonBase
                    className="find-elsewhere-toggle"
                    aria-expanded={open}
                    aria-controls={open ? listId : undefined}
                    onClick={() => setOpen(!open)}
                    sx={{
                        "boxSizing": "border-box",
                        "width": "100%",
                        "minHeight": 48,
                        "p": "8px 14px",
                        "borderRadius": "4px",
                        "gap": "10px",
                        "justifyContent": "flex-start",
                        "textAlign": "left",
                        "&.Mui-focusVisible": {
                            outline: `2px solid ${COLORS.navy}`,
                            outlineOffset: "-2px",
                        },
                    }}
                >
                    <svg
                        className="find-elsewhere-icon"
                        width="18"
                        height="18"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        aria-hidden="true"
                        style={{flex: "none"}}
                    >
                        <circle className="find-elsewhere-lens" cx="11" cy="11" r="6.5" />
                        <path className="find-elsewhere-handle" d="M16 16l4.5 4.5" />
                    </svg>
                    <Box
                        component="span"
                        className="find-elsewhere-title"
                        sx={{flex: 1, fontSize: "15px", lineHeight: "22px", fontWeight: 700}}
                    >
                        {t("vitrine.elsewhere.title")}
                    </Box>
                    <Box
                        component="span"
                        className="find-elsewhere-count"
                        sx={{fontSize: "13px", lineHeight: "18px", color: COLORS.quiet}}
                    >
                        {t("vitrine.elsewhere.count", {count: explorers.length, independent})}
                    </Box>
                    <svg
                        className="find-elsewhere-chevron"
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2.2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        aria-hidden="true"
                        style={{flex: "none", transform: open ? "rotate(180deg)" : "none"}}
                    >
                        <path className="find-elsewhere-chevron-path" d="M6 9l6 6 6-6" />
                    </svg>
                </ButtonBase>
            </Box>
            {open ? (
                <Box id={listId} className="find-elsewhere-list">
                    <Box
                        component="p"
                        className="find-elsewhere-intro"
                        sx={{
                            m: 0,
                            p: "0 14px 10px",
                            fontSize: "13px",
                            lineHeight: "18px",
                            color: COLORS.quiet,
                        }}
                    >
                        {t(
                            what === EFindWhat.ENTRY
                                ? "vitrine.elsewhere.introEntry"
                                : "vitrine.elsewhere.introBallot"
                        )}
                    </Box>
                    {explorers.map((explorer) => (
                        <Box
                            component="a"
                            key={`${explorer.kind}-${explorer.url}`}
                            className={`find-elsewhere-link find-elsewhere-${explorer.kind}`}
                            href={explorerLink(explorer.url, target)}
                            target="_blank"
                            rel="noopener noreferrer"
                            sx={{
                                "boxSizing": "border-box",
                                "display": "flex",
                                "alignItems": "center",
                                "gap": "12px",
                                "minHeight": 52,
                                "p": "6px 14px",
                                "borderTop": `1px solid ${COLORS.border}`,
                                "color": COLORS.navy,
                                "textDecoration": "none",
                                "&:hover .find-elsewhere-name": {
                                    textDecoration: "underline",
                                    textUnderlineOffset: "2px",
                                },
                                "&:focus-visible": {
                                    outline: `2px solid ${COLORS.navy}`,
                                    outlineOffset: "-2px",
                                    borderRadius: "4px",
                                },
                            }}
                        >
                            <Box className="find-elsewhere-text" sx={{flex: 1, minWidth: 0}}>
                                <Box
                                    className="find-elsewhere-head"
                                    sx={{
                                        display: "flex",
                                        alignItems: "center",
                                        gap: "8px",
                                        flexWrap: "wrap",
                                    }}
                                >
                                    <Box
                                        component="span"
                                        className="find-elsewhere-name"
                                        sx={{fontSize: "15px", lineHeight: "22px", fontWeight: 500}}
                                    >
                                        {explorer.kind === "operator"
                                            ? t("vitrine.elsewhere.operatorName")
                                            : t("vitrine.elsewhere.name", {name: explorer.name})}
                                    </Box>
                                    <StatusTag
                                        label={t(
                                            explorer.independent
                                                ? "vitrine.elsewhere.independent"
                                                : "vitrine.elsewhere.operator"
                                        )}
                                        tone={
                                            explorer.independent
                                                ? EStatusTone.SUCCESS
                                                : EStatusTone.NEUTRAL
                                        }
                                    />
                                </Box>
                                <Box
                                    className="find-elsewhere-who"
                                    sx={{fontSize: "13px", lineHeight: "18px", color: COLORS.quiet}}
                                >
                                    {t(`vitrine.elsewhere.who.${explorer.kind}`)}
                                </Box>
                            </Box>
                            <svg
                                className="find-elsewhere-external"
                                width="18"
                                height="18"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                strokeWidth="2"
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                role="img"
                                aria-label={t("vitrine.elsewhere.newTab")}
                                style={{flex: "none"}}
                            >
                                <path className="find-elsewhere-external-arrow" d="M14 4h6v6" />
                                <path className="find-elsewhere-external-line" d="M20 4l-9 9" />
                                <path
                                    className="find-elsewhere-external-box"
                                    d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4"
                                />
                            </svg>
                        </Box>
                    ))}
                    <Box
                        className="find-elsewhere-foot"
                        sx={{
                            p: "10px 14px 12px",
                            borderTop: `1px solid ${COLORS.border}`,
                            fontSize: "13px",
                            lineHeight: "18px",
                            color: COLORS.quiet,
                        }}
                    >
                        {t("vitrine.elsewhere.foot")}
                    </Box>
                </Box>
            ) : null}
        </Box>
    )
}
