// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
//import {fetchElectionByIdAsync} from "../store/elections/electionsSlice"
import {IBallotStyle, selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {Box} from "@mui/material"
import {
    PageLimit,
    BreadCrumbSteps,
    Icon,
    IconButton,
    theme,
    stringToHtml,
    isUndefined,
    Dialog,
} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import Typography from "@mui/material/Typography"
import {faCircleQuestion, faAngleLeft, faAngleRight} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import Button from "@mui/material/Button"
import {Link as RouterLink, useNavigate, useParams} from "react-router-dom"
import {selectBallotSelectionByElectionId} from "../store/ballotSelections/ballotSelectionsSlice"
import {provideBallotService} from "../services/BallotService"
import {setAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {Question} from "../components/Question/Question"

const StyledLink = styled(RouterLink)`
    margin: auto 0;
    text-decoration: none;
`

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

const ActionsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    margin-bottom: 20px;
    margin-top: 10px;
    gap: 2px;
`

const StyledButton = styled(Button)`
    display flex;
    padding: 5px;

    span {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        padding: 5px;
    }
`

interface ActionButtonProps {
    ballotStyle: IBallotStyle
}

const ActionButtons: React.FC<ActionButtonProps> = ({ballotStyle}) => {
    const {t} = useTranslation()
    const {encryptBallotSelection} = provideBallotService()
    const selectionState = useAppSelector(selectBallotSelectionByElectionId(ballotStyle.election_id))
    const navigate = useNavigate()
    const dispatch = useAppDispatch()

    const encryptAndReview = () => {
        if (isUndefined(selectionState)) {
            return
        }
        try {
            const auditableBallot = encryptBallotSelection(selectionState, ballotStyle.ballot_eml)
            console.log("Success encrypting ballot:")
            console.log(auditableBallot)
            dispatch(setAuditableBallot({
                electionId: ballotStyle.election_id,
                auditableBallot,
            }))
            navigate(`/election/${ballotStyle.election_id}/review`)
        } catch (error) {
            console.log("ERROR encrypting ballot:")
            console.log(error)
        }
    }

    return (
        <ActionsContainer>
            <StyledLink to="/" sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}>
                <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                    <Icon icon={faAngleLeft} size="sm" />
                    <Box>{t("votingScreen.backButton")}</Box>
                </StyledButton>
            </StyledLink>
            <StyledButton sx={{width: {xs: "100%", sm: "200px"}}} onClick={encryptAndReview}>
                <Box>{t("votingScreen.reviewButton")}</Box>
                <Icon icon={faAngleRight} size="sm" />
            </StyledButton>
        </ActionsContainer>
    )
}

export const VotingScreen: React.FC = () => {
    const {electionId} = useParams<{electionId?: string}>()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const {t} = useTranslation()
    const dispatch = useAppDispatch()
    const [openBallotHelp, setOpenBallotHelp] = useState(false)

    useEffect(() => {
        if (!isUndefined(electionId) && isUndefined(ballotStyle)) {
            //dispatch(fetchElectionByIdAsync(Number(electionId)))
        }
    }, [electionId, ballotStyle, dispatch])

    if (!ballotStyle) {
        return <Box>Loading</Box>
    }

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
                    selected={1}
                />
            </Box>
            <StyledTitle variant="h4">
                <Box>{ballotStyle.ballot_eml.configuration.title}</Box>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setOpenBallotHelp(true)}
                />
                <Dialog
                    handleClose={() => setOpenBallotHelp(false)}
                    open={openBallotHelp}
                    title={t("votingScreen.ballotHelpDialog.title")}
                    ok={t("votingScreen.ballotHelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("votingScreen.ballotHelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            {ballotStyle.ballot_eml.configuration.description ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(ballotStyle.ballot_eml.configuration.description)}
                </Typography>
            ) : null}
            {ballotStyle.ballot_eml.configuration.questions.map((question, index) => (
                <Question
                    ballotStyle={ballotStyle}
                    question={question}
                    questionIndex={index}
                    key={index}
                    isReview={false}
                />
            ))}
            <ActionButtons ballotStyle={ballotStyle} />
        </PageLimit>
    )
}
