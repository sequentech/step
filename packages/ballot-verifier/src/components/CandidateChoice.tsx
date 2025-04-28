// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import Image from "mui-image"

import {ICandidate, IBallotStyle, stringToHtml} from "@sequentech/ui-core"
import {Candidate} from "@sequentech/ui-essentials"

import {checkIsWriteIn, getImageUrl} from "../services/ElectionConfigService"

interface CandidateChoiceProps {
    ballotStyle: IBallotStyle | undefined
    answer: ICandidate
    points: number | null
    ordered: boolean
    writeInValue: string | undefined
}

export const CandidateChoice: React.FC<CandidateChoiceProps> = ({
    ballotStyle,
    answer,
    writeInValue,
}) => {
    const imageUrl = answer && getImageUrl(answer)
    const isWriteIn = checkIsWriteIn(answer)

    return (
        <Candidate
            title={answer?.name}
            description={stringToHtml(answer?.description || "")}
            isWriteIn={isWriteIn}
            writeInValue={writeInValue}
            shouldDisable={false}
        >
            {imageUrl ? <Image src={imageUrl} duration={100} /> : null}
        </Candidate>
    )
}
