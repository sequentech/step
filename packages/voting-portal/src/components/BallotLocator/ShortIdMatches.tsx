// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballots that share a short ID, to choose from (design boards
// ShortIdMatches and ShortIdStates): up to five, each with its third group
// highlighted, its last step and when it happened; above five only a box that
// asks for the third group. The ballot whose receipt this device keeps comes
// first and is marked, and the look-alikes stay listed. A short ID finds
// ballots; it never proves one is yours, so the text says so.

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {Link} from "react-router-dom"
import {Box, ButtonBase, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {BALLOT_ID_GROUP, shortBallotId} from "../../services/vitrine/ballotId"
import {EShortIdKind, IShortIdMatch, IShortIdResult} from "../../services/vitrine/shortId"
import {EStatusTone, StatusTag} from "../Vitrine/StatusTag"
import {MONO, PALETTE, PHONE} from "./trackerStyle"

const THIRD_ID = "short-id-third"
const WHY_ID = "short-id-why"

const rowStyle = {
    "display": "flex",
    "alignItems": "center",
    "gap": "14px",
    "width": "100%",
    "padding": "12px 14px",
    "border": `1px solid ${PALETTE.border}`,
    "borderRadius": "12px",
    "backgroundColor": "#fff",
    "color": PALETTE.navy,
    "textAlign": "left",
    "textDecoration": "none",
    "transition": "border-color .15s, box-shadow .15s",
    "&:hover": {borderColor: PALETTE.navy, boxShadow: "0 2px 10px rgba(15, 5, 76, 0.08)"},
    "&:focus-visible": {outline: `2px solid ${PALETTE.navy}`, outlineOffset: 2},
    "& + &": {marginTop: "8px"},
    [PHONE]: {gap: "10px"},
} as const

const RowLink = styled(Link)(rowStyle)
const RowButton = styled(ButtonBase)(rowStyle)

const Arrow: React.FC = () => (
    <svg
        className="short-id-arrow"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.4"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        focusable="false"
        style={{flex: "none"}}
    >
        <path d="M9 5l7 7-7 7" />
    </svg>
)

/** `FTBE-MHRX-71J7-5ZBD-…` with the third group highlighted. */
const IdStart: React.FC<{match: IShortIdMatch}> = ({match}) => (
    <Box
        component="span"
        className="short-id-start"
        sx={{
            font: `400 14px/20px ${MONO}`,
            color: PALETTE.quiet,
            whiteSpace: "nowrap",
            overflowWrap: "normal",
        }}
    >
        <b style={{color: PALETTE.text, fontWeight: 700}}>{shortBallotId(match.trackingId)}</b>-
        <mark
            className="short-id-third-group"
            style={{
                background: "#FEF3C7",
                color: PALETTE.text,
                fontWeight: 700,
                borderRadius: 3,
                padding: "0 2px",
            }}
        >
            {match.third}
        </mark>
        -{match.fourth}-…
    </Box>
)

export interface IShortIdMatchesProps {
    result: IShortIdResult
    /** Where a ballot opens, for a list of links. */
    hrefOf?: (match: IShortIdMatch) => string
    /** Opens a ballot, for a list of buttons. */
    onOpen?: (match: IShortIdMatch) => void
    /** POSIX seconds to the text shown. */
    formatTime: (seconds: number) => string
    /** The voter typed the third group: what is looked up now, as characters. */
    onNarrow: (typed: string) => void
}

/** The matches live in a region that is always there, so that the voter hears
 * of them when they appear. */
export const ShortIdMatches: React.FC<IShortIdMatchesProps> = ({
    result,
    hrefOf,
    onOpen,
    formatTime,
    onNarrow,
}) => {
    const {t} = useTranslation()
    const start = shortBallotId(result.typed)
    const thirdTyped = result.typed.slice(2 * BALLOT_ID_GROUP, 3 * BALLOT_ID_GROUP)
    const [third, setThird] = useState(thirdTyped)
    const many = result.kind === EShortIdKind.MANY
    const mine = result.matches.find((match) => match.onDevice)
    const others = result.total - (mine ? 1 : 0)
    const narrowed = result.typed.length >= 3 * BALLOT_ID_GROUP
    const startText = narrowed ? shortBallotId(result.typed.slice(0, 3 * BALLOT_ID_GROUP)) : start
    // A third group is asked unless the device already settles it.
    const asks = !mine && (many || result.matches.length > 0)

    const head = mine
        ? others === 0
            ? t("ballotLocator.shortId.head.mineAlone", {start: startText})
            : t("ballotLocator.shortId.head.mine", {count: others, start: startText})
        : result.truncated
          ? t("ballotLocator.shortId.head.atLeast", {count: result.total, start: startText})
          : result.total === 1
            ? t("ballotLocator.shortId.head.one", {start: startText})
            : t("ballotLocator.shortId.head.other", {count: result.total, start: startText})
    const sub = mine
        ? t("ballotLocator.shortId.sub.mine")
        : many
          ? t("ballotLocator.shortId.sub.many")
          : result.total === 1
            ? t("ballotLocator.shortId.sub.one")
            : t("ballotLocator.shortId.sub.few")
    const why = mine
        ? others > 0
            ? t("ballotLocator.shortId.why.mine")
            : ""
        : many
          ? t("ballotLocator.shortId.why.many")
          : result.total === 1
            ? ""
            : t("ballotLocator.shortId.why.few")
    const label = many || narrowed ? "ballotLocator.shortId.third" : "ballotLocator.shortId.thirdOr"

    const row = (match: IShortIdMatch) => {
        const key = match.trackingId
        const content = (
            <>
                {match.onDevice ? (
                    <svg
                        className="short-id-device-check"
                        width="22"
                        height="22"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="#1A5FD6"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        aria-hidden="true"
                        focusable="false"
                        style={{overflow: "visible", flex: "none"}}
                    >
                        <path d="M1.5 12.75l4.5 4.5L16.25 7" />
                        <path d="M10.75 15.75l1.5 1.5L22.5 7" />
                    </svg>
                ) : null}
                <Box component="span" sx={{flex: 1, minWidth: 0}}>
                    <IdStart match={match} />
                    <Box
                        component="span"
                        className="short-id-when"
                        sx={{
                            display: "block",
                            fontSize: 13,
                            lineHeight: "18px",
                            color: match.onDevice ? "#1E3A8A" : PALETTE.quiet,
                        }}
                    >
                        {match.onDevice
                            ? t("ballotLocator.shortId.deviceMatches")
                            : match.at === undefined
                              ? ""
                              : t(
                                    match.audited
                                        ? "ballotLocator.matches.audited"
                                        : "ballotLocator.matches.cast",
                                    {time: formatTime(match.at)}
                                )}
                    </Box>
                </Box>
                {match.onDevice ? (
                    <StatusTag
                        label={t("ballotLocator.shortId.onDevice")}
                        tone={EStatusTone.INFO}
                    />
                ) : match.at !== undefined ? (
                    <StatusTag
                        label={t(
                            match.audited ? "vitrine.status.tag.audited" : "vitrine.status.tag.cast"
                        )}
                        tone={match.audited ? EStatusTone.NEUTRAL : EStatusTone.SUCCESS}
                    />
                ) : null}
                <Arrow />
            </>
        )
        const common = {
            className: `short-id-match${match.onDevice ? " short-id-match-device" : ""}`,
            style: match.onDevice ? {borderColor: "#1A5FD6", background: "#F5F9FF"} : undefined,
        }
        return hrefOf ? (
            <RowLink key={key} to={hrefOf(match)} {...common}>
                {content}
            </RowLink>
        ) : (
            <RowButton key={key} onClick={() => onOpen?.(match)} {...common}>
                {content}
            </RowButton>
        )
    }

    const find = () => {
        const chars = third.replace(/[^0-9A-Z]/g, "")
        if (chars.length === BALLOT_ID_GROUP) {
            onNarrow(result.typed.slice(0, 2 * BALLOT_ID_GROUP) + chars)
        }
    }

    return (
        <Box
            className="short-id-matches"
            role="status"
            sx={{width: "100%", textAlign: "left", marginTop: "24px"}}
        >
            <Typography
                className="short-id-head"
                component="p"
                sx={{margin: "0 0 4px 2px", fontSize: 16, lineHeight: "22px", fontWeight: 700}}
            >
                {head}
            </Typography>
            <Typography
                className="short-id-sub"
                component="p"
                sx={{
                    margin: "0 0 12px 2px",
                    fontSize: 14,
                    lineHeight: "20px",
                    color: PALETTE.muted,
                }}
            >
                {sub}
            </Typography>
            <Box className="short-id-list">{result.matches.map(row)}</Box>
            {asks ? (
                <>
                    <Box
                        component="label"
                        htmlFor={THIRD_ID}
                        className="short-id-third-label"
                        sx={{
                            display: "block",
                            margin: "16px 0 6px 2px",
                            fontSize: 13,
                            fontWeight: 600,
                            color: PALETTE.muted,
                        }}
                    >
                        {t(label)}
                    </Box>
                    <Box
                        className="short-id-third-box"
                        sx={{
                            "display": "flex",
                            "alignItems": "center",
                            "gap": 0,
                            "minHeight": 48,
                            "padding": "0 6px 0 14px",
                            "border": "2px solid #CBD5E1",
                            "borderRadius": "12px",
                            "backgroundColor": "#fff",
                            "font": `600 17px ${MONO}`,
                            "letterSpacing": ".04em",
                            "color": PALETTE.navy,
                            "&:focus-within": {
                                borderColor: PALETTE.navy,
                                boxShadow: "0 0 0 4px rgba(15, 5, 76, 0.1)",
                            },
                        }}
                    >
                        <span>{start}-</span>
                        <input
                            id={THIRD_ID}
                            className="short-id-third-input"
                            value={third}
                            maxLength={BALLOT_ID_GROUP}
                            placeholder="····"
                            autoComplete="off"
                            spellCheck={false}
                            aria-describedby={why ? WHY_ID : undefined}
                            onChange={(event) =>
                                setThird(
                                    event.target.value
                                        .toUpperCase()
                                        .replace(/[^0-9A-Z]/g, "")
                                        .slice(0, BALLOT_ID_GROUP)
                                )
                            }
                            onKeyDown={(event) => {
                                if (event.key === "Enter") find()
                            }}
                            style={{
                                width: 72,
                                height: 40,
                                marginLeft: 2,
                                padding: "0 4px",
                                border: 0,
                                borderBottom: "2px dashed #94A3B8",
                                outline: "none",
                                background: "transparent",
                                font: "inherit",
                                letterSpacing: "inherit",
                                color: PALETTE.navy,
                                textTransform: "uppercase",
                            }}
                        />
                        <ButtonBase
                            className="short-id-find-button"
                            disabled={third.length !== BALLOT_ID_GROUP}
                            onClick={find}
                            sx={{
                                "marginLeft": "auto",
                                "minHeight": 40,
                                "padding": "4px 16px",
                                "border": `1px solid ${PALETTE.navy}`,
                                "borderRadius": "8px",
                                "backgroundColor": PALETTE.navy,
                                "color": "#fff",
                                "fontFamily": "inherit",
                                "fontSize": 15,
                                "fontWeight": 500,
                                "&.Mui-disabled": {
                                    background: PALETTE.border,
                                    borderColor: PALETTE.border,
                                    color: "#757575",
                                },
                            }}
                        >
                            {t("ballotLocator.shortId.find")}
                        </ButtonBase>
                    </Box>
                </>
            ) : null}
            {why ? (
                <Typography
                    id={WHY_ID}
                    className="short-id-why"
                    component="p"
                    sx={{
                        margin: "10px 0 0 2px",
                        fontSize: 13,
                        lineHeight: "18px",
                        color: PALETTE.quiet,
                    }}
                >
                    {why}
                </Typography>
            ) : null}
        </Box>
    )
}

/** The note under a search the bucket route could not answer: the election's
 * own server was asked, and it sees the ID. */
export const ShortIdServerNote: React.FC = () => {
    const {t} = useTranslation()
    return (
        <Typography
            className="short-id-server-note"
            role="status"
            component="p"
            sx={{margin: "12px 2px 0", fontSize: 13, lineHeight: "18px", color: PALETTE.quiet}}
        >
            {t("ballotLocator.shortId.serverSees")}
        </Typography>
    )
}
