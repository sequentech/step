// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {PropsWithChildren, useLayoutEffect, useMemo, useRef} from "react"
import {Box, Button} from "@mui/material"
import {EMobileCandidateLists, translate} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

import {IBallotSlates, IResolvedSlate} from "../../services/Slates"
import {
    ESlateSelectionStatus,
    getSlateMembers,
    getSlateSelectionSummary,
    ISlateSelectionSummary,
} from "../../services/SlateSelection"
import {selectBallotSelectionByElectionId} from "../../store/ballotSelections/ballotSelectionsSlice"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useAppSelector} from "../../store/hooks"
import {SlateCandidateList} from "./SlateCandidateList"
import {SlateChooser} from "./SlateChooser"
import {SlateMember} from "./SlateMember"
import {SlateSelectionStatus} from "./SlateSelectionStatus"

export interface SlateSelectionChooserProps {
    ballotStyle: IBallotStyle
    slates: IBallotSlates
    defaultLanguage?: string
    onEditSelections: () => void
    /** The action that selects a slate, shown while it is not fully selected. */
    renderApplyAction?: (slate: IResolvedSlate) => React.ReactNode
    renderCoverage?: (slate: IResolvedSlate) => React.ReactNode
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
    renderApplyAction,
    renderCoverage,
}) => {
    const {t, i18n} = useTranslation()
    const selection = useAppSelector(selectBallotSelectionByElectionId(ballotStyle.election_id))

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

    return (
        <SlateChooser
            slates={slates}
            defaultLanguage={defaultLanguage}
            renderCoverage={renderCoverage}
            renderSummary={(slate) => {
                const summary = summaries.get(slate.id)
                return summary ? <SlateSelectionStatus summary={summary} /> : null
            }}
            renderMemberLists={(slate, lists) => (
                <SlateCandidateList
                    electionId={ballotStyle.election_id}
                    slateId={slate.id}
                    defaultExpanded={slates.mobileCandidateLists === EMobileCandidateLists.EXPANDED}
                >
                    <Box
                        className="slate-member-lists"
                        display="flex"
                        flexDirection="column"
                        gap="12px"
                    >
                        {lists}
                    </Box>
                </SlateCandidateList>
            )}
            renderMember={(slate, _slateContest, candidate) => (
                <SlateMember
                    candidateId={candidate.id}
                    selected={
                        summaries.get(slate.id)?.selectedMemberIds.includes(candidate.id) ?? false
                    }
                >
                    {translate(candidate, "name", i18n.language)}
                </SlateMember>
            )}
            renderActions={(slate) => (
                <SlateCardActions
                    isFullySelected={summaries.get(slate.id)?.status === ESlateSelectionStatus.ALL}
                    onEditSelections={onEditSelections}
                >
                    {renderApplyAction?.(slate)}
                </SlateCardActions>
            )}
        />
    )
}

interface SlateCardActionsProps extends PropsWithChildren {
    isFullySelected: boolean
    onEditSelections: () => void
}

/**
 * Selecting a slate that is already fully selected would change nothing, so
 * the card offers to edit the selections instead. The apply action stays
 * mounted with its button hidden, which keeps its announcement of the
 * selection that has just been made.
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
                >
                    {t("slates.selection.edit")}
                </Button>
            ) : null}
            {children}
        </Box>
    )
}
