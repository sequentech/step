import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {PageLimit, theme} from "@sequentech/ui-essentials"
import {Box, TextField, Typography, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useNavigate, useParams} from "react-router-dom"

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

    function locate() {
        navigate(
            `/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${inputBallotId}`
        )
    }

    const hasBallotId = !!ballotId

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
                <Button className="normal" onClick={() => locate()}>
                    <span>{t("ballotLocator.locate")}</span>
                </Button>
            </PageLimit>
        </>
    )
}
