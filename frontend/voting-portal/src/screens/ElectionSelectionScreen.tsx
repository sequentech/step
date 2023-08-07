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
    stringToHtml,
    theme,
} from "@sequentech/ui-essentials"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {fetchElectionByIdAsync, selectElectionById} from "../store/elections/electionsSlice"
import {ELECTIONS_LIST} from "../fixtures/election"
import {useNavigate} from "react-router-dom"
import { useQuery, gql } from '@apollo/client'

const GET_LOCATIONS = gql`
  query GetLocations {
    locations {
      id
      name
      description
      photo
    }
  }
`

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

const DisplayLocations: React.FC = () => {

    const { loading, error, data } = useQuery(GET_LOCATIONS);
  
  
    if (loading) return <p>Loading...</p>;
  
    if (error) return <p>Error : {error.message}</p>;
  
  
    return data.locations.map((info: any) => {
        const { id, name, description, photo } = info as any
  
      return <div key={id}>
        <h3>{name}</h3>
        <img width="400" height="250" alt="location-reference" src={`${photo}`} />
        <br />
        <b>About this location:</b>
        <p>{description}</p>
        <br />
      </div>
  
});
  
  }

interface ElectionWrapperProps {
    electionId: number
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({electionId}) => {
    const election = useAppSelector(selectElectionById(electionId))
    const dispatch = useAppDispatch()
    const navigate = useNavigate()

    useEffect(() => {
        dispatch(fetchElectionByIdAsync(electionId))
    }, [])

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
            <DisplayLocations />
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
