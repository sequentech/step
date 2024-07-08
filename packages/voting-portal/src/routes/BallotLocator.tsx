// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    BreadCrumbSteps,
    PageLimit,
    theme,
    Icon,
    InfoDataBox,
    IconButton,
    Dialog,
    stringToHtml,
    translateText,
} from "@sequentech/ui-essentials"
import {Box, TextField, Typography, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import {Link, useLocation, useNavigate, useParams} from "react-router-dom"
import {GET_CAST_VOTE} from "../queries/GetCastVote"
import {useQuery} from "@apollo/client"
import {GetBallotStylesQuery, GetCastVoteQuery} from "../gql/graphql"
import {faAngleLeft, faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {updateBallotStyleAndSelection} from "../services/BallotStyles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectFirstBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {getLanguageFromURL} from "../utils/queryParams"
import useLanguage from "../hooks/useLanguage"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"

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
`

function isHex(str: string) {
    if (str.trim() === "") {
        return true
    }

    const regex = /^[0-9a-fA-F]+$/
    return regex.test(str)
}

const BallotLocator: React.FC = () => {
    const {tenantId, eventId, electionId, ballotId} = useParams()
    const [openTitleHelp, setOpenTitleHelp] = useState<boolean>(false)
    const navigate = useNavigate()
    const location = useLocation()
    const {t, i18n} = useTranslation()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const [inputBallotId, setInputBallotId] = useState<string>("")

    const hasBallotId = !!ballotId
    const {data: dataBallotStyles} = useQuery<GetBallotStylesQuery>(GET_BALLOT_STYLES)
    const dispatch = useAppDispatch()
    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    useLanguage({ballotStyle})

    const {data, loading} = useQuery<GetCastVoteQuery>(GET_CAST_VOTE, {
        variables: {
            tenantId,
            electionEventId: eventId,
            electionId,
            ballotId,
        },
    })

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
        <>
            <PageLimit maxWidth="lg" className="ballot-locator-screen screen">
                <Box marginTop="48px">
                    <BreadCrumbSteps
                        labels={["ballotLocator.steps.lookup", "ballotLocator.steps.result"]}
                        selected={2}
                    />
                </Box>

                <Box sx={{display: "flex", justifyContent: "space-between"}}>
                    <Box>
                        <StyledTitle variant="h1">
                            {!hasBallotId ? (
                                <Box>
                                    {translateText(
                                        electionEvent,
                                        "ballotLocator.title",
                                        i18n.language,
                                        t("ballotLocator.title")
                                    )}
                                </Box>
                            ) : (
                                <Box>
                                    {translateText(
                                        electionEvent,
                                        "ballotLocator.titleResult",
                                        i18n.language,
                                        t("ballotLocator.titleResult")
                                    )}
                                </Box>
                            )}
                            <IconButton
                                icon={faCircleQuestion}
                                sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                                fontSize="16px"
                                onClick={() => setOpenTitleHelp(true)}
                            />
                            <Dialog
                                handleClose={() => setOpenTitleHelp(false)}
                                open={openTitleHelp}
                                title={translateText(
                                    electionEvent,
                                    "ballotLocator.titleHelpDialog.title",
                                    i18n.language,
                                    t("ballotLocator.titleHelpDialog.title")
                                )}
                                ok={translateText(
                                    electionEvent,
                                    "ballotLocator.titleHelpDialog.ok",
                                    i18n.language,
                                    t("ballotLocator.titleHelpDialog.ok")
                                )}
                                variant="info"
                            >
                                {stringToHtml(
                                    translateText(
                                        electionEvent,
                                        "ballotLocator.titleHelpDialog.content",
                                        i18n.language,
                                        t("ballotLocator.titleHelpDialog.content")
                                    )
                                )}
                            </Dialog>
                        </StyledTitle>

                        <Typography
                            variant="body1"
                            sx={{color: theme.palette.customGrey.contrastText}}
                        >
                            {translateText(
                                electionEvent,
                                "ballotLocator.description",
                                i18n.language,
                                t("ballotLocator.description")
                            )}
                        </Typography>
                    </Box>
                    <Box sx={{marginTop: "20px"}}>
                        <StyledLink
                            to={`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`}
                        >
                            <Button variant="secondary" className="secondary">
                                <Icon icon={faAngleLeft} size="sm" />
                                <Box paddingLeft="12px">
                                    {translateText(
                                        electionEvent,
                                        "votingScreen.backButton",
                                        i18n.language,
                                        t("votingScreen.backButton")
                                    )}
                                </Box>
                            </Button>
                        </StyledLink>
                    </Box>
                </Box>

                {hasBallotId && !loading && (
                    <Box>
                        {hasBallotId && !!ballotContent ? (
                            <MessageSuccess>
                                {translateText(
                                    electionEvent,
                                    "ballotLocator.found",
                                    i18n.language,
                                    t("ballotLocator.found", {ballotId})
                                )}
                            </MessageSuccess>
                        ) : (
                            <MessageFailed>
                                {translateText(
                                    electionEvent,
                                    "ballotLocator.notFound",
                                    i18n.language,
                                    t("ballotLocator.notFound", {ballotId})
                                )}
                            </MessageFailed>
                        )}
                    </Box>
                )}
                {!hasBallotId && (
                    <>
                        <TextField
                            onChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                                setInputBallotId(event.target.value)
                            }}
                            value={inputBallotId}
                            InputLabelProps={{
                                shrink: true,
                            }}
                            label="Ballot ID"
                            placeholder={translateText(
                                electionEvent,
                                "ballotLocator.description",
                                i18n.language,
                                t("ballotLocator.description")
                            )}
                            onKeyDown={captureEnter}
                        />
                        {!validatedBallotId && (
                            <StyledError>
                                {translateText(
                                    electionEvent,
                                    "ballotLocator.wrongFormatBallotId",
                                    i18n.language,
                                    t("ballotLocator.wrongFormatBallotId")
                                )}
                            </StyledError>
                        )}
                    </>
                )}

                {hasBallotId && ballotContent && (
                    <>
                        <Typography>
                            {translateText(
                                electionEvent,
                                "ballotLocator.contentDesc",
                                i18n.language,
                                t("ballotLocator.contentDesc")
                            )}
                        </Typography>
                        <InfoDataBox>{ballotContent}</InfoDataBox>
                    </>
                )}

                {!hasBallotId ? (
                    <Button
                        sx={{marginTop: "10px"}}
                        disabled={!validatedBallotId || inputBallotId.trim() === ""}
                        className="normal"
                        onClick={() => locate(true)}
                    >
                        <span>
                            {translateText(
                                electionEvent,
                                "ballotLocator.locate",
                                i18n.language,
                                t("ballotLocator.locate")
                            )}
                        </span>
                    </Button>
                ) : (
                    <>
                        <Button
                            sx={{marginTop: "10px"}}
                            className="normal"
                            onClick={() => locate()}
                        >
                            <span>
                                {translateText(
                                    electionEvent,
                                    "ballotLocator.locateAgain",
                                    i18n.language,
                                    t("ballotLocator.locateAgain")
                                )}
                            </span>
                        </Button>
                    </>
                )}
            </PageLimit>
        </>
    )
}

export default BallotLocator
