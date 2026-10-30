// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useEffect, useMemo, useContext} from "react"
import Typography from "@mui/material/Typography"
import Box from "@mui/material/Box"
import {useNavigate} from "react-router-dom"
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
import {getConfirmationContests} from "../services/confirmationContests"
import {getBallotStyleDefaultLanguageCode} from "../services/defaultLanguageCode"
import {sameBallotId} from "../services/vitrine/ballotId"
import {GroupedBallotId} from "../components/Vitrine/GroupedBallotId"

const HorizontalWrap = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 16px;
    margin-bottom: 12px;
`

const IdRow = styled(Box)(({theme}) => ({
    display: "flex",
    alignItems: "center",
    gap: 16,
    marginBottom: 12,
    [theme.breakpoints.down("sm")]: {
        flexDirection: "column",
        alignItems: "stretch",
        gap: 4,
    },
}))

const ID_LABEL_STYLE = {
    flex: "none",
    width: {xs: "auto", sm: 106},
    fontSize: 16,
    lineHeight: 1.5,
}

enum EIdBox {
    INFO = "info",
    ERROR = "error",
}

const IdBox = styled(Box, {shouldForwardProp: (prop) => prop !== "variant"})<{
    variant: EIdBox
}>(({variant}) => ({
    flex: 1,
    minWidth: 0,
    padding: "10px 16px",
    borderRadius: 4,
    background: variant === EIdBox.ERROR ? theme.palette.red.light : theme.palette.blue.light,
    color: variant === EIdBox.ERROR ? theme.palette.red.main : theme.palette.blue.main,
}))

const IdLine = styled(Box)`
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
`

interface BallotIdSectionProps {
    confirmationBallot: IConfirmationBallot | null
    ballotId: string
}

const ballotIdsMatch = (decodedBallotId: string | undefined, ballotId: string): boolean =>
    !!decodedBallotId && sameBallotId(decodedBallotId, ballotId)

interface IBallotIdHelpProps {
    title: string
    ok: string
    content: string
}

const BallotIdHelp: React.FC<IBallotIdHelpProps> = ({title, ok, content}) => {
    const [open, setOpen] = useState(false)

    return (
        <>
            <IconButton
                icon={faCircleQuestion}
                ariaLabel={title}
                sx={{
                    fontSize: "unset",
                    lineHeight: "unset",
                    paddingBottom: "2px",
                    color: "inherit",
                    flex: "none",
                }}
                fontSize="16px"
                onClick={() => setOpen(true)}
            />
            <Dialog
                handleClose={() => setOpen(false)}
                open={open}
                title={title}
                ok={ok}
                variant="info"
            >
                <p>{content}</p>
            </Dialog>
        </>
    )
}

const BallotIdSection: React.FC<BallotIdSectionProps> = ({confirmationBallot, ballotId}) => {
    const {t} = useTranslation()
    const decodedBallotId = confirmationBallot?.ballot_hash
    const matches = ballotIdsMatch(decodedBallotId, ballotId)

    return (
        <>
            <Typography variant="h5" component="h1">
                {t("confirmationScreen.ballotIdTitle")}
            </Typography>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {t("confirmationScreen.ballotIdDescription")}
            </Typography>
            <IdRow>
                <Typography component="p" sx={ID_LABEL_STYLE}>
                    {t("confirmationScreen.decodedBallotId")}
                </Typography>
                <IdBox variant={EIdBox.INFO}>
                    <IdLine>
                        <GroupedBallotId
                            id={decodedBallotId ?? ""}
                            data-testid="decoded-ballot-id"
                        />
                        <BallotIdHelp
                            title={t("confirmationScreen.decodedBallotIdHelpDialog.title")}
                            ok={t("confirmationScreen.decodedBallotIdHelpDialog.ok")}
                            content={t("confirmationScreen.decodedBallotIdHelpDialog.content")}
                        />
                    </IdLine>
                </IdBox>
            </IdRow>
            <IdRow>
                <Typography component="p" sx={ID_LABEL_STYLE}>
                    {t("confirmationScreen.yourBallotId")}
                </Typography>
                <IdBox variant={matches ? EIdBox.INFO : EIdBox.ERROR}>
                    <IdLine>
                        <GroupedBallotId
                            id={ballotId}
                            compareTo={matches ? undefined : decodedBallotId}
                            data-testid="provided-ballot-id"
                        />
                        <BallotIdHelp
                            title={t("confirmationScreen.userBallotIdHelpDialog.title")}
                            ok={t("confirmationScreen.userBallotIdHelpDialog.ok")}
                            content={t("confirmationScreen.userBallotIdHelpDialog.content")}
                        />
                    </IdLine>
                    {matches ? null : (
                        <Box
                            data-testid="ballot-id-mismatch"
                            sx={{
                                display: "flex",
                                alignItems: "center",
                                gap: "6px",
                                marginTop: "6px",
                                fontSize: 12,
                                lineHeight: 1.5,
                            }}
                        >
                            <Icon icon={faTimesCircle} size="sm" />
                            {t("confirmationScreen.ballotIdError")}
                        </Box>
                    )}
                </IdBox>
            </IdRow>
        </>
    )
}

interface ActionButtonProps {}

const ActionButtons: React.FC<ActionButtonProps> = () => {
    const {t} = useTranslation()
    const triggerPrint = () => window.print()

    return (
        <ActionsContainer sx={{marginBottom: "20px", marginTop: "10px"}}>
            <StyledButton href="/" sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}>
                <Icon icon={faAngleLeft} size="sm" />
                <span>{t("confirmationScreen.backButton")}</span>
            </StyledButton>
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
        const sortedContests = sortContestList(
            confirmationBallot?.election_config.contests || [],
            contestsOrderType
        )

        return getConfirmationContests(sortedContests, plaintextVoteQuestions)
    }, [confirmationBallot?.election_config.contests, contestsOrderType, plaintextVoteQuestions])
    const {globalSettings} = useContext(SettingsContext)
    const defaultLanguageCode = getBallotStyleDefaultLanguageCode(
        confirmationBallot?.election_config
    )

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
                <Typography variant="h5" component="h2">
                    {t("confirmationScreen.verifySelectionsTitle")}
                </Typography>
                <IconButton
                    icon={faCircleQuestion}
                    ariaLabel={t("confirmationScreen.verifySelectionsHelpDialog.title")}
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
            ) : confirmationBallot?.election_config.description?.trim() ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {confirmationBallot?.election_config.description}
                </Typography>
            ) : null}
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
                            acclamationDescription={t("confirmationScreen.acclamationDescription")}
                            defaultLanguageCode={defaultLanguageCode}
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
            {ballotIdsMatch(confirmationBallot?.ballot_hash, ballotId) ? (
                <VerifySelectionsSection
                    confirmationBallot={confirmationBallot}
                    isLoading={isLoading}
                />
            ) : null}
            <ActionButtons />
        </PageLimit>
    )
}
