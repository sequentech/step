// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Button, Typography} from "@mui/material"
import {faPen} from "@fortawesome/free-solid-svg-icons"
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

/**
 * How many candidates a contest holds, whether any choice is left unused, and
 * the way back to edit it.
 */
export const ReviewContestFooter: React.FC<ReviewContestFooterProps> = ({contest, count, to}) => {
    const {t, i18n} = useTranslation()
    const state: IEditContestState = {editContestId: contest.id}
    const isUnfilled = count.selected < count.max

    return (
        <Box
            className={`review-contest-footer${isUnfilled ? " review-contest-unfilled" : ""}`}
            sx={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "12px",
                margin: "12px 0",
                overflowWrap: "anywhere",
            }}
        >
            <Box className="review-contest-status" sx={{minWidth: 0}}>
                <Typography
                    className="review-contest-count"
                    variant="body2"
                    sx={{margin: 0, fontSize: "12px", color: "customGrey.dark"}}
                >
                    {t("slates.review.contestCount", {selected: count.selected, max: count.max})}
                </Typography>
                {count.selected === 0 ? (
                    <Typography
                        className="review-contest-blank"
                        variant="body2"
                        sx={{margin: "2px 0 0", fontSize: "14px", color: "#8a6b23"}}
                    >
                        {t("slates.review.unfilled")}
                    </Typography>
                ) : null}
            </Box>
            <Button
                className="review-contest-edit"
                component={RouterLink}
                to={to}
                state={state}
                variant="secondary"
                sx={{minHeight: "44px", minWidth: "44px", width: "44px", padding: "9px"}}
                aria-label={t("slates.review.editLabel", {
                    contest: translate(contest, "name", i18n.language) ?? "",
                })}
            >
                <Icon className="review-contest-edit-icon" icon={faPen} size="sm" />
            </Button>
        </Box>
    )
}
