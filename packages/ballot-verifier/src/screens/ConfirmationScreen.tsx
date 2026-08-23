// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useEffect, useMemo, PropsWithChildren, useContext} from "react"
import Typography from "@mui/material/Typography"
import Paper, {PaperProps} from "@mui/material/Paper"
import Box from "@mui/material/Box"
import {useNavigate} from "react-router-dom"
import {Link as RouterLink} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import Skeleton from "@mui/material/Skeleton"
import {IConfirmationBallot} from "../services/BallotService"
import {
    faCircleQuestion,
    faTimesCircle,
    faPrint,
    faAngleLeft,
} from "@fortawesome/free-solid-svg-icons"
import {
    Icon,
    IconButton,
    BreadCrumbSteps,
    PageLimit,
    Dialog,
    theme,
    ActionsContainer,
    StyledButton,
    PlaintextVoteContest,
} from "@sequentech/ui-essentials"
import {sortContestList} from "@sequentech/ui-core"
import {keyBy} from "lodash"
import {useElectionClassName} from "./hooks/useElectionClassName"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {
    EDeclineToVotePolicy,
    EElectionEventContestEncryptionPolicy,
    EBlankBallotsPolicy,
} from "@sequentech/ui-core"

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
        <ActionsContainer sx={{marginBottom: "20px", marginTop: "10px"}}>
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
}

const VerifySelectionsSection: React.FC<VerifySelectionsSectionProps> = ({
    isLoading,
    confirmationBallot,
}) => {
    const {t} = useTranslation()
    const [verifySelectionsHelp, setVerifySelectionsHelp] = useState(false)
    const plaintextVoteQuestions = confirmationBallot?.decoded_questions || []
    const questionsMap = keyBy(confirmationBallot?.election_config.contests || [], "id")
    const contestsOrderType =
        confirmationBallot?.election_config.election_presentation?.contests_order
    const sortedPlaintextVoteQuestions = useMemo(() => {
        if (!plaintextVoteQuestions.length) {
            return []
        }

        const sortedContests = sortContestList(
            confirmationBallot?.election_config.contests || [],
            contestsOrderType
        )
        const contestIndexMap = new Map(
            sortedContests.map((contest, index) => [contest.id, index] as const)
        )

        return [...plaintextVoteQuestions].sort((a, b) => {
            const firstIndex = contestIndexMap.get(a.contest_id) ?? Number.MAX_SAFE_INTEGER
            const secondIndex = contestIndexMap.get(b.contest_id) ?? Number.MAX_SAFE_INTEGER
            return firstIndex - secondIndex
        })
    }, [confirmationBallot?.election_config.contests, contestsOrderType, plaintextVoteQuestions])
    const {globalSettings} = useContext(SettingsContext)

    const isDeclineToVotePolicyEnabled =
        confirmationBallot?.election_config?.election_presentation?.decline_to_vote_policy ===
            EDeclineToVotePolicy.ENABLED &&
        confirmationBallot?.election_config?.election_event_presentation
            ?.contest_encryption_policy === EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS

    const isBlankBallotsPolicyEnabled =
        confirmationBallot?.election_config?.election_presentation?.blank_ballots_policy ===
            EBlankBallotsPolicy.ENABLED &&
        confirmationBallot?.election_config?.election_event_presentation
            ?.contest_encryption_policy === EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS

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
                    {sortedPlaintextVoteQuestions.map((voteQuestion) => (
                        <PlaintextVoteContest
                            questionPlaintext={voteQuestion}
                            question={questionsMap[voteQuestion.contest_id] ?? null}
                            key={voteQuestion.contest_id}
                            publicBucketUrl={globalSettings.PUBLIC_BUCKET_URL}
                            contestNotFoundLabel={t("confirmationScreen.contestNotFound", {
                                contestId: voteQuestion.contest_id,
                            })}
                            markedInvalidLabel={t("confirmationScreen.markedInvalid")}
                            pointsLabel={(points) =>
                                t("confirmationScreen.points", {count: points})
                            }
                            isDeclineToVotePolicyEnabled={isDeclineToVotePolicyEnabled}
                            declineToVoteLabel={t("confirmationScreen.declineToVote")}
                            isBlankBallotsPolicyEnabled={isBlankBallotsPolicyEnabled}
                            blankBallotLabel={t("confirmationScreen.blankBallot")}
                        />
                    ))}
                </>
            )}
        </>
    )
}

interface IProps {
    confirmationBallot: IConfirmationBallot | null
    ballotId: string
    label?: string
}

export const ConfirmationScreen: React.FC<IProps> = ({confirmationBallot, ballotId}) => {
    const navigate = useNavigate()
    const [isLoading, setIsLoading] = useState(confirmationBallot === null)
    useElectionClassName(confirmationBallot)

    useEffect(() => {
        setIsLoading(confirmationBallot === null)
        if (confirmationBallot == null) {
            navigate("/")
        }
    }, [confirmationBallot])

    return (
        <PageLimit maxWidth="md" className="confirmation-screen screen">
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
                />
            ) : null}
            <ActionButtons />
        </PageLimit>
    )
}
