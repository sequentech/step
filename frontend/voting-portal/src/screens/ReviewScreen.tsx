// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useNavigate, useParams} from "react-router-dom"
import {fetchElectionByIdAsync, selectElectionById} from "../store/elections/electionsSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {Box} from "@mui/material"
import {
    PageLimit,
    BreadCrumbSteps,
    Icon,
    IconButton,
    theme,
    stringToHtml,
    BallotHash,
    isUndefined,
    Dialog,
} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import Typography from "@mui/material/Typography"
import {
    faCircleQuestion,
    faAngleLeft,
    faAngleRight,
    faFire,
} from "@fortawesome/free-solid-svg-icons"
import {IElectionDTO} from "sequent-core"
import {useTranslation} from "react-i18next"
import Button from "@mui/material/Button"
import {Link as RouterLink} from "react-router-dom"
import {selectAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {Question} from "../components/Question/Question"
import {useMutation} from "@apollo/client"
import { INSERT_CAST_VOTE } from "../queries/InsertCastVote"
import { InsertCastVoteMutation } from "../gql/graphql"
import { v4 as uuidv4 } from 'uuid'

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
    election: IElectionDTO
}

const ActionButtons: React.FC<ActionButtonProps> = ({election}) => {
    const [insertCastVote] = useMutation<InsertCastVoteMutation>(INSERT_CAST_VOTE)
    const {t} = useTranslation()
    const navigate = useNavigate()
    const [auditBallotHelp, setAuditBallotHelp] = useState(false)
    const handleClose = (value: boolean) => {
        setAuditBallotHelp(false)
        if (value) {
            navigate(`/election/${election.id}/audit`)
        }
    }

    const castBallotAction = async () => {
        try {
            await insertCastVote({
                variables: {
                    id: uuidv4(),
                    electionId: "f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
                    electionEventId: "33f18502-a67c-4853-8333-a58630663559",
                    tenantId: "f74bf7ee-824a-46fe-b3de-d773604e0552",
                    voterIdString: "voter-id-1",
                    content: "something",
                },
            })
            navigate(`/election/${election.id}/confirmation`)
        } catch (error) {
            console.log(`error casting vote: ${error}`)
        }
    }

    return (
        <Box sx={{marginBottom: "10px", marginTop: "10px"}}>
            <StyledButton
                sx={{display: {xs: "flex", sm: "none"}, marginBottom: "2px", width: "100%"}}
                variant="warning"
                onClick={() => setAuditBallotHelp(true)}
            >
                <Icon icon={faFire} size="sm" />
                <Box>{t("reviewScreen.auditButton")}</Box>
            </StyledButton>
            <Dialog
                handleClose={handleClose}
                open={auditBallotHelp}
                title={t("reviewScreen.auditBallotHelpDialog.title")}
                ok={t("reviewScreen.auditBallotHelpDialog.ok")}
                cancel={t("reviewScreen.auditBallotHelpDialog.cancel")}
                variant="warning"
            >
                {stringToHtml(t("reviewScreen.auditBallotHelpDialog.content"))}
            </Dialog>
            <ActionsContainer>
                <StyledLink
                    to={`/election/${election.id}/vote`}
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                >
                    <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                        <Icon icon={faAngleLeft} size="sm" />
                        <Box>{t("reviewScreen.backButton")}</Box>
                    </StyledButton>
                </StyledLink>
                <StyledButton
                    sx={{width: {xs: "100%", sm: "200px"}, display: {xs: "none", sm: "flex"}}}
                    variant="warning"
                    onClick={() => setAuditBallotHelp(true)}
                >
                    <Icon icon={faFire} size="sm" />
                    <Box>{t("reviewScreen.auditButton")}</Box>
                </StyledButton>
                <StyledButton sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}} onClick={castBallotAction}>
                    <Box>{t("reviewScreen.castBallotButton")}</Box>
                    <Icon icon={faAngleRight} size="sm" />
                </StyledButton>
            </ActionsContainer>
        </Box>
    )
}

export const ReviewScreen: React.FC = () => {
    const {electionId} = useParams<{electionId?: string}>()
    const election = useAppSelector(selectElectionById(Number(electionId)))
    const auditableBallot = useAppSelector(selectAuditableBallot(Number(electionId)))
    const dispatch = useAppDispatch()
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openReviewScreenHelp, setReviewScreenHelp] = useState(false)
    const {t} = useTranslation()

    useEffect(() => {
        if (!isUndefined(electionId) && isUndefined(election)) {
            dispatch(fetchElectionByIdAsync(Number(electionId)))
        }
    }, [electionId, election, dispatch])

    if (!election) {
        return <Box>Loading</Box>
    }

    return (
        <PageLimit maxWidth="lg">
            <BallotHash
                hash={auditableBallot?.ballot_hash || ""}
                onHelpClick={() => setOpenBallotIdHelp(true)}
            />
            <Dialog
                handleClose={() => setOpenBallotIdHelp(false)}
                open={openBallotIdHelp}
                title={t("reviewScreen.ballotIdHelpDialog.title")}
                ok={t("reviewScreen.ballotIdHelpDialog.ok")}
                cancel={t("reviewScreen.ballotIdHelpDialog.cancel")}
                variant="info"
            >
                {stringToHtml(t("reviewScreen.ballotIdHelpDialog.content"))}
            </Dialog>
            <Box marginTop="48px">
                <BreadCrumbSteps
                    labels={[
                        "breadcrumbSteps.electionList",
                        "breadcrumbSteps.ballot",
                        "breadcrumbSteps.review",
                        "breadcrumbSteps.confirmation",
                    ]}
                    selected={2}
                />
            </Box>
            <StyledTitle variant="h4" fontSize="24px" fontWeight="bold" sx={{margin: 0}}>
                <Box>{t("reviewScreen.title")}</Box>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setReviewScreenHelp(true)}
                />
                <Dialog
                    handleClose={() => setReviewScreenHelp(false)}
                    open={openReviewScreenHelp}
                    title={t("reviewScreen.reviewScreenHelpDialog.title")}
                    ok={t("reviewScreen.reviewScreenHelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("reviewScreen.reviewScreenHelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {stringToHtml(t("reviewScreen.description"))}
            </Typography>
            {election.configuration.questions.map((question, index) => (
                <Question
                    election={election}
                    question={question}
                    key={index}
                    questionIndex={index}
                    isReview={true}
                />
            ))}
            <ActionButtons election={election} />
        </PageLimit>
    )
}
