// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {Alert, Box, CircularProgress} from "@mui/material"
import {useTranslation, Trans} from "react-i18next"
import {
    Icon,
    PageLimit,
    BallotHash,
    Dialog,
    IconButton,
    WarnBox,
    EWarnBoxAnnouncement,
    theme,
    InfoDataBox,
} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    isUndefined,
    downloadBlob,
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    EElectionEventContestEncryptionPolicy,
    EReceiptsPolicy,
    formatVotingPortalDateTime,
} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import Button from "@mui/material/Button"
import {
    faPrint,
    faAngleRight,
    faCircleQuestion,
    faDownload,
} from "@fortawesome/free-solid-svg-icons"
import {Link as RouterLink, useLocation, useNavigate, useParams} from "react-router-dom"
import {Typography} from "@mui/material"
import {useAppSelector} from "../store/hooks"
import {selectBallotForAudit} from "../store/auditableBallots/auditableBallotsSlice"
import {useAuditBallot} from "../hooks/useAuditBallot"
import {useVitrinePolicy} from "../hooks/useVitrinePolicy"
import {useElectionName} from "../hooks/useElectionName"
import {AuditCheck} from "../components/Vitrine/AuditCheck"
import {eventTransparency} from "../services/vitrine/presentation"
import {ballotIdChars} from "../services/vitrine/ballotId"
import {trackingIdOf} from "../services/vitrine/trackingId"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import {ETransparencyMode} from "../services/vitrine/types"
import {provideBallotService} from "../services/BallotService"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {AuthContext} from "../providers/AuthContextProvider"
import {useRootBackLink} from "../hooks/root-back-link"
import StyledLinkContainer from "../components/Link"
import Stepper from "../components/Stepper"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"

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
` as typeof Button

const StyledTitle = styled(Typography)<{component?: React.ElementType}>`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

const Step1Container = styled(Box)`
    display: flex;
    flex-direction: row;
    justify-content: space-between;
`

const ActionButtons: React.FC = () => {
    const {t} = useTranslation()
    const triggerPrint = () => window.print()
    const backLink = useRootBackLink()

    return (
        <ActionsContainer className="actions-container">
            <StyledButton
                className="print-audit-button"
                onClick={triggerPrint}
                variant="secondary"
                sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
            >
                <Icon className="print-audit-icon" icon={faPrint} size="sm" />
                <Box className="print-audit-label">{t("auditScreen.printButton")}</Box>
            </StyledButton>
            <StyledButton
                className="restart-voting-button"
                component={RouterLink}
                to={backLink}
                sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
            >
                <Box className="restart-voting-label">{t("auditScreen.restartButton")}</Box>
                <Icon className="restart-voting-icon" icon={faAngleRight} size="sm" />
            </StyledButton>
        </ActionsContainer>
    )
}

