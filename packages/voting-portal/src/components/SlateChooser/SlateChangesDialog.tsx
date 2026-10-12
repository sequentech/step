// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {translate} from "@sequentech/ui-core"
import type {ICandidate, IContest} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"
import type {ISlateChoices} from "../../services/SlateChoices"

export interface SlateChangesDialogProps {
    className: string
    /** The ballot to confirm, or undefined while the dialog is closed. */
    pending?: ISlateChoices
    /** The contests of the ballot, to name the contests and candidates. */
    contests: Array<IContest>
    title: string
    content: string
    ok: string
    cancel: string
    handleClose: (confirmed: boolean) => void
}

/**
 * Before and after of every contest that loses a choice. A contest left
 * without any choice says so instead of naming a candidate.
 */
export const SlateChangesDialog: React.FC<SlateChangesDialogProps> = ({
    className,
    pending,
    contests,
    title,
    content,
    ok,
    cancel,
    handleClose,
}) => {
    const {t, i18n} = useTranslation()

    const candidateNames = (candidates: ICandidate[], ids: string[]): string =>
        ids
            .map((id) => {
                const candidate = candidates.find((entry) => entry.id === id)
                return candidate ? (translate(candidate, "name", i18n.language) ?? id) : id
            })
            .join(", ")

    const isLeftBlank = (contestId: string): boolean =>
        !pending?.selection
            .find((entry) => entry.contest_id === contestId)
            ?.choices.some((choice) => choice.selected > -1)

    return (
        <Dialog
            className={className}
            open={!!pending}
            handleClose={handleClose}
            title={title}
            ok={ok}
            cancel={cancel}
            variant="action"
        >
            <Box className="slate-changes" sx={{overflowWrap: "anywhere"}}>
                <Typography className="slate-replace-content">{content}</Typography>
                {(pending?.changes ?? [])
                    .filter((change) => change.removed.length > 0)
                    .map((change) => {
                        const contest = contests.find((entry) => entry.id === change.contestId)
                        if (!contest) {
                            return null
                        }
                        return (
                            <Box
                                key={change.contestId}
                                className="slate-replace-contest"
                                sx={{marginTop: 2}}
                            >
                                <Typography
                                    className="slate-replace-contest-name"
                                    variant="h6"
                                    component="h3"
                                >
                                    {translate(contest, "name", i18n.language) ?? contest.id}
                                </Typography>
                                <Typography className="slate-replace-removed">
                                    {t("slates.apply.replaceDialog.removed")}{" "}
                                    <strong className="slate-replace-candidates">
                                        {candidateNames(contest.candidates, change.removed)}
                                    </strong>
                                </Typography>
                                {change.added.length > 0 ? (
                                    <Typography className="slate-replace-added">
                                        {t("slates.apply.replaceDialog.added")}{" "}
                                        <strong className="slate-replace-candidates">
                                            {candidateNames(contest.candidates, change.added)}
                                        </strong>
                                    </Typography>
                                ) : isLeftBlank(change.contestId) ? (
                                    <Typography className="slate-replace-added slate-replace-none">
                                        {t("slates.apply.replaceDialog.added")}{" "}
                                        <strong className="slate-replace-candidates">
                                            {t("slates.noCandidate")}
                                        </strong>
                                    </Typography>
                                ) : null}
                            </Box>
                        )
                    })}
            </Box>
        </Dialog>
    )
}
