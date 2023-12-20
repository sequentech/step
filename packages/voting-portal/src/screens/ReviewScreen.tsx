// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useState} from "react"
import {useNavigate, useParams} from "react-router-dom"
//import {fetchElectionByIdAsync} from "../store/elections/electionsSlice"
import {IBallotStyle, selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {useAppSelector} from "../store/hooks"
import {Box} from "@mui/material"
import {
    PageLimit,
    BreadCrumbSteps,
    Icon,
    IconButton,
    theme,
    stringToHtml,
    BallotHash,
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
import {useTranslation} from "react-i18next"
import Button from "@mui/material/Button"
import {Link as RouterLink} from "react-router-dom"
import {selectAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {Question} from "../components/Question/Question"
import {useMutation} from "@apollo/client"
import {INSERT_CAST_VOTE} from "../queries/InsertCastVote"
import {InsertCastVoteMutation} from "../gql/graphql"
import {v4 as uuidv4} from "uuid"
import {CircularProgress} from "@mui/material"
import {hashBallot, provideBallotService} from "../services/BallotService"
import {TenantEventContext} from ".."

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
    ballotStyle: IBallotStyle
    auditableBallot: string
}

const ActionButtons: React.FC<ActionButtonProps> = ({ballotStyle, auditableBallot}) => {
    const [insertCastVote] = useMutation<InsertCastVoteMutation>(INSERT_CAST_VOTE)
    const {tenantId, eventId} = useContext(TenantEventContext)
    const {t} = useTranslation()
    const navigate = useNavigate()
    const [auditBallotHelp, setAuditBallotHelp] = useState(false)
    const {toHashableBallot} = provideBallotService()
    const ballotId = hashBallot(auditableBallot)

    const handleClose = (value: boolean) => {
        setAuditBallotHelp(false)
        if (value) {
            navigate(
                `/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/audit`
            )
        }
    }

    const castBallotAction = async () => {
        try {
            const hashableBallot = toHashableBallot(auditableBallot)

            await insertCastVote({
                variables: {
                    id: uuidv4(),
                    ballotId,
                    electionId: ballotStyle.election_id,
                    electionEventId: ballotStyle.election_event_id,
                    tenantId: ballotStyle.tenant_id,
                    areaId: ballotStyle.area_id,
                    content: hashableBallot,
                },
            })
            navigate(
                `/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/confirmation`
            )
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
                    to={`/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/vote`}
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
                <StyledButton
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                    onClick={castBallotAction}
                >
                    <Box>{t("reviewScreen.castBallotButton")}</Box>
                    <Icon icon={faAngleRight} size="sm" />
                </StyledButton>
            </ActionsContainer>
        </Box>
    )
}

export const ReviewScreen: React.FC = () => {
    const {electionId} = useParams<{electionId?: string}>()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openReviewScreenHelp, setReviewScreenHelp] = useState(false)
    const {t} = useTranslation()
    const {hashBallot} = provideBallotService()
    const ballotHash = auditableBallot && hashBallot(auditableBallot)

    if (!ballotStyle || !auditableBallot) {
        return <CircularProgress />
    }

    return (
        <PageLimit maxWidth="lg">
            <BallotHash hash={ballotHash || ""} onHelpClick={() => setOpenBallotIdHelp(true)} />
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
            {ballotStyle.ballot_eml.contests.map((question, index) => (
                <Question
                    ballotStyle={ballotStyle}
                    question={question}
                    key={index}
                    questionIndex={index}
                    isReview={true}
                />
            ))}
            <ActionButtons ballotStyle={ballotStyle} auditableBallot={auditableBallot} />
        </PageLimit>
    )
}
