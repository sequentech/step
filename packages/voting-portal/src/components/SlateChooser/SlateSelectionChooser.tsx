// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {PropsWithChildren, useLayoutEffect, useMemo, useRef, useState} from "react"
import {Box, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import {faCheck} from "@fortawesome/free-solid-svg-icons"
import {EMobileCandidateLists, isAcclaimedContest, translate} from "@sequentech/ui-core"
import {Icon} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"

import {sortCandidatesInBallotOrder, sortContestsInBallotOrder} from "../../services/ContestOrder"
import {getSlateName, IBallotSlates, IResolvedSlate} from "../../services/Slates"
import {
    ESlateSelectionStatus,
    getSlateMembers,
    getSlateSelectionSummary,
    ISlateSelectionSummary,
} from "../../services/SlateSelection"
import {selectBallotSelectionByElectionId} from "../../store/ballotSelections/ballotSelectionsSlice"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useAppSelector} from "../../store/hooks"
import {SlateApplyAction} from "./SlateApplyAction"
import {SlateCandidateList} from "./SlateCandidateList"
import {ESlateCardState, SlateChooser} from "./SlateChooser"
import {SlateMember} from "./SlateMember"
import {SlateSelectionStatus} from "./SlateSelectionStatus"

const AppliedNotice = styled("div")(({theme}) => ({
    "&:not(:empty)": {
        display: "flex",
        alignItems: "flex-start",
        gap: "10px",
        margin: "0 0 20px",
        padding: "13px 15px",
        border: "1px solid #bee7d4",
        borderRadius: "7px",
        background: "#ecfdf5",
        color: "#065f46",
        fontSize: "14px",
        lineHeight: 1.45,
        overflowWrap: "anywhere",
    },
    "& .slate-applied-notice-icon": {
        marginTop: "3px",
    },
    [theme.breakpoints.down("sm")]: {
        "&:not(:empty)": {
            fontSize: "13px",
            padding: "11px 12px",
            gap: "8px",
            marginBottom: "16px",
        },
    },
}))

const AllSelections = styled("div")(({theme}) => ({
    "display": "grid",
    "gridTemplateColumns": "repeat(var(--office-columns), minmax(0, 1fr))",
    "gap": "15px",
    "marginTop": "24px",
    "padding": "19px",
    "borderRadius": "8px",
    "background": theme.palette.lightBackground,
    "overflowWrap": "anywhere",
    "& .slate-all-selections-office": {
        display: "block",
        marginBottom: "7px",
        fontSize: "12px",
        color: theme.palette.customGrey.dark,
    },
    "& .slate-all-selections-name": {
        display: "block",
        fontSize: "13px",
        lineHeight: 1.55,
    },
    [theme.breakpoints.down("sm")]: {
        "gridTemplateColumns": "1fr 1fr",
        "padding": "14px",
        "marginTop": "19px",
        "& .slate-all-selections-office": {fontSize: "11px"},
        "& .slate-all-selections-name": {fontSize: "12px"},
    },
}))

const MAX_OFFICE_COLUMNS = 5

const CARD_STATES: Record<ESlateSelectionStatus, ESlateCardState> = {
    [ESlateSelectionStatus.NONE]: ESlateCardState.NONE,
    [ESlateSelectionStatus.PARTLY]: ESlateCardState.SOME,
    [ESlateSelectionStatus.MIXED]: ESlateCardState.SOME,
    [ESlateSelectionStatus.ALL]: ESlateCardState.EXACT,
}

export interface SlateSelectionChooserProps {
    ballotStyle: IBallotStyle
    slates: IBallotSlates
    defaultLanguage?: string
    onEditSelections: () => void
}

/**
 * The slates of the ballot with what the voter currently holds of each one:
 * the selected members are marked and each slate says how much of it is
 * selected. Everything is read from the candidate selections of the ballot.
 */
