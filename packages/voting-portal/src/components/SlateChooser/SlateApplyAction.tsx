// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useId, useMemo, useState} from "react"
import {Box, Button, Typography} from "@mui/material"
import {isAcclaimedContest, translate} from "@sequentech/ui-core"
import {Trans, useTranslation} from "react-i18next"
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
import {SlateChangesDialog} from "./SlateChangesDialog"

export interface SlateApplyActionProps {
    ballotStyle: IBallotStyle
    slate: IResolvedSlate
    /** Told once the slate's candidates are the selection. */
    onApplied?: (slate: IResolvedSlate) => void
}

export const SlateApplyAction: React.FC<SlateApplyActionProps> = ({
    ballotStyle,
    slate,
    onApplied,
}) => {
    const {t, i18n} = useTranslation()
    const dispatch = useAppDispatch()
    const selection = useAppSelector(selectBallotSelectionByElectionId(ballotStyle.election_id))
    const [pending, setPending] = useState<ISlateChoices | undefined>()
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
            return {choices: computeSlateChoices(slate, ballotStyle.ballot_eml.contests, selection)}
        } catch (error) {
            console.log(`Error computing slate choices: ${error}`)
            return {problem: error}
        }
    }, [slate, ballotStyle, selection])

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
        onApplied?.(slate)
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

    const affected = (pending?.changes ?? []).filter((change) => change.removed.length > 0)
    const [onlyAffected] = affected
    const onlyAffectedContest =
        affected.length === 1
            ? ballotStyle.ballot_eml.contests.find(
                  (contest) => contest.id === onlyAffected.contestId
              )
            : undefined
    const leavesOfficesBlank = ballotStyle.ballot_eml.contests.some(
        (contest) =>
            !isAcclaimedContest(contest) &&
            !slate.contests.some((entry) => entry.contest.id === contest.id)
    )

    return (
        <Box className="slate-apply">
            <Button
                className="slate-apply-button"
                onClick={handleChoose}
                disabled={!choices}
                aria-label={t("slates.apply.buttonLabel", {slate: slateName})}
                aria-describedby={unavailableReason ? unavailableId : undefined}
            >
                {t("slates.apply.button")}
            </Button>
            {unavailableReason && (
                <Typography className="slate-apply-unavailable" id={unavailableId} variant="body2">
                    {unavailableReason}
                </Typography>
            )}
            <SlateChangesDialog
                className="slate-replace-dialog"
                pending={pending}
                current={selection}
                contests={ballotStyle.ballot_eml.contests}
                handleClose={handleClose}
                title={t("slates.apply.replaceDialog.title")}
                content={
                    <Trans
                        i18nKey="slates.apply.replaceDialog.content"
                        values={{slate: slateName}}
                        count={affected.length}
                        components={{strong: <strong className="slate-replace-slate" />}}
                    />
                }
                afterLabel={slateName}
                note={
                    leavesOfficesBlank
                        ? t("slates.apply.replaceDialog.blankNote", {slate: slateName})
                        : undefined
                }
                ok={
                    onlyAffectedContest
                        ? t("slates.apply.replaceDialog.okOffice", {
                              office: (
                                  translate(onlyAffectedContest, "name", i18n.language) ?? ""
                              ).toLocaleLowerCase(),
                          })
                        : t("slates.apply.replaceDialog.ok")
                }
                cancel={t("slates.apply.replaceDialog.cancel")}
            />
        </Box>
    )
}
