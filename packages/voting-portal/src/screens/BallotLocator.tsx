// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {BreadCrumbSteps, PageLimit, theme} from "@sequentech/ui-essentials"
import {Box, TextField, Typography, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import {Link, useNavigate, useParams} from "react-router-dom"
import {GET_CAST_VOTE} from "../queries/GetCastVote"
import {useQuery} from "@apollo/client"

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

export default function BallotLocator() {
    const {tenantId, eventId, electionId, ballotId} = useParams()
    const navigate = useNavigate()
    const {t} = useTranslation()

    const [inputBallotId, setInputBallotId] = useState<string>("")

    const hasBallotId = !!ballotId

    const {data, loading} = useQuery(GET_CAST_VOTE, {
        variables: {
            tenantId,
            electionEventId: eventId,
            electionId,
            ballotId,
        },
    })

    const validatedBallotId = isHex(inputBallotId ?? "")

    const ballotContent =
        data?.["sequent_backend_cast_vote"]?.find((item: any) => item.ballot_id === ballotId)
            ?.content ?? null

    function locate(withBallotId = false) {
        let id = withBallotId ? inputBallotId : ""

        setInputBallotId("")

        navigate(`/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${id}`)
    }

    return (
        <>
            <PageLimit maxWidth="lg">
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
                                <Box>{t("ballotLocator.title")}</Box>
                            ) : (
                                <Box>{t("ballotLocator.titleResult")}</Box>
                            )}
                        </StyledTitle>

                        <Typography
                            variant="body1"
                            sx={{color: theme.palette.customGrey.contrastText}}
                        >
                            {t("ballotLocator.description")}
                        </Typography>
                    </Box>
                    <Box sx={{marginTop: "20px"}}>
                        <StyledLink to={`/tenant/${tenantId}/event/${eventId}/election-chooser`}>
                            <Button variant="secondary" className="secondary">
                                {t("votingScreen.backButton")}
                            </Button>
                        </StyledLink>
                    </Box>
                </Box>

                {hasBallotId && !loading && (
                    <Box>
                        {hasBallotId && !!ballotContent ? (
                            <MessageSuccess>{t("ballotLocator.found", {ballotId})}</MessageSuccess>
                        ) : (
                            <MessageFailed>{t("ballotLocator.notFound", {ballotId})}</MessageFailed>
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
                            placeholder={t("ballotLocator.description")}
                        />
                        {!validatedBallotId && (
                            <StyledError>{t("ballotLocator.wrongFormatBallotId")}</StyledError>
                        )}
                    </>
                )}

                <Box sx={{height: "250px"}}>
                    {hasBallotId && ballotContent && (
                        <>
                            <Typography>{t("ballotLocator.contentDesc")}</Typography>
                            <Box sx={{wordWrap: "break-word", fontFamily: "monospace"}}>
                                {ballotContent}
                            </Box>
                        </>
                    )}
                </Box>

                {!hasBallotId ? (
                    <Button
                        disabled={!validatedBallotId || inputBallotId.trim() === ""}
                        className="normal"
                        onClick={() => locate(true)}
                    >
                        <span>{t("ballotLocator.locate")}</span>
                    </Button>
                ) : (
                    <>
                        <Button className="normal" onClick={() => locate()}>
                            <span>{t("ballotLocator.locateAgain")}</span>
                        </Button>
                    </>
                )}
            </PageLimit>
        </>
    )
}