export const SlateSelectionChooser: React.FC<SlateSelectionChooserProps> = ({
    ballotStyle,
    slates,
    defaultLanguage,
    onEditSelections,
}) => {
    const {t, i18n} = useTranslation()
    const selection = useAppSelector(selectBallotSelectionByElectionId(ballotStyle.election_id))
    const [appliedSlate, setAppliedSlate] = useState<IResolvedSlate | undefined>()

    const summaries = useMemo(
        () =>
            new Map<string, ISlateSelectionSummary>(
                slates.slates.map((slate) => [
                    slate.id,
                    getSlateSelectionSummary(getSlateMembers(slate), selection),
                ])
            ),
        [slates, selection]
    )

    const contests = useMemo(
        () =>
            sortContestsInBallotOrder(ballotStyle.ballot_eml.contests).filter(
                (contest) => !isAcclaimedContest(contest)
            ),
        [ballotStyle]
    )
    const selectedNames = (contestId: string): string[] => {
        const contest = contests.find((entry) => entry.id === contestId)
        const entry = selection?.find((item) => item.contest_id === contestId)
        if (!contest) {
            return []
        }
        return sortCandidatesInBallotOrder(contest, contest.candidates)
            .filter((candidate) =>
                entry?.choices.some((choice) => choice.id === candidate.id && choice.selected > -1)
            )
            .map((candidate) => translate(candidate, "name", i18n.language) ?? candidate.id)
    }
    const hasSelection = contests.some((contest) => selectedNames(contest.id).length > 0)

    const isAppliedSlateHeld =
        !!appliedSlate && summaries.get(appliedSlate.id)?.status === ESlateSelectionStatus.ALL
    const appliedNotice = (() => {
        if (!appliedSlate || !isAppliedSlateHeld) {
            return null
        }
        const name = getSlateName(appliedSlate, i18n.language, defaultLanguage)
        const blankOffices = contests.filter(
            (contest) => !appliedSlate.contests.some((entry) => entry.contest.id === contest.id)
        )
        return blankOffices.length
            ? t("slates.apply.appliedPartial", {
                  slate: name,
                  count: blankOffices.length,
                  offices: blankOffices
                      .map((contest) => translate(contest, "name", i18n.language) ?? contest.id)
                      .join(", "),
              })
            : t("slates.apply.applied", {slate: name})
    })()

    return (
        <>
            <AppliedNotice className="slate-applied-notice" role="status">
                {appliedNotice ? (
                    <>
                        <Icon
                            className="slate-applied-notice-icon"
                            icon={faCheck}
                            aria-hidden="true"
                        />
                        <span className="slate-applied-notice-text">{appliedNotice}</span>
                    </>
                ) : null}
            </AppliedNotice>
            <SlateChooser
                slates={slates}
                defaultLanguage={defaultLanguage}
                getCardState={(slate) =>
                    CARD_STATES[summaries.get(slate.id)?.status ?? ESlateSelectionStatus.NONE]
                }
                renderSummary={(slate) => {
                    const summary = summaries.get(slate.id)
                    return summary ? <SlateSelectionStatus summary={summary} /> : null
                }}
                renderMemberLists={(slate, lists) => (
                    <SlateCandidateList
                        electionId={ballotStyle.election_id}
                        slateId={slate.id}
                        defaultExpanded={
                            slates.mobileCandidateLists === EMobileCandidateLists.EXPANDED
                        }
                    >
                        {lists}
                    </SlateCandidateList>
                )}
                renderMember={(slate, _slateContest, candidate) => (
                    <SlateMember
                        candidateId={candidate.id}
                        selected={
                            summaries.get(slate.id)?.selectedMemberIds.includes(candidate.id) ??
                            false
                        }
                    >
                        {translate(candidate, "name", i18n.language)}
                    </SlateMember>
                )}
                renderActions={(slate) => (
                    <SlateCardActions
                        isFullySelected={
                            summaries.get(slate.id)?.status === ESlateSelectionStatus.ALL
                        }
                        onEditSelections={onEditSelections}
                    >
                        <SlateApplyAction
                            ballotStyle={ballotStyle}
                            slate={slate}
                            onApplied={setAppliedSlate}
                        />
                    </SlateCardActions>
                )}
            />
            {hasSelection ? (
                <AllSelections
                    className="slate-all-selections"
                    role="group"
                    aria-label={t("slates.allSelections.label")}
                    style={
                        {
                            "--office-columns": Math.min(MAX_OFFICE_COLUMNS, contests.length),
                        } as React.CSSProperties
                    }
                >
                    {contests.map((contest) => {
                        const names = selectedNames(contest.id)
                        return (
                            <div
                                key={contest.id}
                                className="slate-all-selections-contest"
                                data-contest-id={contest.id}
                            >
                                <span className="slate-all-selections-office">
                                    {translate(contest, "name", i18n.language)}
                                </span>
                                {(names.length ? names : [t("slates.allSelections.none")]).map(
                                    (name, index) => (
                                        <span key={index} className="slate-all-selections-name">
                                            {name}
                                        </span>
                                    )
                                )}
                            </div>
                        )
                    })}
                </AllSelections>
            ) : null}
        </>
    )
}

interface SlateCardActionsProps extends PropsWithChildren {
    isFullySelected: boolean
    onEditSelections: () => void
}

/**
 * Selecting a slate that is already fully selected would change nothing, so
 * the card offers to edit the selections instead. The apply action stays
 * mounted with its button hidden.
 */
const SlateCardActions: React.FC<SlateCardActionsProps> = ({
    isFullySelected,
    onEditSelections,
    children,
}) => {
    const {t} = useTranslation()
    const containerRef = useRef<HTMLDivElement>(null)
    const editRef = useRef<HTMLButtonElement>(null)

    useLayoutEffect(() => {
        if (isFullySelected && containerRef.current?.contains(document.activeElement)) {
            editRef.current?.focus()
        }
    }, [isFullySelected])

    return (
        <Box
            className="slate-card-actions"
            ref={containerRef}
            sx={isFullySelected ? {"& .slate-apply-button": {display: "none"}} : undefined}
        >
            {isFullySelected ? (
                <Button
                    className="slate-edit-selections-button"
                    variant="secondary"
                    ref={editRef}
                    onClick={onEditSelections}
                    startIcon={
                        <Icon
                            className="slate-edit-selections-icon"
                            icon={faCheck}
                            aria-hidden="true"
                        />
                    }
                >
                    {t("slates.selection.edit")}
                </Button>
            ) : null}
            {children}
        </Box>
    )
}
