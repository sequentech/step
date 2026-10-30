// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"

import {
    EBallotSelectionLabel,
    getBallotSelectionCount,
    getBallotSelectionLabel,
} from "../../services/ReviewSummary"
import {getSlateName, IBallotSlates} from "../../services/Slates"
import {selectBallotSelectionByElectionId} from "../../store/ballotSelections/ballotSelectionsSlice"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useAppSelector} from "../../store/hooks"

const Count = styled("p")(({theme}) => ({
    width: "fit-content",
    margin: "0 0 16px auto",
    padding: "10px 13px",
    border: "1px solid #e7eaee",
    borderRadius: "7px",
    background: theme.palette.lightBackground,
    color: theme.palette.brandColor,
    fontSize: "14px",
    whiteSpace: "nowrap",
    [theme.breakpoints.down("sm")]: {
        display: "none",
    },
}))

const Label = styled("p")(({theme}) => ({
    "display": "flex",
    "flexWrap": "wrap",
    "alignItems": "center",
    "gap": "6px 12px",
    "margin": "0 0 18px",
    "overflowWrap": "anywhere",
    "& strong": {fontSize: "14px"},
    "& span": {fontSize: "13px", color: theme.palette.customGrey.dark},
    [theme.breakpoints.down("sm")]: {
        "& strong": {fontSize: "13px"},
        "& span": {fontSize: "12px"},
    },
}))

export interface SlateBallotSummaryProps {
    ballotStyle: IBallotStyle
    slates: IBallotSlates
    defaultLanguage?: string
}

const useBallotCount = (ballotStyle: IBallotStyle) => {
    const selection = useAppSelector(selectBallotSelectionByElectionId(ballotStyle.election_id))
    return {selection, count: getBallotSelectionCount(ballotStyle.ballot_eml.contests, selection)}
}

/** How many candidates the ballot holds, next to the ballot's title. */
export const SlateBallotCount: React.FC<{ballotStyle: IBallotStyle}> = ({ballotStyle}) => {
    const {t} = useTranslation()
    const {count} = useBallotCount(ballotStyle)

    return (
        <Count className="slate-selection-count">
            {t("slates.count.short", {selected: count.selected, seats: count.seats})}
        </Count>
    )
}

/** What the ballot holds as a whole, above the individual candidates. */
export const SlateSelectionLabel: React.FC<SlateBallotSummaryProps> = ({
    ballotStyle,
    slates,
    defaultLanguage,
}) => {
    const {t, i18n} = useTranslation()
    const {selection, count} = useBallotCount(ballotStyle)
    const label = getBallotSelectionLabel(slates, selection)
    const text =
        label.kind === EBallotSelectionLabel.SLATE && label.slate
            ? getSlateName(label.slate, i18n.language, defaultLanguage)
            : t(`slates.count.${label.kind}`)

    return (
        <Label className={`slate-selection-label slate-selection-label-${label.kind}`}>
            <strong className="slate-selection-label-name">{text}</strong>
            <span className="slate-selection-label-count">
                {t("slates.count.long", {selected: count.selected, seats: count.seats})}
            </span>
        </Label>
    )
}
