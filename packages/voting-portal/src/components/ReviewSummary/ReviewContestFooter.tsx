// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Button, Typography} from "@mui/material"
import {faPenToSquare} from "@fortawesome/free-solid-svg-icons"
import {Icon} from "@sequentech/ui-essentials"
import {IContest, translate} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"
import {Link as RouterLink} from "react-router-dom"
import {IEditContestState} from "../../services/EditContest"
import {IContestReviewCount} from "../../services/ReviewSummary"

export interface ReviewContestFooterProps {
    contest: IContest
    count: IContestReviewCount
    /** The voting screen of the election. */
    to: string
}

export const ReviewContestFooter: React.FC<ReviewContestFooterProps> = ({contest, count, to}) => {
    const {t, i18n} = useTranslation()
    const state: IEditContestState = {editContestId: contest.id}

    return (
        <Box
            className="review-contest-footer"
            sx={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                flexWrap: "wrap",
                gap: "8px",
                margin: "12px 0",
            }}
        >
            <Typography className="review-contest-count" variant="body2" sx={{margin: 0}}>
                {t("slates.review.contestCount", {selected: count.selected, max: count.max})}
            </Typography>
            <Button
                className="review-contest-edit"
                component={RouterLink}
                to={to}
                state={state}
                variant="secondary"
                sx={{minHeight: "44px"}}
                aria-label={t("slates.review.editLabel", {
                    contest: translate(contest, "name", i18n.language) ?? "",
                })}
            >
                <Icon className="review-contest-edit-icon" icon={faPenToSquare} size="sm" />
                <Box
                    className="review-contest-edit-label"
                    component="span"
                    sx={{paddingInlineStart: "8px"}}
                >
                    {t("slates.review.edit")}
                </Box>
            </Button>
        </Box>
    )
}
