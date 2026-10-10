// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useId, useMemo, useState} from "react"
import {Box, Button, Typography} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {translate} from "@sequentech/ui-core"
import type {ICandidate} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"
import {
    computeSlateChoices,
    ISlateChoices,
    SlateOverMaximumError,
    slateRemovesChoices,
} from "../../services/SlateChoices"
import {getSlateName, IResolvedSlate} from "../../services/Slates"
import {
    applySlateSelection,
    selectBallotSelectionByElectionId,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../../store/hooks"

export interface SlateApplyActionProps {
    ballotStyle: IBallotStyle
    slate: IResolvedSlate
}

export const SlateApplyAction: React.FC<SlateApplyActionProps> = ({ballotStyle, slate}) => {
    const {t, i18n} = useTranslation()
    const dispatch = useAppDispatch()
    const selection = useAppSelector(selectBallotSelectionByElectionId(ballotStyle.election_id))
    const [pending, setPending] = useState<ISlateChoices | undefined>()
    const [wasChosen, setWasChosen] = useState(false)
    const unavailableId = useId()

    const slateName = getSlateName(
        slate,
        i18n.language,
        ballotStyle.ballot_eml.election_presentation?.language_conf?.default_language_code ??
            ballotStyle.ballot_eml.election_event_presentation?.language_conf?.default_language_code
    )

    const {choices, problem} = useMemo((): {choices?: ISlateChoices; problem?: unknown} => {
        if (!selection) {
            return {}
        }
        try {
            return {choices: computeSlateChoices(slate, selection)}
        } catch (error) {
            console.log(`Error computing slate choices: ${error}`)
            return {problem: error}
        }
    }, [slate, selection])

    const unavailableReason = useMemo(() => {
        if (!problem) {
            return null
        }
        if (problem instanceof SlateOverMaximumError) {
            const {contest, candidates} = problem.slateContest
            return t("slates.apply.overMaximum", {
                slate: slateName,
                contest: translate(contest, "name", i18n.language) ?? contest.id,
                candidates: candidates.length,
                max: contest.max_votes,
            })
        }
        return t("slates.apply.unavailable", {slate: slateName})
    }, [problem, slateName, t, i18n.language])

    const apply = () => {
        dispatch(applySlateSelection({ballotStyle, slate}))
        setWasChosen(true)
    }

    const handleChoose = () => {
        if (!choices) {
            return
        }
        if (slateRemovesChoices(choices)) {
            setPending(choices)
        } else {
            apply()
        }
    }

    const handleClose = (confirmed: boolean) => {
        setPending(undefined)
        if (confirmed) {
            apply()
        }
    }

    const candidateNames = (candidates: ICandidate[], ids: string[]): string =>
        ids
            .map((id) => {
                const candidate = candidates.find((entry) => entry.id === id)
                return candidate ? (translate(candidate, "name", i18n.language) ?? id) : id
            })
            .join(", ")

    const isSelection = wasChosen && choices?.changes.length === 0
    const candidatesCount = slate.contests.reduce(
        (total, entry) => total + entry.candidates.length,
        0
    )

    return (
        <Box className="slate-apply">
            <Button
                className="slate-apply-button"
                variant="secondary"
                onClick={handleChoose}
                disabled={!choices}
                aria-label={t("slates.apply.buttonLabel", {slate: slateName})}
                aria-describedby={unavailableReason ? unavailableId : undefined}
            >
                {t("slates.apply.button")}
            </Button>
            <Typography className="slate-apply-status" role="status" variant="body2">
                {isSelection
                    ? t("slates.apply.chosen", {
                          slate: slateName,
                          candidates: candidatesCount,
                          contests: slate.contests.length,
                      })
                    : null}
            </Typography>
            {unavailableReason && (
                <Typography className="slate-apply-unavailable" id={unavailableId} variant="body2">
                    {unavailableReason}
                </Typography>
            )}
            <Dialog
                className="slate-replace-dialog"
                open={!!pending}
                handleClose={handleClose}
                title={t("slates.apply.replaceDialog.title")}
                ok={t("slates.apply.replaceDialog.ok")}
                cancel={t("slates.apply.replaceDialog.cancel")}
                variant="action"
            >
                <Typography className="slate-replace-content">
                    {t("slates.apply.replaceDialog.content", {slate: slateName})}
                </Typography>
                {(pending?.changes ?? [])
                    .filter((change) => change.removed.length > 0)
                    .map((change) => {
                        const contest = slate.contests.find(
                            (entry) => entry.contest.id === change.contestId
                        )?.contest
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
                                {change.added.length > 0 && (
                                    <Typography className="slate-replace-added">
                                        {t("slates.apply.replaceDialog.added")}{" "}
                                        <strong className="slate-replace-candidates">
                                            {candidateNames(contest.candidates, change.added)}
                                        </strong>
                                    </Typography>
                                )}
                            </Box>
                        )
                    })}
            </Dialog>
        </Box>
    )
}
