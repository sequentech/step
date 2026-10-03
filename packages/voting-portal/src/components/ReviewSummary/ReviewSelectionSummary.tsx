// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useId} from "react"
import {Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {IBallotReviewSummary} from "../../services/ReviewSummary"
import {getSlateName} from "../../services/Slates"

const SummaryBox = styled("section")(({theme}) => ({
    margin: "16px 0 0",
    padding: "16px",
    borderRadius: "8px",
    border: `1px solid ${theme.palette.customGrey.light}`,
    backgroundColor: theme.palette.lightBackground,
}))

const SummaryList = styled("ul")({
    margin: "8px 0 0",
    paddingInlineStart: "20px",
    fontSize: "14px",
})

export interface ReviewSelectionSummaryProps {
    summary: IBallotReviewSummary
    defaultLanguage?: string
}

export const ReviewSelectionSummary: React.FC<ReviewSelectionSummaryProps> = ({
    summary,
    defaultLanguage,
}) => {
    const {t, i18n} = useTranslation()
    const titleId = useId()
    const hasLines = summary.slates.length > 0 || summary.independent > 0

    return (
        <SummaryBox className="review-selection-summary" aria-labelledby={titleId}>
            <Typography
                className="review-selection-title"
                component="h2"
                id={titleId}
                sx={{fontSize: "16px", fontWeight: 700, margin: 0}}
            >
                {t("slates.review.title")}
            </Typography>
            <Typography className="review-selection-total" variant="body2" sx={{margin: "8px 0 0"}}>
                {t("slates.review.total", {selected: summary.selected, seats: summary.seats})}
            </Typography>
            {hasLines ? (
                <SummaryList className="review-selection-lines">
                    {summary.slates.map(({slate, summary: slateSummary}) => (
                        <li
                            key={slate.id}
                            className={`review-selection-slate review-selection-slate-${slateSummary.status}`}
                        >
                            {t("slates.review.slate", {
                                slate: getSlateName(slate, i18n.language, defaultLanguage),
                                status: t(`slates.selection.${slateSummary.status}`, {
                                    selected: slateSummary.selected,
                                    total: slateSummary.total,
                                }),
                            })}
                        </li>
                    ))}
                    {summary.independent > 0 ? (
                        <li className="review-selection-independent">
                            {t("slates.review.independent", {count: summary.independent})}
                        </li>
                    ) : null}
                </SummaryList>
            ) : null}
            <Typography className="review-selection-note" variant="body2" sx={{margin: "8px 0 0"}}>
                {t("slates.review.note")}
            </Typography>
        </SummaryBox>
    )
}
