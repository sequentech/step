// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {PageLimit, theme} from "@sequentech/ui-essentials"
import {Box, TextField, Typography, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import {Link, useNavigate, useParams} from "react-router-dom"
import {GET_CAST_VOTE} from "../queries/GetCastVote"
import {useQuery} from "@apollo/client"

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
    margin-top: 25.5px;
    padding: 8px 16px;
    background-color: ${({theme}) => theme.palette.green.light};
`

const StyleParagraphDanger = styled(Typography)`
    margin-top: 25.5px;
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

    return (
        <>
            <PageLimit maxWidth="lg">
                <Box marginTop="48px"></Box>
                <StyledTitle variant="h1">
                    <Box>{t("ballotLocator.title")}</Box>
                </StyledTitle>
                {
                    // <Typography variant="body1" sx={{color: theme.palette.customGrey.contrastText}}>
                    //     {t("ballotLocator.description")}
                    // </Typography>
                }

                {hasBallotId && !loading && (
                    <Box>
                        {data &&
                        data["sequent_backend_cast_vote"]
                            .map((item: any) => item.ballot_id)
                            .some((id: string) => id === ballotId) ? (
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
                        <Button className="normal" onClick={() => locate(true)}>
                            <span>{t("ballotLocator.locate")}</span>
                        </Button>
                    </>
                )}

                {hasBallotId && (
                    <>
                        <Button className="normal" onClick={() => locate()}>
                            <span>{t("ballotLocator.locateAgain")}</span>
                        </Button>
                    </>
                )}

                <Box sx={{marginTop: "32px"}}>
                    <Link to={`/tenant/${tenantId}/event/${eventId}/election-chooser`}>
                        {t("votingScreen.backButton")}
                    </Link>
                </Box>
            </PageLimit>
        </>
    )
}
