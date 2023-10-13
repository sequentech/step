// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
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
import {
    selectBallotSelectionByElectionId,
    setBallotSelection,
} from "../store/ballotSelections/ballotSelectionsSlice"
import {provideBallotService} from "../services/BallotService"
import {setAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {Question} from "../components/Question/Question"
import {CircularProgress} from "@mui/material"
import {selectElectionById} from "../store/elections/electionsSlice"

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
    disableNext: boolean
}

const ActionButtons: React.FC<ActionButtonProps> = ({ballotStyle, disableNext}) => {
    const {t} = useTranslation()
    const {encryptBallotSelection, decodeAuditableBallot} = provideBallotService()
    const selectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle.election_id)
    )
    const navigate = useNavigate()
    const dispatch = useAppDispatch()

    const encryptAndReview = () => {
        if (isUndefined(selectionState) || disableNext) {
            return
        }
        try {
            const startMs = Date.now()
            const auditableBallot = encryptBallotSelection(selectionState, ballotStyle.ballot_eml)
            const endMs = Date.now()
            console.log(`Success encrypting ballot: ${endMs - startMs} ms`)
            console.log(auditableBallot)
            dispatch(
                setAuditableBallot({
                    electionId: ballotStyle.election_id,
                    auditableBallot,
                })
            )
            let decodedSelectionState = decodeAuditableBallot(auditableBallot)
            if (null !== decodedSelectionState) {
                dispatch(
                    setBallotSelection({
                        ballotStyle,
                        ballotSelection: decodedSelectionState,
                    })
                )
            }
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
            <StyledButton
                sx={{width: {xs: "100%", sm: "200px"}}}
                onClick={encryptAndReview}
                disabled={disableNext}
            >
                <Box>{t("votingScreen.reviewButton")}</Box>
                <Icon icon={faAngleRight} size="sm" />
            </StyledButton>
        </ActionsContainer>
    )
}

export const VotingScreen: React.FC = () => {
    let [disableNext, setDisableNext] = useState<Record<string, boolean>>({})
    const {electionId} = useParams<{electionId?: string}>()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const election = useAppSelector(selectElectionById(String(electionId)))
    const {t} = useTranslation()
    const [openBallotHelp, setOpenBallotHelp] = useState(false)

    const onSetDisableNext = (id: string) => (value: boolean) => {
        setDisableNext({
            ...disableNext,
            [id]: value,
        })
    }

    if (!ballotStyle || !election) {
        return <CircularProgress />
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
                <Box>{election.name || ""}</Box>
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
            {election.description ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(election.description)}
                </Typography>
            ) : null}
            {ballotStyle.ballot_eml.configuration.questions.map((question, index) => (
                <Question
                    ballotStyle={ballotStyle}
                    question={question}
                    questionIndex={index}
                    key={index}
                    isReview={false}
                    setDisableNext={onSetDisableNext(question.id)}
                />
            ))}
            <ActionButtons
                ballotStyle={ballotStyle}
                disableNext={Object.values(disableNext).some((v) => v)}
            />
        </PageLimit>
    )
}
