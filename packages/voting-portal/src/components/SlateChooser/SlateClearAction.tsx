// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useState} from "react"
import {Box, Button, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {clearRemovesChoices, computeClearChoices} from "../../services/ClearSelections"
import type {ISlateChoices} from "../../services/SlateChoices"
import {
    clearBallotSelectionChoices,
    selectBallotSelectionByElectionId,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../../store/hooks"
import {SlateChangesDialog} from "./SlateChangesDialog"

export interface SlateClearActionProps {
    ballotStyle: IBallotStyle
}

/** Removes every choice on the ballot, after showing what goes. */
export const SlateClearAction: React.FC<SlateClearActionProps> = ({ballotStyle}) => {
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
            setWasCleared(true)
        }
    }

    return (
        <Box className="slate-clear" sx={{marginBottom: "24px", overflowWrap: "anywhere"}}>
            <Button
                className="slate-clear-button"
                variant="secondary"
                onClick={() => setPending(choices)}
                disabled={!canClear}
            >
                {t("slates.clear.button")}
            </Button>
            <Typography className="slate-clear-status" role="status" variant="body2">
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
        </Box>
    )
}