const AuditScreen: React.FC = () => {
    const {tenantId, eventId, electionId} = useParams<{
        tenantId?: string
        eventId: string
        electionId?: string
    }>()
    const {globalSettings} = useContext(SettingsContext)
    const {isKiosk} = useContext(AuthContext)
    const auditableBallot = useAppSelector(selectBallotForAudit(String(electionId)))
    const events = useAppSelector((state) => state.electionEvent)
    const transparency = eventTransparency(
        events[eventId ?? ""]?.presentation,
        auditableBallot?.config.election_event_presentation
    )
    const audit = useAuditBallot(
        String(electionId),
        !globalSettings.DISABLE_AUTH &&
            !auditableBallot?.config.public_key?.is_demo &&
            auditableBallot?.config.election_event_presentation?.receipts?.policy ===
                EReceiptsPolicy.SIGNED_BY_BALLOT_BOX,
        transparency
    )
    const {t, i18n} = useTranslation()
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openStep1Help, setOpenStep1Help] = useState(false)
    const {hashBallot, hashMultiBallot} = provideBallotService()
    const isMultiContest =
        auditableBallot?.config.election_event_presentation?.contest_encryption_policy ==
        EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS
    // Hashing runs during render, before the redirect effect below, so it
    // must not be attempted when there is no ballot to hash - which is the
    // permanent state of this route for a fully acclaimed election.
    const hashedBallot = !auditableBallot
        ? undefined
        : isMultiContest
          ? hashMultiBallot(auditableBallot as IAuditableMultiBallot)
          : hashBallot(auditableBallot as IAuditableSingleBallot)
    const ballotHash = auditableBallot && hashedBallot
    const backLink = useRootBackLink()
    const navigate = useNavigate()
    const location = useLocation()

    if (ballotHash && auditableBallot?.ballot_hash && ballotHash !== auditableBallot.ballot_hash) {
        console.log(
            `ballotId: ${ballotHash}\n auditable Ballot Hash: ${auditableBallot.ballot_hash}`
        )
        throw new VotingPortalError(VotingPortalErrorType.INCONSISTENT_HASH)
    }

    useEffect(() => {
        if (!ballotHash) {
            navigate(backLink)
        }
    })

    const verifierSearchParams = new URLSearchParams(location.search)
    verifierSearchParams.set("lang", i18n.language)
    const verifierHref = `${globalSettings.BALLOT_VERIFIER_URL}tenant/${tenantId}/event/${eventId}/start?${verifierSearchParams.toString()}`

    const downloadAuditableBallot = () => {
        if (!auditableBallot || !audit.ready) {
            return
        }
        let fileName = `${electionId}-ballot.txt`
        let file = new File([JSON.stringify(auditableBallot)], fileName, {type: "text/plain"})
        downloadBlob(file, fileName)
    }

    // The audited ballot's Tracking ID, once the log holds the audit. An event
    // that keeps a transparency log shows the check on its hubs; one that keeps
    // none, the ballot to take to the ballot verifier.
    const auditedTrackingId =
        audit.ready && transparency.mode !== ETransparencyMode.DISABLED
            ? trackingIdOf(ballotHash, auditableBallot?.voter_signing_pk)
            : undefined
    const pinnedPolicy = useVitrinePolicy(transparency)
    const policy = audit.view?.policy ?? pinnedPolicy
    const electionName = useElectionName(electionId)
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const acceptedAt = audit.view?.own?.acceptedAt
    const auditedAt =
        acceptedAt === undefined
            ? undefined
            : formatVotingPortalDateTime(
                  acceptedAt * 1000,
                  electionEvent,
                  i18n.resolvedLanguage || i18n.language
              )

    if (!audit.ready)
        return (
            <PageLimit maxWidth="lg" className="audit-screen screen">
                {audit.error ? (
                    <Alert
                        className="audit-confirmation-error"
                        severity="error"
                        action={
                            <Button
                                className="audit-confirmation-retry-button"
                                onClick={() => void audit.retry()}
                                disabled={audit.loading}
                            >
                                {t("ballotPrivacy.retry")}
                            </Button>
                        }
                    >
                        {t("auditScreen.auditUnavailable")}
                    </Alert>
                ) : (
                    <CircularProgress
                        className="audit-confirmation-loading"
                        aria-label={t("auditScreen.confirmingAudit")}
                    />
                )}
            </PageLimit>
        )

    const ballotIdHelp = (
        <Dialog
            className="audit-ballot-id-help-dialog"
            handleClose={() => setOpenBallotIdHelp(false)}
            open={openBallotIdHelp}
            title={t("reviewScreen.ballotIdHelpDialog.title")}
            ok={t("reviewScreen.ballotIdHelpDialog.ok")}
            cancel={t("reviewScreen.ballotIdHelpDialog.cancel")}
            variant="info"
        >
            {stringToHtml(t("reviewScreen.ballotIdHelpDialog.content"))}
        </Dialog>
    )

    if (auditedTrackingId)
        return (
            <PageLimit maxWidth="lg" className="audit-screen screen">
                <AuditCheck
                    key={policy?.id ?? ""}
                    trackingId={auditedTrackingId}
                    tenantId={tenantId ?? ""}
                    eventId={eventId ?? ""}
                    electionId={electionId ?? ""}
                    electionName={electionName}
                    view={audit.view}
                    publication={audit.publication}
                    onPublishRetry={() => void audit.retryPublish()}
                    policy={policy}
                    receivedAt={audit.receivedAt}
                    auditedAt={auditedAt}
                    ballotVerifierUrl={globalSettings.BALLOT_VERIFIER_URL || undefined}
                    trackerUrl={`/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${ballotIdChars(auditedTrackingId)}${isKiosk() ? "?kiosk" : ""}`}
                    voteAgainTo={backLink}
                    steps={<Stepper selected={2} />}
                    onTitleHelp={() => setOpenStep1Help(true)}
                    onBallotIdHelp={() => setOpenBallotIdHelp(true)}
                />
                {ballotIdHelp}
                <Dialog
                    className="screen-help-dialog audit-help-dialog"
                    handleClose={() => setOpenStep1Help(false)}
                    open={openStep1Help}
                    title={t("auditScreen.check.helpDialog.title")}
                    ok={t("auditScreen.check.helpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("auditScreen.check.helpDialog.content"))}
                </Dialog>
            </PageLimit>
        )

    return (
        <PageLimit maxWidth="lg" className="audit-screen screen">
            <BallotHash hash={ballotHash || ""} onHelpClick={() => setOpenBallotIdHelp(true)} />
            <Box className="stepper-box" marginTop="24px">
                {ballotIdHelp}
                <Stepper selected={4} warning={true} />
            </Box>
            <StyledTitle className="screen-title" variant="h4" component="h1" fontSize="1.5rem">
                <Box className="screen-title-text">{t("auditScreen.title")}</Box>
                <IconButton
                    buttonClassName="screen-help-button"
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="1rem"
                    onClick={() => setOpenStep1Help(true)}
                    ariaLabel={t("a11y.helpAbout", {topic: t("auditScreen.title")})}
                />
                <Dialog
                    className="screen-help-dialog audit-help-dialog"
                    handleClose={() => setOpenStep1Help(false)}
                    open={openStep1Help}
                    title={t("auditScreen.step1HelpDialog.title")}
                    ok={t("auditScreen.step1HelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("auditScreen.step1HelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            <Typography
                className="screen-description"
                variant="body2"
                component="div"
                sx={{color: theme.palette.customGrey.main}}
            >
                {stringToHtml(t("auditScreen.description"))}
            </Typography>
            <StyledTitle
                className="audit-download-title"
                variant="h5"
                component="h2"
                fontWeight="bold"
                fontSize="1.125rem"
            >
                <Box className="audit-download-title-text">{t("auditScreen.step1Title")}</Box>
                <IconButton
                    buttonClassName="audit-download-help-button"
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="1rem"
                    onClick={() => setOpenStep1Help(true)}
                    ariaLabel={t("a11y.helpAbout", {topic: t("auditScreen.step1Title")})}
                />
                <Dialog
                    className="audit-download-help-dialog"
                    handleClose={() => setOpenStep1Help(false)}
                    open={openStep1Help}
                    title={t("auditScreen.step1HelpDialog.title")}
                    ok={t("auditScreen.step1HelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("auditScreen.step1HelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            <Step1Container className="audit-download-container">
                <Typography
                    className="audit-download-description"
                    variant="body2"
                    component="div"
                    sx={{color: theme.palette.customGrey.main}}
                >
                    {stringToHtml(t("auditScreen.step1Description"))}
                </Typography>
                <StyledButton
                    className="download-auditable-ballot-button"
                    aria-label={t("auditScreen.downloadButton")}
                    sx={{minWidth: "unset", padding: "10px 16px"}}
                    onClick={downloadAuditableBallot}
                    disabled={isUndefined(auditableBallot)}
                >
                    <Icon className="download-auditable-ballot-icon" icon={faDownload} size="sm" />
                    <Box
                        className="download-auditable-ballot-label"
                        sx={{display: {xs: "none", md: "flex"}}}
                    >
                        {t("auditScreen.downloadButton")}
                    </Box>
                </StyledButton>
            </Step1Container>

            <InfoDataBox className="auditable-ballot-data">
                {(auditableBallot && JSON.stringify(auditableBallot)) || ""}
            </InfoDataBox>
            <StyledTitle
                className="audit-verification-title"
                variant="h5"
                component="h2"
                fontWeight="bold"
                fontSize="1.125rem"
            >
                <Box className="audit-verification-title-text">{t("auditScreen.step2Title")}</Box>
            </StyledTitle>
            <Typography
                className="audit-verification-description"
                variant="body2"
                component="div"
                sx={{color: theme.palette.customGrey.main}}
            >
                <StyledLinkContainer className="audit-verification-links">
                    <Trans
                        i18nKey="auditScreen.step2Description"
                        components={{
                            VerifierLink: (
                                <a
                                    className="ballot-verifier-link"
                                    target="_blank"
                                    href={verifierHref}
                                />
                            ),
                        }}
                    />
                </StyledLinkContainer>
            </Typography>
            <Box className="audit-warning-container" margin="15px 0 25px 0">
                {/* Static advice that is part of the page, not a response to
                    anything the voter did, so it is read in document order. */}
                <WarnBox
                    className="audit-warning"
                    variant="warning"
                    announcement={EWarnBoxAnnouncement.SILENT}
                >
                    {stringToHtml(t("auditScreen.bottomWarning"))}
                </WarnBox>
            </Box>
            <ActionButtons />
        </PageLimit>
    )
}

export default AuditScreen
