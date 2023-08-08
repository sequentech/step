// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React, {useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    BreadCrumbSteps,
    Dialog,
    IconButton,
    PageLimit,
    SelectElection,
    isString,
    isUndefined,
    stringToHtml,
    theme,
} from "@sequentech/ui-essentials"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectElectionById, setElection} from "../store/elections/electionsSlice"
import {ELECTIONS_LIST} from "../fixtures/election"
import {useNavigate} from "react-router-dom"
import {useQuery} from "@apollo/client"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {GetBallotStylesQuery} from "../gql/graphql"
import { IElectionDTO } from "sequent-core"
import { resetBallotSelection } from "../store/ballotSelections/ballotSelectionsSlice"

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 24px;
    font-weight: 500;
    line-height: 27px;
    margin-top: 20px;
    margin-bottom: 16px;
`

const ElectionContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 30px;
`

interface ElectionWrapperProps {
    electionId: number
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({electionId}) => {
    const {loading, error, data} = useQuery<GetBallotStylesQuery>(GET_BALLOT_STYLES)
    const election = useAppSelector(selectElectionById(electionId))
    const dispatch = useAppDispatch()
    const navigate = useNavigate()

    useEffect(() => {
        if (!loading && !error && data) {
            data.sequent_backend_ballot_style
            .filter(ballotStyle => !isUndefined(ballotStyle.ballot_eml))
            .map(ballotStyle => {
                const ballotEml = ballotStyle.ballot_eml
                if (!isString(ballotEml)) {
                    return
                }
                try {
                    const electionData: IElectionDTO = JSON.parse(atob(ballotEml))
                    dispatch(setElection(electionData))
                    dispatch(resetBallotSelection({
                        election: electionData
                    }))
                } catch (error) {
                    console.log(`Error loading EML: ${error}`)
                    console.log(ballotEml)
                }
            })
            
        }
        //dispatch(fetchElectionByIdAsync(electionId))
    }, [loading, error, data])

    const onClickToVote = () => {
        navigate(`/election/${electionId}/start`)
    }

    const formatDate = (dateStr: string): string => {
        let date = new Date(dateStr)
        let dateFormat = new Intl.DateTimeFormat("en", {
            hour12: false,
            day: "numeric",
            month: "short",
            hour: "numeric",
            minute: "2-digit",
        })
        return dateFormat.format(date)
    }

    if (!election) {
        return null
    }

    return (
        <SelectElection
            isActive={true}
            isOpen={election.state === "started"}
            title={election.configuration.title}
            electionHomeUrl={"https://sequentech.io"}
            hasVoted={electionId === ELECTIONS_LIST[0].id}
            openDate={election.startDate && formatDate(election.startDate)}
            closeDate={election.endDate && formatDate(election.endDate)}
            onClickToVote={onClickToVote}
            onClickElectionResults={() => undefined}
        />
    )
}

export const ElectionSelectionScreen: React.FC = () => {
    const {t} = useTranslation()
    const [openChooserHelp, setOpenChooserHelp] = useState(false)

    const electionIds = ELECTIONS_LIST.map((election) => election.id)

    return (
        <PageLimit maxWidth="lg">
            <Box marginTop="48px">
                <BreadCrumbSteps
                    labels={[
                        "breadcrumbSteps.electionList",
                        "breadcrumbSteps.ballot",
                        "breadcrumbSteps.review",
                        "breadcrumbSteps.confirmation",
                    ]}
                    selected={0}
                />
            </Box>
            <StyledTitle variant="h1">
                <Box>{t("electionSelectionScreen.title")}</Box>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setOpenChooserHelp(true)}
                />
                <Dialog
                    handleClose={() => setOpenChooserHelp(false)}
                    open={openChooserHelp}
                    title={t("electionSelectionScreen.chooserHelpDialog.title")}
                    ok={t("electionSelectionScreen.chooserHelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("electionSelectionScreen.chooserHelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            <Typography variant="body1" sx={{color: theme.palette.customGrey.contrastText}}>
                {stringToHtml(t("electionSelectionScreen.description"))}
            </Typography>
            <ElectionContainer>
                {electionIds.map((electionId) => (
                    <ElectionWrapper electionId={electionId} key={electionId} />
                ))}
            </ElectionContainer>
        </PageLimit>
    )
}
