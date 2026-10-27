// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {SettingsContext} from "../providers/SettingsContextProvider"
import React, {useState, useEffect, useContext, useMemo} from "react"
import Typography from "@mui/material/Typography"
import Button from "@mui/material/Button"
import {Link as RouterLink} from "react-router-dom"
import {styled} from "@mui/material/styles"
import Alert from "@mui/material/Alert"
import AlertTitle from "@mui/material/AlertTitle"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    PageLimit,
    DropFile,
    BreadCrumbSteps,
    Icon,
    IconButton,
    theme,
    Dialog,
} from "@sequentech/ui-essentials"
import {IAuditableBallot, IAuditableMultiBallot, IAuditableSingleBallot} from "@sequentech/ui-core"
import {useNavigate} from "react-router-dom"
import {Box} from "@mui/material"
import {IBallotService, IConfirmationBallot} from "../services/BallotService"
import {
    EBallotCiphertextCheck,
    EBallotEncoding,
    checkAuditableBallotCiphertext,
} from "../services/ballotCiphertextVerification"
import TextField from "@mui/material/TextField"
import {faCircleQuestion, faAngleRight} from "@fortawesome/free-solid-svg-icons"
import JsonImg from "../public/json.png"
import Image from "mui-image"
import {TenantEventContext} from "../providers/TenantEventContext"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {
    EBallotStyleCheck,
    LEGACY_BALLOT_VERSION,
    checkAuditedBallotStyle,
} from "../services/ballotStyleBinding"
import {useAppDispatch} from "../store/hooks"
import {
    EPublishedBallotStyleLookup,
    GetPublishedBallotStylesQuery,
    PublishedBallotStyleLookup,
    findPublishedBallotStyle,
    updateBallotStyleAndSelection,
} from "../services/BallotStyles"

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

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
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

const StyledImage = styled(Image)`
    max-height: 64px;
    max-width: 64px;
`

const FileWrapper = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 10px;
    align-items: center;
