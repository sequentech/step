// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {faCircleInfo} from "@fortawesome/free-solid-svg-icons"
import {Dialog, Icon} from "@sequentech/ui-essentials"
import {BallotSelection, translate} from "@sequentech/ui-core"
import type {IContest} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"
import {sortCandidatesInBallotOrder, sortContestsInBallotOrder} from "../../services/ContestOrder"
import type {ISlateChoices} from "../../services/SlateChoices"

// On a phone the changes take the whole width of the dialog and the two
// answers a row each, the one that applies the change first.
const ChangesDialog = styled(Dialog)`
    @media (max-width: 749.95px) {
        .MuiDialogContent-root {
            padding: 16px 20px 0 20px !important;
        }
        .MuiDialogActions-root {
            flex-direction: column-reverse;
            gap: 9px;
            padding: 15px 20px 20px;
        }
        .MuiDialogActions-root > .MuiButtonBase-root {
            width: 100%;
            margin: 0;
        }
    }
`

const ChangeRow = styled("div")(({theme}) => ({
    "padding": "15px 0",
    "borderBottom": "1px solid #e7eaee",
    "overflowWrap": "anywhere",
    "&:last-of-type": {
        borderBottom: "none",
    },
    "& .slate-change-columns": {
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr) 24px minmax(0, 1fr)",
        gap: "8px",
        fontSize: "14px",
        lineHeight: 1.8,
    },
    "& .slate-change-label": {
        display: "block",
        marginBottom: "5px",
        fontSize: "12px",
        lineHeight: 1.4,
        color: theme.palette.customGrey.dark,
    },
    "& .slate-change-arrow": {
        paddingTop: "26px",
        color: theme.palette.customGrey.dark,
    },
    "& .slate-change-after": {
        fontWeight: 500,
        color: theme.palette.brandColor,
    },
    [theme.breakpoints.down("sm")]: {
        "& .slate-change-columns": {
            gridTemplateColumns: "minmax(0, 1fr) 15px minmax(0, 1fr)",
            gap: "7px",
            fontSize: "13px",
        },
        "& .slate-change-label": {
            fontSize: "11px",
        },
    },
}))

const Notice = styled("div")(({theme}) => ({
    display: "flex",
    alignItems: "flex-start",
    gap: "10px",
    marginTop: "18px",
    padding: "13px 15px",
    border: "1px solid #e7eaee",
    borderRadius: "7px",
    background: theme.palette.lightBackground,
    color: theme.palette.customGrey.dark,
    fontSize: "14px",
    lineHeight: 1.45,
}))

export interface SlateChangesDialogProps {
    className: string
    /** The ballot to confirm, or undefined while the dialog is closed. */
    pending?: ISlateChoices
    /** The ballot as it is now. */
    current?: BallotSelection
    /** The contests of the ballot, to name the contests and candidates. */
    contests: Array<IContest>
    title: string
    content: React.ReactNode
    /** The heading of the column that shows each contest after the change. */
    afterLabel: string
    /** A remark under the changes, such as offices that will be left blank. */
    note?: string
    ok: string
    cancel: string
    handleClose: (confirmed: boolean) => void
}

/**
 * The before and after of every contest that loses a choice. A contest left
 * without any choice says so instead of naming a candidate.
 */
export const SlateChangesDialog: React.FC<SlateChangesDialogProps> = ({
    className,
    pending,
    current,
    contests,
    title,
    content,
    afterLabel,
    note,
    ok,
    cancel,
    handleClose,
}) => {
    const {t, i18n} = useTranslation()

    const namesOf = (contest: IContest, selection: BallotSelection | undefined): string[] => {
        const entry = selection?.find((item) => item.contest_id === contest.id)
        return sortCandidatesInBallotOrder(contest, contest.candidates)
            .filter((candidate) =>
                entry?.choices.some((choice) => choice.id === candidate.id && choice.selected > -1)
            )
            .map((candidate) => translate(candidate, "name", i18n.language) ?? candidate.id)
    }

    const lines = (names: string[]) =>
        names.map((name, index) => (
            <span key={index} className="slate-change-candidate" style={{display: "block"}}>
                {name}
            </span>
        ))

    return (
        <ChangesDialog
            className={className}
            open={!!pending}
            handleClose={handleClose}
            title={title}
            ok={ok}
            cancel={cancel}
            variant="info"
            maxWidth="sm"
        >
            <Box className="slate-changes" sx={{overflowWrap: "anywhere"}}>
                <Typography className="slate-replace-content">{content}</Typography>
                {sortContestsInBallotOrder(contests).map((contest) => {
                    const change = pending?.changes.find((entry) => entry.contestId === contest.id)
                    if (!change || change.removed.length === 0) {
                        return null
                    }
                    const after = namesOf(contest, pending?.selection)
                    return (
                        <ChangeRow key={contest.id} className="slate-replace-contest">
                            <Typography
                                className="slate-replace-contest-name"
                                component="h3"
                                sx={{fontSize: "15px", fontWeight: 700, margin: "0 0 11px"}}
                            >
                                {translate(contest, "name", i18n.language) ?? contest.id}
                            </Typography>
                            <div className="slate-change-columns">
                                <div className="slate-replace-removed">
                                    <span className="slate-change-label">
                                        {t("slates.apply.replaceDialog.current")}
                                    </span>
                                    {lines(namesOf(contest, current))}
                                </div>
                                <div className="slate-change-arrow" aria-hidden="true">
                                    →
                                </div>
                                <div
                                    className={`slate-replace-added slate-change-after${
                                        after.length ? "" : " slate-replace-none"
                                    }`}
                                >
                                    <span className="slate-change-label">{afterLabel}</span>
                                    {after.length ? lines(after) : lines([t("slates.noCandidate")])}
                                </div>
                            </div>
                        </ChangeRow>
                    )
                })}
                {note ? (
                    <Notice className="slate-replace-note">
                        <Icon
                            className="slate-replace-note-icon"
                            icon={faCircleInfo}
                            aria-hidden="true"
                            style={{marginTop: "3px"}}
                        />
                        <span className="slate-replace-note-text">{note}</span>
                    </Notice>
                ) : null}
            </Box>
        </ChangesDialog>
    )
}
