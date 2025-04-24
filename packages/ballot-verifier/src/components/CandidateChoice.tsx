// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useEffect, PropsWithChildren} from "react"
import Typography from "@mui/material/Typography"
import Paper, {PaperProps} from "@mui/material/Paper"
import Box from "@mui/material/Box"
import {useNavigate} from "react-router-dom"
import {Link as RouterLink} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import Skeleton from "@mui/material/Skeleton"
import {IBallotStyle, IDecodedVoteContest, checkIsBlank} from "@sequentech/ui-core"
import {IBallotService, IConfirmationBallot} from "../services/BallotService"
import {checkIsInvalidVote, checkIsWriteIn, getImageUrl} from "../services/ElectionConfigService"
import Button from "@mui/material/Button"
import {
    faCircleQuestion,
    faTimesCircle,
    faPrint,
    faAngleLeft,
} from "@fortawesome/free-solid-svg-icons"
import {
    Candidate,
    Icon,
    IconButton,
    BreadCrumbSteps,
    PageLimit,
    WarnBox,
    Dialog,
    theme,
    BlankAnswer,
} from "@sequentech/ui-essentials"
import {translate, ICandidate, IContest, EInvalidVotePolicy} from "@sequentech/ui-core"
import {keyBy} from "lodash"
import Image from "mui-image"

const StyledLink = styled(RouterLink)`
    margin: auto 0;
    text-decoration: none;
`

const HorizontalWrap = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 16px;
    margin-bottom: 12px;
`

const BallotIdPaper = styled(Paper)`
    padding: 10px 16px;
    display: flex;
    overflow: auto;
`

const OneLine = styled(Paper)`
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
`

const ActionsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    margin-bottom: 20px;
    margin-top: 10px;
    gap: 2px;
`

const StyledButton = styled(Button)`
    display flex;
    padding: 5px;

    span {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        padding: 5px;
    }
`

const CandidatesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 12px 0;
`

interface CandidateChoiceProps {
    ballotStyle: IBallotStyle | undefined
    answer?: ICandidate
    points: number | null
    ordered: boolean
    isWriteIn: boolean
    writeInValue: string | undefined
}

export const CandidateChoice: React.FC<CandidateChoiceProps> = ({
    ballotStyle,
    answer,
    isWriteIn,
    writeInValue,
}) => {
    const imageUrl = answer && getImageUrl(answer)

    return (
        <Candidate
            title={answer?.name || ""}
            description={answer?.description}
            isWriteIn={isWriteIn}
            writeInValue={writeInValue}
            shouldDisable={false}
        >
            {imageUrl ? <Image src={imageUrl} duration={100} /> : null}
        </Candidate>
    )
}
