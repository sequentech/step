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

const StyleParagraphSuccess = styled(Typography)`
    margin-top: 16px;
    padding: 8px 16px;
    background-color: ${({theme}) => theme.palette.green.light};
`

const StyleParagraphDanger = styled(Typography)`
    margin-top: 16px;
    padding: 8px 16px;
    background-color: ${({theme}) => theme.palette.red.light};
`

export default function BallotLocator() {
    const {tenantId, eventId, electionId, ballotId} = useParams()
    const navigate = useNavigate()
    const {t} = useTranslation()

    const [inputBallotId, setInputBallotId] = useState<string>("")

    function locate(withBallotId = false) {
        let id = withBallotId ? inputBallotId : ""

        setInputBallotId("")

        navigate(`/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${id}`)
    }

    const hasBallotId = !!ballotId

    const {data, loading} = useQuery(GET_CAST_VOTE, {
        variables: {
            tenantId,
            electionEventId: eventId,
            electionId,
            ballotId,
        },
    })

    const ballotContent =
        data?.["sequent_backend_cast_vote"]?.find((item: any) => item.ballot_id === ballotId)
            ?.content ?? null

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
                            <StyleParagraphSuccess>
                                {t("ballotLocator.found", {ballotId})}
                            </StyleParagraphSuccess>
                        ) : (
                            <StyleParagraphDanger>
                                {t("ballotLocator.notFound", {ballotId})}
                            </StyleParagraphDanger>
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
                    <Button className="normal" onClick={() => locate(true)}>
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
