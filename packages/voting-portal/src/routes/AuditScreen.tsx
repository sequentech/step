// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {Box} from "@mui/material"
import {useTranslation, Trans} from "react-i18next"
import {
    Icon,
    PageLimit,
    BallotHash,
    Dialog,
    IconButton,
    WarnBox,
    theme,
    InfoDataBox,
} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    isUndefined,
    downloadBlob,
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    IAuditablePlaintextBallot,
    EElectionEventContestEncryptionPolicy,
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
import {selectAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {provideBallotService} from "../services/BallotService"
import {SettingsContext} from "../providers/SettingsContextProvider"
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
`

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

const StyledLink = styled(RouterLink)`
    margin: auto 0;
    text-decoration: none;
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
        <ActionsContainer>
            <StyledButton
                onClick={triggerPrint}
                variant="secondary"
                sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
            >
                <Icon icon={faPrint} size="sm" />
                <Box>{t("auditScreen.printButton")}</Box>
            </StyledButton>
            <StyledLink to={backLink} sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}>
                <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                    <Box>{t("auditScreen.restartButton")}</Box>
                    <Icon icon={faAngleRight} size="sm" />
                </StyledButton>
            </StyledLink>
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
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const {t} = useTranslation()
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openStep1Help, setOpenStep1Help] = useState(false)
    const {hashBallot, hashMultiBallot, hashPlaintextBallot} = provideBallotService()

    const encryptionPolicy =
        auditableBallot?.config.election_event_presentation?.contest_encryption_policy
    const hashedBallot = (function () {
        switch (encryptionPolicy) {
            case EElectionEventContestEncryptionPolicy.SINGLE_CONTEST:
                hashBallot(auditableBallot as IAuditableSingleBallot)
                break
            case EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS:
                hashMultiBallot(auditableBallot as IAuditableMultiBallot)
                break
            case EElectionEventContestEncryptionPolicy.PLAINTEXT:
                hashPlaintextBallot(auditableBallot as IAuditablePlaintextBallot)
                break
            default:
                // TODO New VotingPortalError?
                throw new VotingPortalError(VotingPortalErrorType.INCONSISTENT_HASH)
        }
    })()

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

    const downloadAuditableBallot = () => {
        if (!auditableBallot) {
            return
        }
        let fileName = `${electionId}-ballot.txt`
        let file = new File([JSON.stringify(auditableBallot)], fileName, {type: "text/plain"})
        downloadBlob(file, fileName)
    }

    return (
        <PageLimit maxWidth="lg" className="audit-screen screen">
            <BallotHash hash={ballotHash || ""} onHelpClick={() => setOpenBallotIdHelp(true)} />
            <Box marginTop="24px">
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
                <Stepper selected={4} warning={true} />
            </Box>
            <StyledTitle variant="h4" fontSize="24px">
                <Box>{t("auditScreen.title")}</Box>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setOpenStep1Help(true)}
                />
                <Dialog
                    handleClose={() => setOpenStep1Help(false)}
                    open={openStep1Help}
                    title={t("auditScreen.step1HelpDialog.title")}
                    ok={t("auditScreen.step1HelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("auditScreen.step1HelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {stringToHtml(t("auditScreen.description"))}
            </Typography>
            <StyledTitle variant="h5" fontWeight="bold" fontSize="18px">
                <Box>{t("auditScreen.step1Title")}</Box>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setOpenStep1Help(true)}
                />
                <Dialog
                    handleClose={() => setOpenStep1Help(false)}
                    open={openStep1Help}
                    title={t("auditScreen.step1HelpDialog.title")}
                    ok={t("auditScreen.step1HelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("auditScreen.step1HelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            <Step1Container>
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(t("auditScreen.step1Description"))}
                </Typography>
                <StyledButton
                    sx={{minWidth: "unset", padding: "10px 16px"}}
                    onClick={downloadAuditableBallot}
                    disabled={isUndefined(auditableBallot)}
                >
                    <Icon icon={faDownload} size="sm" />
                    <Box sx={{display: {xs: "none", md: "flex"}}}>
                        {t("auditScreen.downloadButton")}
                    </Box>
                </StyledButton>
            </Step1Container>

            <InfoDataBox>{(auditableBallot && JSON.stringify(auditableBallot)) || ""}</InfoDataBox>
            <StyledTitle variant="h5" fontWeight="bold" fontSize="18px">
                <Box>{t("auditScreen.step2Title")}</Box>
            </StyledTitle>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                <StyledLinkContainer>
                    <Trans
                        i18nKey="auditScreen.step2Description"
                        components={{
                            VerifierLink: (
                                <a
                                    target="_blank"
                                    href={`${globalSettings.BALLOT_VERIFIER_URL}tenant/${tenantId}/event/${eventId}/start${location.search}`}
                                />
                            ),
                        }}
                    />
                </StyledLinkContainer>
            </Typography>
            <Box margin="15px 0 25px 0">
                <WarnBox variant="warning">{stringToHtml(t("auditScreen.bottomWarning"))}</WarnBox>
            </Box>
            <ActionButtons />
        </PageLimit>
    )
}

export default AuditScreen
