// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    BreadCrumbSteps,
    PageLimit,
    theme,
    Icon,
    InfoDataBox,
    IconButton,
    Dialog,
    BallotInput,
} from "@sequentech/ui-essentials"
import {stringToHtml} from "@sequentech/ui-core"
import {Box, TextField, Typography, Button, Stack} from "@mui/material"
import {styled} from "@mui/material/styles"
import {Link, useLocation, useNavigate, useParams} from "react-router-dom"
import {GET_CAST_VOTE} from "../queries/GetCastVote"
import {useQuery} from "@apollo/client"
import {GetBallotStylesQuery, GetCastVoteQuery, GetElectionEventQuery} from "../gql/graphql"
import {faAngleLeft, faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {updateBallotStyleAndSelection} from "../services/BallotStyles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectFirstBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import useLanguage from "../hooks/useLanguage"
import {SettingsContext} from "../providers/SettingsContextProvider"
import useUpdateTranslation from "../hooks/useUpdateTranslation"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {IElectionEvent} from "../store/electionEvents/electionEventsSlice"

const StyledLink = styled(Link)`
    text-decoration: none;
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

const StyledError = styled(Typography)`
    position: absolute;
    margin-top: -12px;
    color: ${({theme}) => theme.palette.red.main};
`

const MessageSuccess = styled(Box)`
    display: flex;
    padding: 10px 22px;
    color: ${({theme}) => theme.palette.green.dark};
    background-color: ${({theme}) => theme.palette.green.light};
    gap: 8px;
    border-radius: 4px;
    border: 1px solid ${({theme}) => theme.palette.green.dark};
    align-items: center;
    margin-right: auto;
    margin-left: auto;
    overflow-wrap: anywhere;
`

const MessageFailed = styled(Box)`
    display: flex;
    padding: 10px 22px;
    color: ${({theme}) => theme.palette.red.dark};
    background-color: ${({theme}) => theme.palette.red.light};
    gap: 8px;
    border-radius: 4px;
    border: 1px solid ${({theme}) => theme.palette.red.dark};
    align-items: center;
    margin-right: auto;
    margin-left: auto;
    overflow-wrap: anywhere;
`

function isHex(str: string) {
    if (str.trim() === "") {
        return true
    }

    const regex = /^[0-9a-fA-F]+$/
    return regex.test(str)
}

const StyledApp = styled(Stack)<{css: string}>`
    min-height: 100vh;
    min-width: 100vw;
    ${({css}) => css}
`

const BallotLocator: React.FC = () => {
    const {t} = useTranslation()
    const navigate = useNavigate()
    const {tenantId, eventId, electionId, ballotId} = useParams()
    const {data: dataBallotStyles} = useQuery<GetBallotStylesQuery>(GET_BALLOT_STYLES)
    const dispatch = useAppDispatch()

    const [openTitleHelp, setOpenTitleHelp] = useState<boolean>(false)
    const location = useLocation()
    const [inputBallotId, setInputBallotId] = useState<string>("")
    const {globalSettings} = useContext(SettingsContext)
    const [step, setStep] = useState<number>(0)

    const hasBallotId = !!ballotId

    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    useLanguage({ballotStyle})

    const {data, loading} = useQuery<GetCastVoteQuery>(GET_CAST_VOTE, {
        variables: {
            tenantId,
            electionEventId: eventId,
            electionId,
            ballotId,
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })

    const {data: dataElectionEvent} = useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {
        variables: {
            electionEventId: eventId,
            tenantId,
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })

    useUpdateTranslation({
        electionEvent: dataElectionEvent?.sequent_backend_election_event[0] as IElectionEvent,
    }) // Overwrite translations

    useEffect(() => {
        if (dataBallotStyles && dataBallotStyles.sequent_backend_ballot_style.length > 0) {
            updateBallotStyleAndSelection(dataBallotStyles, dispatch)
        }
    }, [dataBallotStyles, dispatch])

    const validatedBallotId = isHex(inputBallotId ?? "")

    const ballotContent =
        data?.["sequent_backend_cast_vote"]?.find((item) => item.ballot_id === ballotId)?.content ??
        null

    const locate = (withBallotId = false) => {
        let id = withBallotId ? inputBallotId : ""

        setInputBallotId("")

        if (withBallotId) {
            setStep(1)
        } else {
            setStep(0)
        }

        navigate(
            `/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${id}${location.search}`
        )
    }

    const captureEnter: React.KeyboardEventHandler<HTMLDivElement> = (event) => {
        if ("Enter" === event.key) {
            locate(true)
        }
    }

    return (
        <StyledApp
            css={dataElectionEvent?.sequent_backend_election_event[0]?.presentation?.css ?? ""}
        >
            <PageLimit maxWidth="lg" className="ballot-locator-screen screen">
                <Box marginTop="48px">
                    <BreadCrumbSteps
                        labels={["ballotLocator.steps.lookup", "ballotLocator.steps.result"]}
                        selected={step}
                    />
                </Box>

                {!hasBallotId ? (
                    <>
                        <BallotInput
                            title="ballotLocator.title"
                            subTitle="ballotLocator.description"
                            label="Ballot ID"
                            error="ballotLocator.wrongFormatBallotId"
                            placeholder={t("ballotLocator.description")}
                            value={inputBallotId}
                            doChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                                setInputBallotId(event.target.value)
                            }}
                            captureEnterAction={captureEnter}
                            labelProps={{
                                shrink: true,
                            }}
                            helpText="ballotLocator.titleHelpDialog.content"
                            dialogTitle="ballotLocator.titleHelpDialog.title"
                            dialogOk="ballotLocator.titleHelpDialog.ok"
                            backButtonText="votingScreen.backButton"
                            ballotStyle={ballotStyle}
                        />
                        <Button
                            sx={{marginTop: "10px"}}
                            disabled={!validatedBallotId || inputBallotId.trim() === ""}
                            className="normal"
                            onClick={() => locate(true)}
                        >
                            <span>{t("ballotLocator.locate")}</span>
                        </Button>
                    </>
                ) : (
                    <Box
                        sx={{
                            display: "flex",
                            flexDirection: {xs: "column", md: "row"},
                            justifyContent: "space-between",
                            alignItems: "flex-start",
                        }}
                    >
                        <Box
                            sx={{
                                order: {xs: 2, md: 1},
                            }}
                        >
                            {!loading && (
                                <Box>
                                    {hasBallotId && !!ballotContent ? (
                                        <MessageSuccess>
                                            {t("ballotLocator.found", {ballotId})}
                                        </MessageSuccess>
                                    ) : (
                                        <MessageFailed>
                                            {t("ballotLocator.notFound", {ballotId})}
                                        </MessageFailed>
                                    )}
                                </Box>
                            )}
                            {ballotContent && (
                                <>
                                    <Typography>{t("ballotLocator.contentDesc")}</Typography>
                                    <InfoDataBox>{ballotContent}</InfoDataBox>
                                </>
                            )}

                            <Button
                                sx={{marginTop: "10px"}}
                                className="normal"
                                onClick={() => locate()}
                            >
                                <span>{t("ballotLocator.locateAgain")}</span>
                            </Button>
                        </Box>
                    </Box>
                )}
            </PageLimit>
        </StyledApp>
    )
}

export default BallotLocator
