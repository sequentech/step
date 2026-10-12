// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useState} from "react"
import {Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {clearRemovesChoices, computeClearChoices} from "../../services/ClearSelections"
import type {ISlateChoices} from "../../services/SlateChoices"
import {
    clearBallotSelectionChoices,
    selectBallotSelectionByElectionId,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {clearIsVoted} from "../../store/extra/extraSlice"
import {useAppDispatch, useAppSelector} from "../../store/hooks"
import {SlateChangesDialog} from "./SlateChangesDialog"

export interface ISlateClear {
    /** Whether clearing would remove anything. */
    canClear: boolean
    /** Asks the voter to confirm what clearing removes. */
    requestClear: () => void
    /** The confirmation and the announcement, to render once. */
    feedback: React.ReactNode
}

/**
 * Clearing every choice of a ballot with slates: the voter first sees what
 * goes, and is told once it is done.
 */
export const useSlateClear = (ballotStyle: IBallotStyle): ISlateClear => {
    const {t} = useTranslation()
    const dispatch = useAppDispatch()
    const selection = useAppSelector(selectBallotSelectionByElectionId(ballotStyle.election_id))
    const [pending, setPending] = useState<ISlateChoices | undefined>()
    const [wasCleared, setWasCleared] = useState(false)
    const contests = ballotStyle.ballot_eml.contests

    const choices = useMemo(
        () => (selection ? computeClearChoices(contests, selection) : undefined),
        [contests, selection]
    )
    const canClear = !!choices && clearRemovesChoices(choices)

    useEffect(() => {
        if (canClear) {
            setWasCleared(false)
        }
    }, [canClear])

    const handleClose = (confirmed: boolean) => {
        setPending(undefined)
        if (confirmed) {
            dispatch(clearBallotSelectionChoices({ballotStyle}))
            dispatch(clearIsVoted())
            setWasCleared(true)
        }
    }

    const feedback = (
        <>
            <Typography
                className="slate-clear-status"
                role="status"
                variant="body2"
                sx={{overflowWrap: "anywhere"}}
            >
                {wasCleared && !canClear ? t("slates.clear.cleared") : null}
            </Typography>
            <SlateChangesDialog
                className="slate-clear-dialog"
                pending={pending}
                contests={contests}
                handleClose={handleClose}
                title={t("slates.clear.dialog.title")}
                content={t("slates.clear.dialog.content")}
                ok={t("slates.clear.dialog.ok")}
                cancel={t("slates.clear.dialog.cancel")}
            />
        </>
    )

    return {
        canClear,
        requestClear: () => {
            if (canClear) {
                setPending(choices)
            }
        },
        feedback,
    }
}
