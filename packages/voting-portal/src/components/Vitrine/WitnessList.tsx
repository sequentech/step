// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Who signed" and "Which hubs" (design board WitnessList): a collapsed link
// under the step that opens a list of the policy's witnesses or hubs. Each row
// says who runs the signer, as the policy states, and when it signed or
// anchored what this browser checked. No row is called independent and none
// carries a badge: the footnote states the rules, and the voter judges.
// A signer whose site the policy names gets "Check on its site".

import React, {useId, useState} from "react"
import {Box, ButtonBase} from "@mui/material"
import {useTranslation} from "react-i18next"
import {CHECK_COLORS} from "./BallotStatusIcon"
import {ESignerKind, ESignerState, ISignerList, ISignerRow} from "../../services/vitrine/signers"
import {clockSeconds} from "../../services/vitrine/statusRows"

const COLORS = {
    border: "#E7EAEE",
    text: "#191D23",
    navy: "#0F054C",
    quiet: "#64748B",
    hover: "#F1F5F9",
} as const

/** Below this width the time and the link go under "Run by". */
const NARROW = 360

const KEYS = "vitrine.signers"

/** The board's toggle sits under the step's title: its indent is the title's
 * minus the toggle's own padding. */
const TOGGLE_PADDING = 6

const checkColor = (kind: ESignerKind, state: ESignerState): string => {
    if (state !== ESignerState.DONE) return CHECK_COLORS.LIGHT
    return kind === ESignerKind.HUBS ? CHECK_COLORS.BLUE : CHECK_COLORS.GREY
}

const Check: React.FC<{color: string; dashed: boolean}> = ({color, dashed}) => (
    <svg
        className="witness-list-check"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke={color}
        strokeWidth="2.4"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        style={{flex: "none"}}
    >
        <path
            className="witness-list-check-path"
            d="M4.5 12.5l4.75 4.75L19.5 7"
            strokeDasharray={dashed ? "2.5 3" : "none"}
        />
    </svg>
)

const ExternalIcon: React.FC = () => (
    <svg
        className="witness-list-external"
        width="12"
        height="12"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.4"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
    >
        <path d="M14 4h6v6" />
        <path d="M20 4l-9 9" />
        <path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" />
    </svg>
)

export interface IWitnessListProps {
    /** The signers to list; nothing is shown without them. */
    list?: ISignerList
    /** Minutes that a hub still to anchor has, from the policy. */
    minutes: number
    /** Pixels from the left of the step row to its title, to line the toggle up with it. */
    indent?: number
    /** Whether the list starts open. */
    defaultOpen?: boolean
}

