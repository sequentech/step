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
    stringToHtml,
    theme,
} from "@sequentech/ui-essentials"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {IBallotStyle, selectBallotStyleByElectionId, setBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {ELECTIONS_LIST} from "../fixtures/election"
import {useNavigate} from "react-router-dom"
import {useQuery} from "@apollo/client"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {GetBallotStylesQuery, GetElectionsQuery} from "../gql/graphql"
import { IElectionDTO } from "sequent-core"
import { resetBallotSelection } from "../store/ballotSelections/ballotSelectionsSlice"
import { IElection, selectAllElectionIds, setElection } from "../store/elections/electionsSlice"
import { GET_ELECTIONS } from "../queries/GetElections"

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
    margin-bottom: 30px;
`

interface ElectionWrapperProps {
    electionId: string
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({electionId}) => {
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(electionId))
    const navigate = useNavigate()

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

    if (!ballotStyle) {
        return null
    }

    return (
        <SelectElection
            isActive={true}
            isOpen={ballotStyle.ballot_eml.state === "started"}
            title={ballotStyle.ballot_eml.configuration.title}
            electionHomeUrl={"https://sequentech.io"}
            hasVoted={ballotStyle.ballot_eml.id === ELECTIONS_LIST[0].id}
            openDate={ballotStyle.ballot_eml.startDate && formatDate(ballotStyle.ballot_eml.startDate)}
            closeDate={ballotStyle.ballot_eml.endDate && formatDate(ballotStyle.ballot_eml.endDate)}
            onClickToVote={onClickToVote}
            onClickElectionResults={() => undefined}
        />
    )
}

export const ElectionSelectionScreen: React.FC = () => {
    const {loading, error, data} = useQuery<GetBallotStylesQuery>(GET_BALLOT_STYLES)
    const {loading: loadingElections, error: errorElections, data: dataElections} = useQuery<GetElectionsQuery>(GET_ELECTIONS)
    const dispatch = useAppDispatch()
    const {t} = useTranslation()
    const [openChooserHelp, setOpenChooserHelp] = useState(false)

    const electionIds = useAppSelector(selectAllElectionIds)

    useEffect(() => {
        if (!loadingElections && !errorElections && dataElections) {
            for (let election of dataElections.sequent_backend_election) {
                const formattedElection: IElection = {
                    id: election.id,
                    annotations: election.annotations,
                    created_at: election.created_at,
                    dates: election.dates,
                    description: election.description,
                    election_event_id: election.election_event_id,
                    eml: election.eml,
                    is_consolidated_ballot_encoding: election.is_consolidated_ballot_encoding,
                    labels: election.labels,
                    last_updated_at: election.last_updated_at,
                    name: election.name,
                    num_allowed_revotes: election.num_allowed_revotes,
                    presentation: election.presentation,
                    spoil_ballot_option: election.spoil_ballot_option,
                    status: election.status,
                    tenant_id: election.tenant_id,
                }
                dispatch(setElection(formattedElection))

            }
        }
    }, [loadingElections, errorElections, dataElections, dispatch])

    useEffect(() => {
        if (!loading && !error && data) {
            for (let ballotStyle of data.sequent_backend_ballot_style) {
                const ballotEml = ballotStyle.ballot_eml
                if (!isString(ballotEml)) {
                    continue
                }
                try {
                    const electionData: IElectionDTO = JSON.parse(atob(ballotEml))
                    const formattedBallotStyle: IBallotStyle = {
                        id: ballotStyle.id,
                        election_id: ballotStyle.election_id,
                        election_event_id: ballotStyle.election_event_id,
                        status: ballotStyle.status || undefined,
                        tenant_id: ballotStyle.tenant_id,
                        ballot_eml: electionData,
                        ballot_signature: ballotStyle.ballot_signature,
                        created_at: ballotStyle.created_at,
                        area_id: ballotStyle.area_id,
                        annotations: ballotStyle.annotations,
                        labels: ballotStyle.labels,
                        last_updated_at: ballotStyle.last_updated_at,
                    }
                    dispatch(setBallotStyle(formattedBallotStyle))
                    dispatch(resetBallotSelection({
                        ballotStyle: formattedBallotStyle
                    }))
                } catch (error) {
                    console.log(`Error loading EML: ${error}`)
                    console.log(ballotEml)
                }
            }
            
        }
    }, [loading, error, data, dispatch])

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
