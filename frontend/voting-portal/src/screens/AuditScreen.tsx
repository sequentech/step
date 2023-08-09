// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Box} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    Icon,
    PageLimit,
    BreadCrumbSteps,
    BallotHash,
    Dialog,
    IconButton,
    WarnBox,
    stringToHtml,
    theme,
    isUndefined,
    downloadBlob,
} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import Button from "@mui/material/Button"
import {
    faPrint,
    faAngleRight,
    faCircleQuestion,
    faDownload,
} from "@fortawesome/free-solid-svg-icons"
import {Link as RouterLink, useParams} from "react-router-dom"
import {Typography} from "@mui/material"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectElectionById} from "../store/elections/electionsSlice"
import {selectAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"

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

const AuditableBallotData = styled(Box)`
    word-break: break-word;
    hyphens: auto;
    padding: 15px;
    background-color: #ecfdf5;
    color: #000;
    border-radius: 4px;
    display: block;
    overflow-y: scroll;
    max-height: 200px;
    border: 1px solid #047857;
    margin: 4px 0;
`

const ActionButtons: React.FC = () => {
    const {t} = useTranslation()
    const triggerPrint = () => window.print()

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
            <StyledLink to="/" sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}>
                <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                    <Box>{t("auditScreen.restartButton")}</Box>
                    <Icon icon={faAngleRight} size="sm" />
                </StyledButton>
            </StyledLink>
        </ActionsContainer>
    )
}

export const AuditScreen: React.FC = () => {
    const {electionId} = useParams<{electionId?: string}>()
    const election = useAppSelector(selectElectionById(String(electionId)))
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const {t} = useTranslation()
    const dispatch = useAppDispatch()
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openStep1Help, setOpenStep1Help] = useState(false)
    const [openStep2Help, setOpenStep2Help] = useState(false)

    useEffect(() => {
        if (!isUndefined(electionId) && isUndefined(election)) {
            //dispatch(fetchElectionByIdAsync(String(electionId)))
        }
    }, [electionId, election, dispatch])

    const downloadAuditableBallot = () => {
        if (!auditableBallot) {
            return
        }
        let fileName = `${auditableBallot.config.id}-ballot.json`
        let file = new File([JSON.stringify(auditableBallot)], fileName, {type: "application/json"})
        downloadBlob(file, fileName)
    }

    return (
        <PageLimit maxWidth="lg">
            <BallotHash
                hash={auditableBallot?.ballot_hash || ""}
                onHelpClick={() => setOpenBallotIdHelp(true)}
            />
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
                <BreadCrumbSteps
                    labels={[
                        "breadcrumbSteps.electionList",
                        "breadcrumbSteps.ballot",
                        "breadcrumbSteps.review",
                        "breadcrumbSteps.confirmation",
                        "breadcrumbSteps.audit",
                    ]}
                    selected={4}
                    warning={true}
                />
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

            <AuditableBallotData>
                {(auditableBallot && JSON.stringify(auditableBallot)) || ""}
            </AuditableBallotData>
            <StyledTitle variant="h5" fontWeight="bold" fontSize="18px">
                <Box>{t("auditScreen.step2Title")}</Box>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setOpenStep2Help(true)}
                />
                <Dialog
                    handleClose={() => setOpenStep2Help(false)}
                    open={openStep2Help}
                    title={t("auditScreen.step2HelpDialog.title")}
                    ok={t("auditScreen.step2HelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("auditScreen.step2HelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {stringToHtml(t("auditScreen.step2Description"))}
            </Typography>
            <Box margin="15px 0 25px 0">
                <WarnBox variant="warning">{stringToHtml(t("auditScreen.bottomWarning"))}</WarnBox>
            </Box>
            <ActionButtons />
        </PageLimit>
    )
}
