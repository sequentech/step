// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {
    stringToHtml,
    isUndefined,
    normalizeWriteInText,
    translate,
    ICandidate,
    IContest,
} from "@sequentech/ui-core"
import Image from "mui-image"
import {useTranslation} from "react-i18next"
import {ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"
import {IBallotStyle as IElectionDTO} from "@sequentech/ui-core"
import {IDecodedVoteContest} from "sequent-core"

import {
    checkAllowWriteIns,
    checkIsWriteIn,
    getImageUrl,
    getLinkUrl,
} from "../../services/ElectionConfigService"
import {provideBallotService} from "../../services/BallotService"
import Candidate from "../Candidate/Candidate"

interface IBallotStyle {
    id: string
    election_id: string
    election_event_id: string
    tenant_id: string
    ballot_eml: IElectionDTO
    ballot_signature?: string | null
    created_at: string
    area_id?: string | null
    annotations?: string | null
    labels?: string | null
    last_updated_at: string
}

export interface ICandidatesDisplayProps {
    answer: ICandidate
    questionPlaintext?: IDecodedVoteContest
    contestId: string
    index: number
    ballotStyle: IBallotStyle
    hasCategory?: boolean
    isActive: boolean
    iconCheckboxPolicy?: ECandidatesIconCheckboxPolicy
    isReview: boolean
    isInvalidVote?: boolean
    isExplicitBlankVote?: boolean
    isInvalidWriteIns?: boolean
    isRadioSelection?: boolean
    contest: IContest
    selectedChoicesSum?: number
    setSelectedChoicesSum?: (num: number) => void
    disableSelect?: boolean
    writeInValue?: string | undefined
    onResetBallotSelection?: (action: any) => any
    onSetBallotSelectionBlankVote?: (action: any) => any
    onSetBallotSelectionInvalidVote?: (action: any) => any
    onSetBallotSelectionVoteChoice?: (action: any) => any
    url?: string
}

export const CandidatesDisplay: React.FC<ICandidatesDisplayProps> = ({
    answer,
    questionPlaintext,
    contestId,
    ballotStyle,
    hasCategory,
    isActive,
    iconCheckboxPolicy,
    isReview,
    isInvalidVote,
    isExplicitBlankVote,
    isInvalidWriteIns,
    isRadioSelection,
    contest,
    selectedChoicesSum,
    setSelectedChoicesSum,
    disableSelect,
    writeInValue,
    onResetBallotSelection,
    onSetBallotSelectionBlankVote,
    onSetBallotSelectionInvalidVote,
    onSetBallotSelectionVoteChoice,
    url,
}) => {
    const selectionState = questionPlaintext?.choices.find((c) => c.id === answer.id)

    const [explicitBlank, setExplicitBlank] = useState<boolean>(false)
    const question = ballotStyle.ballot_eml.contests.find((contest) => contest.id === contestId)
    const imageUrl = getImageUrl(answer)
    const infoUrl = getLinkUrl(answer)
    const {i18n} = useTranslation()
    const ballotService = provideBallotService()
    const shouldDisable = disableSelect && selectionState?.selected === -1
    const isWriteIn = checkIsWriteIn(answer)
    const allowWriteIns = question && checkAllowWriteIns(question)

    const isChecked = (): boolean => {
        if (isInvalidVote) {
            return !isUndefined(questionPlaintext) && questionPlaintext.is_explicit_invalid
        } else if (isExplicitBlankVote) {
            return (
                !isUndefined(questionPlaintext) &&
                !!ballotService.checkIsBlank(questionPlaintext) &&
                explicitBlank
            )
        } else {
            return !isUndefined(selectionState) && selectionState.selected > -1
        }
    }

    const setInvalidVote = (value: boolean) => {
        onSetBallotSelectionInvalidVote?.({
            ballotStyle,
            contestId,
            isExplicitInvalid: value,
        })
    }

    const setBlankVote = () => {
        setExplicitBlank(true)
        onSetBallotSelectionBlankVote?.({ballotStyle, contestId})
    }

    const setChecked = (value: boolean) => {
        if (!isActive || isReview) {
            return
        }
        if (isInvalidVote) {
            setInvalidVote(value)
            return
        }

        if (isExplicitBlankVote) {
            if (value) {
                setBlankVote()
            } else {
                setExplicitBlank(false)
            }
            return
        }

        let cleanedText =
            selectionState?.write_in_text && normalizeWriteInText(selectionState?.write_in_text)

        if (isRadioSelection) {
            onResetBallotSelection?.({
                ballotStyle,
                force: true,
                contestId: contest.id,
            })
        }

        onSetBallotSelectionVoteChoice?.({
            ballotStyle,
            contestId,
            voteChoice: {
                id: answer.id,
                selected: value ? 0 : -1,
                write_in_text: cleanedText,
            },
        })
    }

    const setWriteInText = (writeInText: string): void => {
        if (!isWriteIn || !allowWriteIns || !isActive || isReview) {
            return
        }
        let cleanedText = normalizeWriteInText(writeInText)

        onSetBallotSelectionVoteChoice?.({
            ballotStyle,
            contestId,
            voteChoice: {
                id: answer.id,
                selected: isUndefined(selectionState) ? -1 : selectionState.selected,
                write_in_text: cleanedText,
            },
        })
    }

    if (isReview && !isChecked()) {
        return null
    }

    if (isReview && !!isExplicitBlankVote) {
        return null
    }

    return (
        <Candidate
            title={translate(answer, "name", i18n.language)}
            description={stringToHtml(translate(answer, "description", i18n.language) || "")}
            isActive={isActive}
            checked={isChecked()}
            setChecked={setChecked}
            url={infoUrl}
            hasCategory={hasCategory}
            isWriteIn={allowWriteIns && isWriteIn}
            writeInValue={writeInValue || selectionState?.write_in_text}
            setWriteInText={setWriteInText}
            isInvalidVote={isInvalidVote}
            isInvalidWriteIn={!!selectionState?.write_in_text && isInvalidWriteIns}
            shouldDisable={shouldDisable}
            iconCheckboxPolicy={iconCheckboxPolicy}
        >
            {imageUrl ? <Image src={`${url ?? ""}${imageUrl}`} duration={100} /> : null}
        </Candidate>
    )
}