`

interface JsonFileProps {
    name: string
}

const SampleFileName = "sample.json"

const JsonFile: React.FC<JsonFileProps> = ({name}) => {
    const {t} = useTranslation()

    return (
        <>
            <Typography
                variant="body2"
                sx={{textAlign: "center", lineHeight: "36px", color: theme.palette.customGrey.main}}
            >
                {t("homeScreen.fileUploaded")}
            </Typography>
            <FileWrapper>
                <StyledImage src={JsonImg} duration={100} width="unset" />
                <Typography variant="body2" sx={{color: theme.palette.black}}>
                    {name || SampleFileName}
                </Typography>
            </FileWrapper>
        </>
    )
}

interface IProps {
    confirmationBallot: IConfirmationBallot | null
    setConfirmationBallot: (confirmation: IConfirmationBallot | null) => void
    ballotId: string
    setBallotId: (ballotId: string) => void
    fileName: string
    setFileName: (ballotId: string) => void
    ballotService: IBallotService
    label?: string
}

const parseAuditableBallotFile = async (
    file: File,
    ballotService: IBallotService
): Promise<string | null> => {
    try {
        let auditableBallotString = await file.text()
        return auditableBallotString
    } catch (e) {
        console.log(e)
        return null
    }
}

export const HomeScreen: React.FC<IProps> = ({
    confirmationBallot,
    setConfirmationBallot,
    ballotId,
    setBallotId,
    fileName,
    setFileName,
    ballotService,
}) => {
    const {t} = useTranslation()
    const [showError, setShowError] = useState(false)
    const [showCiphertextError, setShowCiphertextError] = useState(false)
    const [showStyleError, setShowStyleError] = useState(false)
    const [legacyWarning, setLegacyWarning] = useState<string | null>(null)
    const [styleLookupFailure, setStyleLookupFailure] =
        useState<EPublishedBallotStyleLookup | null>(null)
    const [openStep1Help, setOpenStep1Help] = useState(false)
    const [openStep2Help, setOpenStep2Help] = useState(false)
    const [isNextActive, setNextActive] = useState(false)
    const navigate = useNavigate()
    const {tenantId, eventId} = useContext(TenantEventContext)
    const {globalSettings} = useContext(SettingsContext)
    const {data: dataBallotStyles} = useQuery<GetPublishedBallotStylesQuery>(GET_BALLOT_STYLES, {
        skip: globalSettings.DISABLE_AUTH,
    })
    const dispatch = useAppDispatch()

    useEffect(() => {
        const newIsNextActive = !!confirmationBallot && !!ballotId
        if (newIsNextActive !== isNextActive) {
            setNextActive(newIsNextActive)
        }
    }, [confirmationBallot, ballotId, isNextActive])

    useMemo(() => {
        if (dataBallotStyles && dataBallotStyles.sequent_backend_ballot_style.length > 0) {
            updateBallotStyleAndSelection(dataBallotStyles, dispatch)
        }
    }, [dataBallotStyles])

    const rejectBallot = (
        showImportError: boolean,
        failure: EPublishedBallotStyleLookup | null = null
    ) => {
        setShowError(showImportError)
        setShowCiphertextError(false)
        setStyleLookupFailure(failure)
        setShowStyleError(false)
        setLegacyWarning(null)
        setConfirmationBallot(null)
    }

    const handleAuditableBallot = (
        file: IAuditableBallot | null,
        ballotStyleLookup: PublishedBallotStyleLookup,
        isPublishedBallot: boolean = true
    ) => {
        if (!file?.config?.id) {
            rejectBallot(true)
            return
        }
        if (EPublishedBallotStyleLookup.FOUND !== ballotStyleLookup.status) {
            rejectBallot(false, ballotStyleLookup.status)
            return
        }
        const {ballotStyle} = ballotStyleLookup
        const mustMatchPublishedStyle = isPublishedBallot && !globalSettings.DISABLE_AUTH
        if (
            mustMatchPublishedStyle &&
            EBallotStyleCheck.MATCHES !== checkAuditedBallotStyle(file.config, [ballotStyle])
        ) {
            rejectBallot(false)
            setShowStyleError(true)
            return
        }
        // The ballot is decoded and shown with the published ballot style,
        // never with the copy of it that the file carries.
        const auditableBallot: IAuditableBallot = {...file, config: ballotStyle}
        const contests = (file as IAuditableSingleBallot | IAuditableMultiBallot).contests
        const isMultiContest = typeof contests === "string"
        if (!isMultiContest && !Array.isArray(contests)) {
            rejectBallot(true)
            return
        }
        let decodedBallot = null
        try {
            decodedBallot = isMultiContest
                ? ballotService.decodeAuditableMultiBallot(auditableBallot as IAuditableMultiBallot)
                : ballotService.decodeAuditableBallot(auditableBallot as IAuditableSingleBallot)
        } catch (error) {
            console.log(error)
            rejectBallot(true)
            return
        }
        if (decodedBallot == null) {
            rejectBallot(true)
            return
        }
        // Decoding only reads the plaintext. The ballot is not verified until the
        // plaintext and randomness are shown to reproduce the ciphertext.
        const encoding = isMultiContest
            ? EBallotEncoding.MULTI_CONTEST
            : EBallotEncoding.SINGLE_CONTEST
        const ciphertextCheck = checkAuditableBallotCiphertext(
            ballotService,
            auditableBallot,
            encoding
        )
        if (EBallotCiphertextCheck.VERIFIED !== ciphertextCheck) {
            // Only a ciphertext that fails to reproduce is a failed
            // verification. A ballot that could not be checked at all is a
            // problem with the file, and reports the generic import error.
            const isMismatch = EBallotCiphertextCheck.MISMATCH === ciphertextCheck
            rejectBallot(!isMismatch)
            setShowCiphertextError(isMismatch)
            return
        }
        setLegacyWarning(
            !isPublishedBallot
                ? null
                : LEGACY_BALLOT_VERSION === auditableBallot.version
                  ? "legacyBallotWarning"
                  : auditableBallot.version === 3
                    ? "historicalTrackerWarning"
                    : null
        )
        // Signatures and trackers cover the uploaded configuration; the published
        // presentation is used for decoding and display after the layout matches.
        let ballotHash = isMultiContest
            ? ballotService.hashMultiBallot(file as IAuditableMultiBallot)
            : ballotService.hashBallot512(file as IAuditableSingleBallot)

        // sequent-core also rejects a signature without its public key, and the reverse.
        if (
            auditableBallot?.voter_ballot_signature != null ||
            auditableBallot?.voter_signing_pk != null
        ) {
            try {
                const signatureVerified = isMultiContest
                    ? ballotService.verifyMultiBallotSignature(
                          ballotHash,
                          ballotStyle.election_id,
                          file as IAuditableMultiBallot
                      )
                    : ballotService.verifyBallotSignature(
                          ballotHash,
                          ballotStyle.election_id,
                          file as IAuditableSingleBallot
                      )
                if (signatureVerified !== true) {
                    rejectBallot(true)
                    return
                }
            } catch (error) {
                console.log(error)
                rejectBallot(true)
                return
            }
        }

        setConfirmationBallot({
            ballot_hash: ballotHash,
            election_config: ballotStyle,
            decoded_questions: decodedBallot,
        })
        setShowError(false)
        setShowCiphertextError(false)
        setStyleLookupFailure(null)
        setShowStyleError(false)
    }

    const handleFiles = async (files: FileList) => {
        try {
            setFileName(files[0].name)
            const auditableBallotString = await parseAuditableBallotFile(files[0], ballotService)
            if (auditableBallotString) {
                const auditableBallot: IAuditableBallot | null = JSON.parse(auditableBallotString)
                handleAuditableBallot(
                    auditableBallot,
                    globalSettings.DISABLE_AUTH && auditableBallot
                        ? // Without authentication the published ballot styles cannot be
                          // fetched, so there is nothing else to check the ballot against.
                          {
                              status: EPublishedBallotStyleLookup.FOUND,
                              ballotStyle: auditableBallot.config,
                          }
                        : findPublishedBallotStyle(
                              dataBallotStyles,
                              auditableBallot?.config?.id,
                              eventId
                          )
                )
            }
        } catch (e) {
            rejectBallot(true)
        }
    }

    // use sample ballot
    const onUseSampleClick = () => {
        setFileName(SampleFileName)
        let auditableBallot = ballotService.generateSampleAuditableBallot()
        if (!auditableBallot) {
            return
        }
        // The sample is generated here rather than uploaded, so the ballot
        // style it carries is the one to check it against.
        handleAuditableBallot(
            auditableBallot,
            {
                status: EPublishedBallotStyleLookup.FOUND,
                ballotStyle: auditableBallot.config,
            },
            false
        )
        let ballotHash = ballotService.hashBallot512(auditableBallot)
        setBallotId(ballotHash)
    }

    const onInputChange: React.ChangeEventHandler<HTMLInputElement> = (e) =>
        setBallotId(e.target.value)

    return (
        <PageLimit maxWidth="md" className="home-screen screen">
            <Box marginTop="48px">
                <BreadCrumbSteps
                    labels={[
                        "breadcrumbSteps.import",
                        "breadcrumbSteps.verify",
                        //"breadcrumbSteps.finish",
                    ]}
                    selected={0}
                />
            </Box>
            <StyledTitle variant="h5">
                <span>{t("homeScreen.step1")}</span>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setOpenStep1Help(true)}
                />
                <Dialog
                    handleClose={() => setOpenStep1Help(false)}
                    open={openStep1Help}
                    title={t("homeScreen.importBallotHelpDialog.title")}
                    ok={t("homeScreen.importBallotHelpDialog.ok")}
                    variant="info"
                >
                    <p>{t("homeScreen.importBallotHelpDialog.content")}</p>
                </Dialog>
            </StyledTitle>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {t("homeScreen.description1")}
            </Typography>
            <Alert
                severity="error"
                style={{display: showError ? undefined : "none"}}
                data-testid="import-error"
            >
                <AlertTitle>{t("homeScreen.importErrorTitle")}</AlertTitle>
                <Typography variant="body2">{t("homeScreen.importErrorDescription")}</Typography>
                <RouterLink
                    to="//sequentech.github.io/documentation/docs/general/reference/ballot-encoding"
                    target="_blank"
                    rel="nofollow"
                >
                    {t("homeScreen.importErrorMoreInfo")}
                </RouterLink>
            </Alert>
            <Alert
                severity="error"
                style={{display: showCiphertextError ? undefined : "none"}}
                data-testid="ciphertext-error"
            >
                <AlertTitle>{t("homeScreen.ciphertextErrorTitle")}</AlertTitle>
                <Typography variant="body2">
                    {t("homeScreen.ciphertextErrorDescription")}
                </Typography>
            </Alert>
            <Alert
                severity="error"
                style={{display: showStyleError ? undefined : "none"}}
                data-testid="style-error"
            >
                <AlertTitle>{t("homeScreen.verificationErrorTitle")}</AlertTitle>
                <Typography variant="body2">{t("homeScreen.styleErrorDescription")}</Typography>
            </Alert>
            <Alert
                severity="warning"
                style={{display: legacyWarning ? undefined : "none"}}
                data-testid="legacy-ballot-warning"
            >
                <Typography variant="body2">
                    {legacyWarning ? t(`homeScreen.${legacyWarning}`) : null}
                </Typography>
            </Alert>
            <Alert
                severity="error"
                style={{
                    display:
                        EPublishedBallotStyleLookup.NOT_PUBLISHED === styleLookupFailure
                            ? undefined
                            : "none",
                }}
                data-testid="unpublished-style-error"
            >
                <AlertTitle>{t("homeScreen.verificationErrorTitle")}</AlertTitle>
                <Typography variant="body2">
                    {t("homeScreen.unpublishedStyleErrorDescription")}
                </Typography>
            </Alert>
            <Alert
                severity="error"
                style={{
                    display:
                        null !== styleLookupFailure &&
                        EPublishedBallotStyleLookup.NOT_PUBLISHED !== styleLookupFailure
                            ? undefined
                            : "none",
                }}
                data-testid="unavailable-style-error"
            >
                <AlertTitle>{t("homeScreen.verificationErrorTitle")}</AlertTitle>
                <Typography variant="body2">
                    {t("homeScreen.unavailableStyleErrorDescription")}
                </Typography>
            </Alert>
            <DropFile handleFiles={handleFiles} />
            {confirmationBallot ? <JsonFile name={fileName} /> : null}
            <StyledTitle variant="h5">
                <span>{t("homeScreen.step2")}</span>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setOpenStep2Help(true)}
                />
                <Dialog
                    handleClose={() => setOpenStep2Help(false)}
                    open={openStep2Help}
                    title={t("homeScreen.ballotIdHelpDialog.title")}
                    ok={t("homeScreen.ballotIdHelpDialog.ok")}
                    variant="info"
                >
                    <p>{t("homeScreen.ballotIdHelpDialog.content")}</p>
                </Dialog>
            </StyledTitle>
            <Typography
                variant="body2"
                sx={{color: theme.palette.customGrey.main, marginBottom: 0}}
            >
                {t("homeScreen.description2")}
            </Typography>
            <Box maxWidth="630px">
                <TextField
                    label={t("homeScreen.ballotIdLabel")}
                    placeholder={t("homeScreen.ballotIdPlaceholder")}
                    InputLabelProps={{shrink: true}}
                    value={ballotId}
                    onChange={onInputChange}
                />
            </Box>
            <ActionsContainer>
                <StyledButton sx={{width: {xs: "100%", sm: "200px"}}} onClick={onUseSampleClick}>
                    <span>{t("homeScreen.useSampleLink")}</span>
                </StyledButton>
                <StyledButton
                    sx={{width: {xs: "100%", sm: "200px"}}}
                    disabled={!isNextActive}
                    onClick={() =>
                        isNextActive &&
                        navigate(`/tenant/${tenantId}/event/${eventId}/confirmation`)
                    }
                >
                    <span>{t("homeScreen.nextButton")}</span>
                    <Icon icon={faAngleRight} size="sm" />
                </StyledButton>
            </ActionsContainer>
        </PageLimit>
    )
}