export const WitnessList: React.FC<IWitnessListProps> = ({
    list,
    minutes,
    indent = 0,
    defaultOpen = false,
}) => {
    const {t} = useTranslation()
    const [open, setOpen] = useState(defaultOpen)
    const listId = useId()
    if (!list) return null

    const hubs = list.kind === ESignerKind.HUBS
    const linked = list.rows.some((row) => row.site)

    const nameOf = (row: ISignerRow): string =>
        row.standIn ? t(`${KEYS}.standIn.name`) : (row.name ?? "")

    const whoOf = (row: ISignerRow): string => {
        if (row.standIn) return t(`${KEYS}.standIn.who`)
        if (row.alsoRuns.length > 0) {
            return t(`${KEYS}.runByAlso`, {party: row.party, hubs: row.alsoRuns.join(", ")})
        }
        return t(`${KEYS}.runBy`, {party: row.party})
    }

    const whenOf = (row: ISignerRow): string => {
        const time = row.at === undefined ? undefined : clockSeconds(row.at * 1000)
        switch (row.state) {
            case ESignerState.DONE:
                if (hubs) {
                    return time === undefined
                        ? t(`${KEYS}.anchoredNoTime`)
                        : t(`${KEYS}.anchored`, {time})
                }
                return time === undefined ? t(`${KEYS}.signedNoTime`) : t(`${KEYS}.signed`, {time})
            case ESignerState.LATE:
                return t(`${KEYS}.late`)
            default:
                return hubs ? t(`${KEYS}.within`, {minutes}) : t(`${KEYS}.notYet`)
        }
    }

    const label = `${KEYS}.${list.kind}.${open ? "close" : "open"}`

    return (
        <Box className={`witness-list witness-list-${list.kind}`} data-open={open}>
            <ButtonBase
                className="witness-list-toggle"
                aria-expanded={open}
                aria-controls={open ? listId : undefined}
                onClick={() => setOpen(!open)}
                sx={{
                    "display": "inline-flex",
                    "alignItems": "center",
                    "gap": "4px",
                    "minHeight": 32,
                    "m": `2px 0 0 ${Math.max(0, indent - TOGGLE_PADDING)}px`,
                    "p": `0 ${TOGGLE_PADDING}px`,
                    "borderRadius": "4px",
                    "fontSize": "13px",
                    "lineHeight": "18px",
                    "fontWeight": 500,
                    "color": COLORS.navy,
                    "&:hover": {backgroundColor: COLORS.hover},
                    "&.Mui-focusVisible": {outline: `2px solid ${COLORS.navy}`, outlineOffset: 0},
                }}
            >
                {t(label)}
                <svg
                    className="witness-list-chevron"
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2.4"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                    style={{flex: "none", transform: open ? "rotate(180deg)" : "none"}}
                >
                    <path d="M6 9l6 6 6-6" />
                </svg>
            </ButtonBase>
            {open ? (
                <>
                    <Box
                        component="ul"
                        id={listId}
                        className="witness-list-rows"
                        sx={{
                            m: "6px 0 0",
                            p: 0,
                            listStyle: "none",
                            border: `1px solid ${COLORS.border}`,
                            borderRadius: "4px",
                            containerType: "inline-size",
                            [`@container (max-width: ${NARROW}px)`]: {
                                "& .witness-list-row": {alignItems: "flex-start"},
                                "& .witness-list-check": {mt: "2px"},
                                "& .witness-list-side": {
                                    flex: "0 0 100%",
                                    flexDirection: "row",
                                    flexWrap: "wrap",
                                    alignItems: "center",
                                    gap: "2px 12px",
                                    pl: "26px",
                                    boxSizing: "border-box",
                                },
                            },
                        }}
                    >
                        {list.rows.map((row, n) => {
                            const done = row.state === ESignerState.DONE
                            return (
                                <Box
                                    component="li"
                                    key={row.name ?? "stand-in"}
                                    className="witness-list-row"
                                    data-state={row.state}
                                    sx={{
                                        display: "flex",
                                        flexWrap: "wrap",
                                        alignItems: "center",
                                        gap: "4px 10px",
                                        minHeight: 52,
                                        p: "10px 12px",
                                        boxSizing: "border-box",
                                        borderTop: n ? `1px solid ${COLORS.border}` : 0,
                                    }}
                                >
                                    <Check
                                        color={checkColor(list.kind, row.state)}
                                        dashed={!done}
                                    />
                                    <Box className="witness-list-text" sx={{flex: 1, minWidth: 0}}>
                                        <Box
                                            className="witness-list-name"
                                            sx={{
                                                fontSize: "14px",
                                                lineHeight: "20px",
                                                fontWeight: 500,
                                                color: done ? COLORS.text : COLORS.quiet,
                                            }}
                                        >
                                            {nameOf(row)}
                                        </Box>
                                        <Box
                                            className="witness-list-who"
                                            sx={{
                                                mt: "2px",
                                                fontSize: "12px",
                                                lineHeight: "16px",
                                                color: COLORS.quiet,
                                            }}
                                        >
                                            {whoOf(row)}
                                        </Box>
                                    </Box>
                                    <Box
                                        className="witness-list-side"
                                        sx={{
                                            flex: "none",
                                            display: "flex",
                                            flexDirection: "column",
                                            alignItems: "flex-end",
                                            gap: "4px",
                                        }}
                                    >
                                        <Box
                                            component="span"
                                            className="witness-list-when"
                                            sx={{
                                                fontSize: "12px",
                                                lineHeight: "16px",
                                                whiteSpace: "nowrap",
                                                color:
                                                    row.state === ESignerState.LATE
                                                        ? CHECK_COLORS.AMBER
                                                        : COLORS.quiet,
                                            }}
                                        >
                                            {whenOf(row)}
                                        </Box>
                                        {row.site ? (
                                            <Box
                                                component="a"
                                                className="witness-list-site"
                                                href={row.site}
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                aria-label={t(
                                                    `${KEYS}.${hubs ? "checkHub" : "checkWitness"}`,
                                                    {name: row.name}
                                                )}
                                                sx={{
                                                    "display": "inline-flex",
                                                    "alignItems": "center",
                                                    "gap": "3px",
                                                    "minHeight": 24,
                                                    "fontSize": "12px",
                                                    "lineHeight": "16px",
                                                    "fontWeight": 500,
                                                    "color": COLORS.navy,
                                                    "textDecoration": "underline",
                                                    "textUnderlineOffset": "2px",
                                                    "whiteSpace": "nowrap",
                                                    "&:focus-visible": {
                                                        outline: `2px solid ${COLORS.navy}`,
                                                        outlineOffset: "1px",
                                                        borderRadius: "2px",
                                                    },
                                                }}
                                            >
                                                {t(`${KEYS}.check`)}
                                                <ExternalIcon />
                                            </Box>
                                        ) : null}
                                    </Box>
                                </Box>
                            )
                        })}
                    </Box>
                    <Box
                        component="p"
                        className="witness-list-foot"
                        sx={{
                            m: "8px 0 0",
                            fontSize: "12px",
                            lineHeight: "16px",
                            color: COLORS.quiet,
                        }}
                    >
                        {[
                            t(list.rule.key, list.rule.values),
                            ...(linked ? [t(`${KEYS}.linksNote`)] : []),
                        ].join(" ")}
                    </Box>
                </>
            ) : null}
        </Box>
    )
}
