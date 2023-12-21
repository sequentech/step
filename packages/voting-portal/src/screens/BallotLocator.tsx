// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {PageLimit, theme} from "@sequentech/ui-essentials"
import {Box, TextField, Typography, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useNavigate, useParams} from "react-router-dom"
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

    const {data} = useQuery(GET_CAST_VOTE, {
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
                <Typography variant="body1" sx={{color: theme.palette.customGrey.contrastText}}>
                    {t("ballotLocator.description")}
                </Typography>

                {hasBallotId && (
                    <Box>
                        {data &&
                        data["sequent_backend_cast_vote"]
                            .map((item: any) => item.ballot_id)
                            .some((id: string) => id === ballotId) ? (
                            <p>hello</p>
                        ) : (
                            <p>good bye</p>
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
                            placeholder="Type in your Ballot ID"
                        />
                        <Button className="normal" onClick={() => locate(true)}>
                            <span>{t("ballotLocator.locate")}</span>
                        </Button>
                    </>
                )}

                {hasBallotId && (
                    <>
                        <Button className="normal" onClick={() => locate()}>
                            <span>{t("ballotLocator.locate")}</span>
                        </Button>
                    </>
                )}
            </PageLimit>
        </>
    )
}
