// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useId} from "react"
import {Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {EBallotSelectionLabel, IBallotReviewSummary} from "../../services/ReviewSummary"
import {getSlateName} from "../../services/Slates"

const SummaryBox = styled("section")(({theme}) => ({
    "margin": "23px 0 0",
    "padding": "21px",
    "borderRadius": "10px",
    "backgroundColor": theme.palette.lightBackground,
    "overflowWrap": "anywhere",
    "& .review-selection-total": {
        margin: "0 0 5px",
        fontSize: "30px",
        fontWeight: 600,
        letterSpacing: "-1px",
        color: theme.palette.brandColor,
    },
    "& .review-selection-seats": {
        fontSize: "16px",
        fontWeight: 400,
        letterSpacing: 0,
        color: theme.palette.customGrey.dark,
    },
    "& .review-selection-caption, & .review-selection-note": {
        margin: 0,
        fontSize: "14px",
        lineHeight: 1.55,
        color: theme.palette.customGrey.dark,
    },
    "& .review-selection-label": {
        display: "inline-flex",
        margin: "12px 0 0",
        padding: "4px 8px",
        borderRadius: "4px",
        background: theme.palette.white,
        color: theme.palette.customGrey.dark,
        fontSize: "12px",
    },
    "& .review-selection-note": {
        marginTop: "20px",
    },
}))

export interface ReviewSelectionSummaryProps {
    summary: IBallotReviewSummary
    defaultLanguage?: string
}

/** How many candidates the ballot holds and what it holds as a whole. */
export const ReviewSelectionSummary: React.FC<ReviewSelectionSummaryProps> = ({
    summary,
    defaultLanguage,
}) => {
    const {t, i18n} = useTranslation()
    const titleId = useId()
    const {label} = summary
    const labelText =
        label.kind === EBallotSelectionLabel.SLATE && label.slate
            ? getSlateName(label.slate, i18n.language, defaultLanguage)
            : t(`slates.count.${label.kind}`)

    return (
        <SummaryBox className="review-selection-summary" aria-labelledby={titleId}>
            <Typography
                className="review-selection-title"
                component="h2"
                id={titleId}
                sx={{fontSize: "16px", fontWeight: 700, margin: "0 0 16px"}}
            >
                {t("slates.review.title")}
            </Typography>
            <p className="review-selection-total">
                {summary.selected}{" "}
                <span className="review-selection-seats">
                    {t("slates.review.of", {seats: summary.seats})}
                </span>
            </p>
            <p className="review-selection-caption">{t("slates.review.selected")}</p>
            <span className={`review-selection-label review-selection-label-${label.kind}`}>
                {labelText}
            </span>
            <p className="review-selection-note">{t("slates.review.note")}</p>
        </SummaryBox>
    )
}
