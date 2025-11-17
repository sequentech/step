// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useEffect, useMemo, PropsWithChildren} from "react"
import Typography from "@mui/material/Typography"
import Paper, {PaperProps} from "@mui/material/Paper"
import Box from "@mui/material/Box"
import {useNavigate} from "react-router-dom"
import {Link as RouterLink} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import Skeleton from "@mui/material/Skeleton"
import {IBallotService, IConfirmationBallot} from "../services/BallotService"
import {IDecodedVoteContest, checkIsBlank} from "@sequentech/ui-core"
import Button from "@mui/material/Button"
import {
    faCircleQuestion,
    faTimesCircle,
    faPrint,
    faAngleLeft,
} from "@fortawesome/free-solid-svg-icons"
import {
    Candidate,
    Icon,
    IconButton,
    BreadCrumbSteps,
    PageLimit,
    WarnBox,
    Dialog,
    theme,
    BlankAnswer,
} from "@sequentech/ui-essentials"
import {translate, ICandidate, IContest, EInvalidVotePolicy} from "@sequentech/ui-core"
import {keyBy} from "lodash"
import Image from "mui-image"
import {checkIsInvalidVote, checkIsWriteIn, getImageUrl} from "../services/ElectionConfigService"
import {provideBallotService} from "../services/BallotService"

const StyledLink = styled(RouterLink)`
    margin: auto 0;
    text-decoration: none;
`

const HorizontalWrap = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 16px;
    margin-bottom: 12px;
`

const BallotIdPaper = styled(Paper)`
    padding: 10px 16px;
    display: flex;
    overflow: auto;
`

const OneLine = styled(Paper)`
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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

const CandidatesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 12px 0;
`

interface VoteChoiceProps {
    text?: string
    points: number | null
    ordered: boolean
}

const VoteChoice: React.FC<VoteChoiceProps> = ({text, points, ordered}) => {
    const {t, i18n} = useTranslation()

    const content = (
        <Typography variant="body2">
            <li>
                <span>
                    {text} {points ? <>{t("confirmationScreen.points", {points})}</> : null}
                </span>
            </li>
        </Typography>
    )

    return ordered ? <ol>{content}</ol> : <ul>{content}</ul>
}

interface CandidateChoiceProps {
    answer?: ICandidate
    points: number | null
    ordered: boolean
    isWriteIn: boolean
    writeInValue: string | undefined
    isPreferentialVote?: boolean
    selectedPosition?: number | null
}

const CandidateChoice: React.FC<CandidateChoiceProps> = ({
    answer,
    isWriteIn,
    writeInValue,
    isPreferentialVote,
    selectedPosition,
}) => {
    const imageUrl = answer && getImageUrl(answer)

    return (
        <Candidate
            title={answer?.name || ""}
            description={answer?.description}
            isWriteIn={isWriteIn}
            writeInValue={writeInValue}
            shouldDisable={false}
            isPreferentialVote={isPreferentialVote}
            selectedPosition={selectedPosition}
        >
            {imageUrl ? <Image src={imageUrl} duration={100} /> : null}
        </Candidate>
    )
}

interface PlaintextVoteQuestionProps {
    questionPlaintext: IDecodedVoteContest
    question: IContest | null
    ballotService: IBallotService
}

const PlaintextVoteQuestion: React.FC<PlaintextVoteQuestionProps> = ({
    questionPlaintext,
    question,
    ballotService,
}) => {
    const {t, i18n} = useTranslation()
    const selectedAnswers = questionPlaintext.choices.filter((a) => a.selected > -1)
    if (!question) {
        return (
            <>
                {t("confirmationScreen.contestNotFound", {contestId: questionPlaintext.contest_id})}
            </>
        )
    }

    const {isPreferential} = provideBallotService()
    const isPreferentialVote = isPreferential(question.counting_algorithm)

    const explicitInvalidAnswer =
        (questionPlaintext.is_explicit_invalid &&
            question.presentation?.invalid_vote_policy !== EInvalidVotePolicy.NOT_ALLOWED &&
            question.candidates.find((answer) => checkIsInvalidVote(answer))) ||
        null
    const answersById = keyBy(question.candidates, (a) => a.id)
    const properties = ballotService.getLayoutProperties(question)
    const showPoints = !!question.presentation?.show_points
    const isBlank = checkIsBlank(questionPlaintext)

    return (
        <>
            <Typography variant="body2" fontWeight={"bold"}>
                {translate(question, "name", i18n.language) || ""}
            </Typography>
            {isBlank ? <BlankAnswer /> : null}
            {questionPlaintext.invalid_errors.map((error, index) => (
                <WarnBox variant="warning" key={index}>
                    {t(
                        error.message || "",
                        error.message_map && Object.fromEntries(error.message_map)
                    )}
                </WarnBox>
            ))}
            {questionPlaintext.is_explicit_invalid ? (
                <VoteChoice
                    text={explicitInvalidAnswer?.name || t("confirmationScreen.markedInvalid")}
                    points={null}
                    ordered={properties?.ordered || false}
                />
            ) : null}
            <CandidatesWrapper>
                {selectedAnswers.map((answer, index) => (
                    <CandidateChoice
                        key={index}
                        answer={answersById[answer.id]}
                        points={(showPoints && ballotService.getPoints(question, answer)) || null}
                        ordered={properties?.ordered || false}
                        isWriteIn={checkIsWriteIn(answersById[answer.id])}
                        writeInValue={answer.write_in_text}
                        isPreferentialVote={isPreferentialVote}
                        selectedPosition={answer.selected + 1}
                    />
                ))}
            </CandidatesWrapper>
        </>
    )
}

enum VariantType {
    Info = "info",
    Error = "error",
}

interface BallotIdContainerProps extends PaperProps {
    variant: VariantType
}

const BallotIdContainer: React.FC<PropsWithChildren<BallotIdContainerProps>> = ({
    variant,
    children,
    ...props
}) => (
    <BallotIdPaper variant={variant} {...props}>
        {children}
    </BallotIdPaper>
)

interface BallotIdSectionProps {
    confirmationBallot: IConfirmationBallot | null
    ballotId: string
}

const isMatchingBallotIds = (
    confirmationBallotId: String | undefined,
    ballotId: String
): boolean => {
    return confirmationBallotId === ballotId
}

const ballotMatchVariantType = (
    confirmationBallotId: string | undefined,
    ballotId: string
): VariantType => {
    return isMatchingBallotIds(confirmationBallotId, ballotId)
        ? VariantType.Info
        : VariantType.Error
}

const BallotIdSection: React.FC<BallotIdSectionProps> = ({confirmationBallot, ballotId}) => {
    const {t} = useTranslation()
    const [decodedBallotIdHelp, setDecodedBallotIdHelp] = useState(false)
    const [userBallotIdHelp, setUserBallotIdHelp] = useState(false)

    return (
        <>
            <Typography variant="h5">{t("confirmationScreen.ballotIdTitle")}</Typography>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {t("confirmationScreen.ballotIdDescription")}
            </Typography>
            <HorizontalWrap>
                <Typography variant="h5" fontSize="16px" width="106px">
                    {t("confirmationScreen.decodedBallotId")}
                </Typography>
                <BallotIdContainer variant={VariantType.Info}>
                    <OneLine variant="info">{confirmationBallot?.ballot_hash}</OneLine>
                    <IconButton
                        icon={faCircleQuestion}
                        sx={{
                            fontSize: "unset",
                            lineHeight: "unset",
                            paddingBottom: "2px",
                            color: theme.palette.black,
                            marginLeft: "10px",
                        }}
                        fontSize="16px"
                        onClick={() => setDecodedBallotIdHelp(true)}
                    />
                    <Dialog
                        handleClose={() => setDecodedBallotIdHelp(false)}
                        open={decodedBallotIdHelp}
                        title={t("confirmationScreen.decodedBallotIdHelpDialog.title")}
                        ok={t("confirmationScreen.decodedBallotIdHelpDialog.ok")}
                        variant="info"
                    >
                        <p>{t("confirmationScreen.decodedBallotIdHelpDialog.content")}</p>
                    </Dialog>
                </BallotIdContainer>
            </HorizontalWrap>
            <HorizontalWrap>
                <Typography variant="h5" fontSize="16px" width="106px">
                    {t("confirmationScreen.yourBallotId")}
                </Typography>
                <Box sx={{overflow: "auto"}}>
                    <BallotIdContainer
                        variant={ballotMatchVariantType(confirmationBallot?.ballot_hash, ballotId)}
                        sx={{
                            marginTop: isMatchingBallotIds(
                                confirmationBallot?.ballot_hash,
                                ballotId
                            )
                                ? undefined
                                : "14px",
                        }}
                    >
                        {isMatchingBallotIds(confirmationBallot?.ballot_hash, ballotId) ? null : (
                            <IconButton
                                icon={faTimesCircle}
                                sx={{
                                    fontSize: "unset",
                                    lineHeight: "unset",
                                    paddingBottom: "2px",
                                    marginRight: "10px",
                                }}
                                fontSize="16px"
                            />
                        )}
                        <OneLine
                            variant={ballotMatchVariantType(
                                confirmationBallot?.ballot_hash,
                                ballotId
                            )}
                        >
                            {ballotId}
                        </OneLine>
                        <IconButton
                            icon={faCircleQuestion}
                            sx={{
                                fontSize: "unset",
                                lineHeight: "unset",
                                paddingBottom: "2px",
                                color: theme.palette.black,
                                marginLeft: "10px",
                            }}
                            fontSize="16px"
                            onClick={() => setUserBallotIdHelp(true)}
                        />
                        <Dialog
                            handleClose={() => setUserBallotIdHelp(false)}
                            open={userBallotIdHelp}
                            title={t("confirmationScreen.userBallotIdHelpDialog.title")}
                            ok={t("confirmationScreen.userBallotIdHelpDialog.ok")}
                            variant="info"
                        >
                            <p>{t("confirmationScreen.userBallotIdHelpDialog.content")}</p>
                        </Dialog>
                    </BallotIdContainer>
                    {isMatchingBallotIds(confirmationBallot?.ballot_hash, ballotId) ? null : (
                        <Typography fontSize="12px" color={theme.palette.red.dark} marginTop="2px">
                            {t("confirmationScreen.ballotIdError")}
                        </Typography>
                    )}
                </Box>
            </HorizontalWrap>
        </>
    )
}

interface ActionButtonProps {}

const ActionButtons: React.FC<ActionButtonProps> = () => {
    const {t} = useTranslation()
    const triggerPrint = () => window.print()

    return (
        <ActionsContainer>
            <StyledLink to="/" sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}>
                <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                    <Icon icon={faAngleLeft} size="sm" />
                    <span>{t("confirmationScreen.backButton")}</span>
                </StyledButton>
            </StyledLink>
            <StyledButton
                onClick={triggerPrint}
                variant="secondary"
                sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
            >
                <Icon icon={faPrint} size="sm" />
                <Box>{t("confirmationScreen.printButton")}</Box>
            </StyledButton>
            {/*<StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                <span>{t("confirmationScreen.finishButton")}</span>
                <Icon icon={faAngleRight} size="sm" />
            </StyledButton>*/}
        </ActionsContainer>
    )
}

interface VerifySelectionsSectionProps {
    isLoading: boolean
    confirmationBallot: IConfirmationBallot | null
    ballotService: IBallotService
}

const VerifySelectionsSection: React.FC<VerifySelectionsSectionProps> = ({
    isLoading,
    confirmationBallot,
    ballotService,
}) => {
    const {t} = useTranslation()
    const [verifySelectionsHelp, setVerifySelectionsHelp] = useState(false)
    const plaintextVoteQuestions = confirmationBallot?.decoded_questions || []
    const questionsMap = keyBy(confirmationBallot?.election_config.contests || [], "id")

    return (
        <>
            <HorizontalWrap marginTop="26px">
                <Typography variant="h5">
                    {t("confirmationScreen.verifySelectionsTitle")}
                </Typography>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{
                        fontSize: "unset",
                        lineHeight: "unset",
                        paddingBottom: "2px",
                    }}
                    fontSize="16px"
                    onClick={() => setVerifySelectionsHelp(true)}
                />
                <Dialog
                    handleClose={() => setVerifySelectionsHelp(false)}
                    open={verifySelectionsHelp}
                    title={t("confirmationScreen.verifySelectionsHelpDialog.title")}
                    ok={t("confirmationScreen.verifySelectionsHelpDialog.ok")}
                    variant="info"
                >
                    <p>{t("confirmationScreen.verifySelectionsHelpDialog.content")}</p>
                </Dialog>
            </HorizontalWrap>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {t("confirmationScreen.verifySelectionsDescription")}
            </Typography>
            {isLoading ? (
                <>
                    <Skeleton variant="text" />
                    <Skeleton variant="text" />
                </>
            ) : (
                <>
                    <Typography variant="h5" textAlign="left">
                        {confirmationBallot?.election_config.description}
                    </Typography>
                    <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                        {confirmationBallot?.election_config.description}
                    </Typography>
                </>
            )}
            {isLoading ? (
                <>
                    <Skeleton variant="text" />
                    <Skeleton variant="text" />
                    <Skeleton variant="text" width={200} />
                    <Skeleton variant="text" width={50} />
                </>
            ) : (
                <>
                    {plaintextVoteQuestions.map((voteQuestion) => (
                        <PlaintextVoteQuestion
                            questionPlaintext={voteQuestion}
                            question={questionsMap[voteQuestion.contest_id] ?? null}
                            ballotService={ballotService}
                            key={voteQuestion.contest_id}
                        />
                    ))}
                </>
            )}
        </>
    )
}

interface IProps {
    confirmationBallot: IConfirmationBallot | null
    ballotService: IBallotService
    ballotId: string
    label?: string
}

export const ConfirmationScreen: React.FC<IProps> = ({
    confirmationBallot,
    ballotService,
    ballotId,
}) => {
    const navigate = useNavigate()
    const [isLoading, setIsLoading] = useState(confirmationBallot === null)

    useEffect(() => {
        setIsLoading(confirmationBallot === null)
        if (confirmationBallot == null) {
            navigate("/")
        }
    }, [confirmationBallot])

    return (
        <PageLimit maxWidth="md">
            <Box marginTop="48px" marginBottom="24px">
                <BreadCrumbSteps
                    labels={[
                        "breadcrumbSteps.import",
                        "breadcrumbSteps.verify",
                        //"breadcrumbSteps.finish",
                    ]}
                    selected={1}
                />
            </Box>
            <BallotIdSection confirmationBallot={confirmationBallot} ballotId={ballotId} />
            {isMatchingBallotIds(confirmationBallot?.ballot_hash, ballotId) ? (
                <VerifySelectionsSection
                    confirmationBallot={confirmationBallot}
                    isLoading={isLoading}
                    ballotService={ballotService}
                />
            ) : null}
            <ActionButtons />
        </PageLimit>
    )
}
