// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useContext, useEffect, useId, useReducer, useRef} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Checkbox,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    FormControlLabel,
    IconButton,
    InputAdornment,
    List,
    ListItem,
    ListItemIcon,
    ListItemText,
    Paper,
    Stack,
    Step,
    StepLabel,
    Stepper,
    TextField,
    Typography,
} from "@mui/material"
import CheckCircleIcon from "@mui/icons-material/CheckCircle"
import DrawIcon from "@mui/icons-material/Draw"
import ErrorIcon from "@mui/icons-material/Error"
import InsertDriveFileOutlinedIcon from "@mui/icons-material/InsertDriveFileOutlined"
import LockOutlinedIcon from "@mui/icons-material/LockOutlined"
import VisibilityIcon from "@mui/icons-material/Visibility"
import VisibilityOffIcon from "@mui/icons-material/VisibilityOff"
import {AuthContext} from "@/providers/AuthContextProvider"
import {TenantContext} from "@/providers/TenantContextProvider"
import {
    SigningApiError,
    SigningApiErrorKind,
    validatePanel,
    type ISigningApi,
    type ISigningPanelData,
} from "@/lib/signing/api"
import type {OpenedCertificate} from "@/lib/signing/certificate"
import {colonHex} from "@/lib/signing/der"
import {
    CertificateFileError,
    CertificateFileErrorCode,
    openFailureReason,
} from "@/lib/signing/errors"
import {
    PayloadMismatchError,
    PayloadProblem,
    commonNameOf,
    documentKindOf,
    isOpenForSigning,
    pendingSigners,
    signerName,
    signerOf,
} from "@/lib/signing/request"
import {
    CertificateCheckId,
    DocumentKind,
    SigningAction,
    SigningRequestStatus,
    type IApproveSigningRequestOutput,
    type ICertificateCheckResult,
} from "@/lib/signing/types"
import {SigningSubject, SigningSubjectVariant} from "./SigningSubject"
import {useSignedView} from "./useSignedView"
import {
    actionDescription,
    actionObject,
    formatList,
    organizationName,
    requestTitle,
    shortFingerprint,
    useSigningFormat,
} from "./format"

// The crypto libraries (about 160 KB gzip) load when a dialog first opens.
let cryptoModule: Promise<typeof import("@/lib/signing/sign")> | undefined
const loadCrypto = () => {
    cryptoModule ??= import("@/lib/signing/sign").catch((error: unknown) => {
        cryptoModule = undefined
        throw error
    })
    return cryptoModule
}

export enum SigningDialogStep {
    Check = 0,
    Certificate = 1,
    Signed = 2,
}

enum CertificatePhase {
    Choose = "choose",
    Opening = "opening",
    Checking = "checking",
    Checked = "checked",
    Signing = "signing",
}

/** A request that stopped waiting, or changed, while the dialog was open. */
interface IClosed {
    /** Translation key. */
    message: string
    /** For a cancellation: the translation key of its reason. */
    reason?: string
}

interface IState {
    step: SigningDialogStep
    confirmed: boolean
    file: File | null
    password: string
    showPassword: boolean
    phase: CertificatePhase
    certificate: OpenedCertificate | null
    /** Every key of the file with its certificate, best first. */
    choices: OpenedCertificate[]
    /** The best two are equally good: the person picks one. */
    ambiguous: boolean
    checks: ICertificateCheckResult[] | null
    fileError: CertificateFileErrorCode | null
    /** Why the request can't be signed any more; Sign stays disabled. */
    closed: IClosed | null
    /** Translation key of the error shown above the buttons. */
    error: string | null
    signed: {at: Date; result: IApproveSigningRequestOutput} | null
}

type TAction =
    | {type: "reset"}
    | {type: "confirm"; confirmed: boolean}
    | {type: "goto"; step: SigningDialogStep}
    | {type: "file"; file: File | null}
    | {type: "password"; password: string}
    | {type: "togglePassword"}
    | {type: "opening"}
    | {type: "openFailed"; code: CertificateFileErrorCode}
    | {type: "openError"}
    | {type: "opened"; choices: OpenedCertificate[]; ambiguous: boolean}
    | {type: "choose"; index: number}
    | {type: "closed"; closed: IClosed}
    | {type: "checking"}
    | {type: "checked"; checks: ICertificateCheckResult[]}
    | {type: "checkFailed"}
    | {type: "signing"}
    | {type: "signFailed"; error: string; refused?: CertificateCheckId | null}
    | {type: "signed"; result: IApproveSigningRequestOutput; at: Date}

const INITIAL: IState = {
    step: SigningDialogStep.Check,
    confirmed: false,
    file: null,
    password: "",
    showPassword: false,
    phase: CertificatePhase.Choose,
    certificate: null,
    choices: [],
    ambiguous: false,
    checks: null,
    fileError: null,
    closed: null,
    error: null,
    signed: null,
}

const reducer = (state: IState, action: TAction): IState => {
    switch (action.type) {
        case "reset":
            return INITIAL
        case "confirm":
            return {...state, confirmed: action.confirmed}
        case "goto":
            return {...state, step: action.step, error: null}
        case "file":
            // A new file starts over: its password, key and checks are another's.
            return {
                ...state,
                file: action.file,
                password: "",
                phase: CertificatePhase.Choose,
                certificate: null,
                choices: [],
                ambiguous: false,
                checks: null,
                fileError: null,
                error: null,
            }
        case "password":
            return {...state, password: action.password, fileError: null}
        case "togglePassword":
            return {...state, showPassword: !state.showPassword}
        case "opening":
            return {...state, phase: CertificatePhase.Opening, fileError: null, error: null}
        case "openFailed":
            return {...state, phase: CertificatePhase.Choose, fileError: action.code}
        case "openError":
            return {...state, phase: CertificatePhase.Choose, error: "signing.widget.openError"}
        case "opened":
            // The keys are open; the password is no longer needed.
            return {
                ...state,
                certificate: action.choices[0],
                choices: action.choices,
                ambiguous: action.ambiguous,
                password: "",
                showPassword: false,
            }
        case "choose":
            return {...state, certificate: state.choices[action.index] ?? state.certificate}
        case "closed":
            return {...state, phase: CertificatePhase.Checked, closed: action.closed, error: null}
        case "checking":
            return {...state, phase: CertificatePhase.Checking, checks: null, error: null}
        case "checked":
            return {...state, phase: CertificatePhase.Checked, checks: action.checks}
        case "checkFailed":
            return {...state, phase: CertificatePhase.Checked, error: "signing.widget.checkError"}
        case "signing":
            return {...state, phase: CertificatePhase.Signing, error: null}
        case "signFailed": {
            const refused = action.refused
            const checks =
                refused && state.checks
                    ? state.checks.some((check) => check.id === refused)
                        ? state.checks.map((check) =>
                              check.id === refused ? {...check, ok: false} : check
                          )
                        : [...state.checks, {id: refused, ok: false, detail: null}]
                    : state.checks
            return {...state, phase: CertificatePhase.Checked, checks, error: action.error}
        }
        case "signed":
            return {
                ...state,
                step: SigningDialogStep.Signed,
                phase: CertificatePhase.Checked,
                signed: {at: action.at, result: action.result},
            }
    }
}

const allPassed = (checks: ICertificateCheckResult[] | null): boolean =>
    !!checks && checks.length > 0 && checks.every((check) => check.ok)

const failed = (checks: ICertificateCheckResult[] | null, id: CertificateCheckId) =>
    !!checks?.some((check) => check.id === id && !check.ok)

/**
 * Why a freshly fetched request can no longer be signed as the dialog shows
 * it, or `null` while it still waits unchanged.
 */
const closedOf = (fresh: ISigningPanelData, shown: ISigningPanelData): IClosed | null => {
    const {request} = fresh
    if (
        request.payload_sha256 !== shown.request.payload_sha256 ||
        request.code !== shown.request.code
    ) {
        return {message: "signing.widget.closed.changed"}
    }
    switch (request.status) {
        case SigningRequestStatus.Cancelled:
            return {
                message: "signing.dialog.problems.cancelled",
                reason: request.cancel_reason
                    ? `signing.cancelReasons.${request.cancel_reason}`
                    : "signing.status.cancelled",
            }
        case SigningRequestStatus.Expired:
            return {message: "signing.widget.panel.expired"}
        case SigningRequestStatus.Completed:
        case SigningRequestStatus.Executed:
            return {message: "signing.widget.closed.allSigned"}
        case SigningRequestStatus.Failed:
            return {message: "signing.widget.panel.failed"}
        default:
            return isOpenForSigning(request) ? null : {message: "signing.widget.panel.expired"}
    }
}

const signErrorKey = (error: unknown): {error: string; refused?: CertificateCheckId | null} => {
    if (error instanceof SigningApiError) {
        if (error.kind === SigningApiErrorKind.Refused) {
            return {error: "signing.widget.refused", refused: error.check}
        }
        if (error.kind === SigningApiErrorKind.Stale) {
            return {error: "signing.widget.stale"}
        }
    }
    if (error instanceof PayloadMismatchError) {
        return {
            error:
                error.problem === PayloadProblem.Document
                    ? "signing.widget.documentMismatch"
                    : "signing.widget.mismatch",
        }
    }
    return {error: "signing.widget.signError"}
}

/** "Signing happens in this browser…": on every step (design §9). */
const LocalSigningNote: React.FC = () => {
    const {t} = useTranslation()
    return (
        <Alert severity="info" icon={<LockOutlinedIcon fontSize="inherit" />} sx={{mt: 3}}>
            {t("signing.dialog.localNote")}
        </Alert>
    )
}

/**
 * The Post the signer signs for: the request's, where its signers are the
 * Post's own (an SBEI signs for their Post). A voter's approval is the voter's
 * Post, not the signer's, which the panel doesn't know: none.
 */
const signerPost = (data: ISigningPanelData): string | null | undefined =>
    data.request.action === SigningAction.ApproveVoter ? null : data.election_name

const CheckStep: React.FC<{
    data: ISigningPanelData
    api: ISigningApi
    confirmed: boolean
    onConfirm: (confirmed: boolean) => void
}> = ({data, api, confirmed, onConfirm}) => {
    const {t} = useTranslation()
    const {userId, firstName, username} = useContext(AuthContext)
    const me = signerOf(data, userId)
    const name = me ? signerName(me) : firstName || username
    const post = signerPost(data)
    const role = me?.title
        ? post
            ? t("signing.dialog.check.titlePost", {title: me.title, post})
            : me.title
        : post
    return (
        <Stack spacing={2}>
            <Typography color="text.secondary">
                {t("signing.dialog.check.signingAs", {name})}
                {role ? ` · ${role}` : ""}
            </Typography>
            <Box>
                <Typography variant="h6" component="p">
                    {requestTitle(t, data)}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                    {actionDescription(t, data.request.action, data.seals_ballots)}
                </Typography>
            </Box>
            <SigningSubject data={data} api={api} variant={SigningSubjectVariant.Dialog} />
            {documentKindOf(data.request) !== DocumentKind.NoDocument ? (
                <FormControlLabel
                    control={
                        <Checkbox
                            checked={confirmed}
                            onChange={(event) => onConfirm(event.target.checked)}
                        />
                    }
                    label={t("signing.dialog.check.confirmDocument", {
                        object: actionObject(t, data.request.action),
                    })}
                />
            ) : null}
        </Stack>
    )
}

const CertificateCard: React.FC<{certificate: OpenedCertificate; timeZone?: string | null}> = ({
    certificate,
    timeZone,
}) => {
    const {t} = useTranslation()
    const format = useSigningFormat(timeZone)
    return (
        <Box>
            <Typography sx={{fontWeight: 700}}>
                {certificate.commonName ?? certificate.subject}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {t("signing.widget.certificateCard", {
                    issuer: certificate.issuerCommonName ?? certificate.issuer,
                    date: format.date(certificate.notAfter),
                    algorithm: t(`signing.widget.algorithms.${certificate.algorithm}`),
                })}
            </Typography>
            <Typography
                variant="caption"
                component="p"
                color="text.secondary"
                sx={{fontFamily: "monospace"}}
                title={colonHex(certificate.fingerprintSha256)}
            >
                {t("signing.widget.fingerprint", {
                    fingerprint: shortFingerprint(certificate.fingerprintSha256),
                })}
            </Typography>
        </Box>
    )
}

/** Identity and backstop checks: the drafts list the five main checks, these only when they fail. */
const SHOWN_ONLY_WHEN_FAILED = new Set<CertificateCheckId>([
    CertificateCheckId.RegisteredToOther,
    CertificateCheckId.AlreadySigned,
    CertificateCheckId.PostBinding,
    CertificateCheckId.Signature,
])

const CheckList: React.FC<{
    checks: ICertificateCheckResult[]
    certificate: OpenedCertificate
    timeZone?: string | null
}> = ({checks, certificate, timeZone}) => {
    const {t} = useTranslation()
    const format = useSigningFormat(timeZone)
    const label = ({id, ok, detail}: ICertificateCheckResult): string => {
        if (!ok) {
            switch (id) {
                case CertificateCheckId.TrustedIssuer:
                    return t("signing.widget.untrustedIssuer", {
                        issuer: certificate.issuerCommonName ?? certificate.issuer,
                    })
                case CertificateCheckId.RegisteredToOther:
                    return detail
                        ? t("signing.dialog.checks.failed.registered-to-other", {name: detail})
                        : t("signing.widget.registeredToSomeoneElse")
                default:
                    return t(`signing.dialog.checks.failed.${id}`)
            }
        }
        switch (id) {
            case CertificateCheckId.TrustedIssuer:
                return detail
                    ? t("signing.dialog.checks.passed.trusted-issuer", {root: detail})
                    : t("signing.widget.checkPassedNoDetail.trusted-issuer")
            case CertificateCheckId.NotRevoked:
                return detail
                    ? t("signing.dialog.checks.passed.not-revoked", {
                          time: format.time(detail) || detail,
                      })
                    : t("signing.widget.checkPassedNoDetail.not-revoked")
            case CertificateCheckId.Registered:
                return detail
                    ? t("signing.dialog.checks.passed.registered", {
                          date: format.date(detail) || detail,
                      })
                    : t("signing.dialog.checks.first-use")
            default:
                return t(`signing.dialog.checks.passed.${id}`)
        }
    }
    return (
        <List dense disablePadding aria-label={t("signing.widget.checksTitle")}>
            {checks
                .filter((check) => !check.ok || !SHOWN_ONLY_WHEN_FAILED.has(check.id))
                .map((check) => (
                    <ListItem
                        key={check.id}
                        disableGutters
                        data-check={check.id}
                        data-ok={check.ok}
                    >
                        <ListItemIcon sx={{minWidth: 32}}>
                            {check.ok ? (
                                <CheckCircleIcon color="success" fontSize="small" aria-hidden />
                            ) : (
                                <ErrorIcon color="error" fontSize="small" aria-hidden />
                            )}
                        </ListItemIcon>
                        <ListItemText
                            primary={label(check)}
                            slotProps={{primary: {color: check.ok ? "text.primary" : "error"}}}
                        />
                    </ListItem>
                ))}
        </List>
    )
}

const CertificateStep: React.FC<{
    state: IState
    dispatch: React.Dispatch<TAction>
    onRetryChecks: () => void
    onChoose: (index: number) => void
    timeZone?: string | null
}> = ({state, dispatch, onRetryChecks, onChoose, timeZone}) => {
    const {t} = useTranslation()
    const {tenant} = useContext(TenantContext)
    const passwordId = useId()
    const fileInput = useRef<HTMLInputElement>(null)
    const {file, certificate, checks, phase, fileError} = state
    const busy = phase === CertificatePhase.Opening || phase === CertificatePhase.Signing
    const organization = organizationName(tenant) ?? t("signing.widget.organization")
    const problem = !checks
        ? null
        : failed(checks, CertificateCheckId.TrustedIssuer)
          ? t("signing.dialog.problems.issuerNotAccepted", {organization})
          : failed(checks, CertificateCheckId.RegisteredToOther) ||
              failed(checks, CertificateCheckId.Registered)
            ? t("signing.dialog.problems.notForYou")
            : allPassed(checks)
              ? null
              : t("signing.widget.cantSign")
    const fileErrorText =
        fileError === CertificateFileErrorCode.WrongPassword
            ? t("signing.dialog.problems.wrongPassword")
            : fileError
              ? t(`signing.widget.fileErrors.${fileError}`)
              : null

    return (
        <Stack spacing={2}>
            <Typography>{t("signing.dialog.certificate.intro")}</Typography>
            <Paper
                variant="outlined"
                sx={{p: 1.5, display: "flex", alignItems: "center", gap: 1.5, flexWrap: "wrap"}}
            >
                <InsertDriveFileOutlinedIcon color="action" aria-hidden />
                <Box sx={{flex: 1, minWidth: 0}}>
                    {file ? (
                        <Typography
                            component="span"
                            sx={{fontWeight: 600, overflowWrap: "anywhere"}}
                        >
                            {file.name}
                            <Typography component="span" color="text.secondary">
                                {" · "}
                                {t("signing.widget.fileSize", {
                                    size: (file.size / 1024).toFixed(1),
                                })}
                            </Typography>
                        </Typography>
                    ) : null}
                </Box>
                <Button
                    variant="outlined"
                    disabled={busy}
                    onClick={() => fileInput.current?.click()}
                >
                    {file
                        ? t("signing.dialog.certificate.chooseAnother")
                        : t("signing.widget.chooseFile")}
                </Button>
                <input
                    ref={fileInput}
                    hidden
                    type="file"
                    accept=".p12,.pfx,application/x-pkcs12"
                    aria-label={t("signing.widget.fileInput")}
                    onChange={(event) => {
                        dispatch({type: "file", file: event.target.files?.[0] ?? null})
                        event.target.value = ""
                    }}
                />
            </Paper>

            {certificate ? (
                <Paper variant="outlined" sx={{p: 2}} data-testid="signing-certificate">
                    <Stack spacing={1.5}>
                        {state.ambiguous ? (
                            <TextField
                                select
                                size="small"
                                label={t("signing.widget.chooseCertificate")}
                                value={state.choices.indexOf(certificate)}
                                disabled={busy}
                                onChange={(event) => onChoose(Number(event.target.value))}
                                slotProps={{select: {native: true}}}
                            >
                                {state.choices.map((choice, index) => (
                                    <option key={choice.fingerprintSha256} value={index}>
                                        {[
                                            choice.commonName ?? choice.subject,
                                            choice.friendlyName,
                                            shortFingerprint(choice.fingerprintSha256),
                                        ]
                                            .filter(Boolean)
                                            .join(" · ")}
                                    </option>
                                ))}
                            </TextField>
                        ) : null}
                        <CertificateCard certificate={certificate} timeZone={timeZone} />
                        {phase === CertificatePhase.Checking ? (
                            <Stack direction="row" spacing={1} sx={{alignItems: "center"}}>
                                <CircularProgress size={16} aria-hidden />
                                <Typography variant="body2">
                                    {t("signing.widget.checking")}
                                </Typography>
                            </Stack>
                        ) : checks ? (
                            <CheckList
                                checks={checks}
                                certificate={certificate}
                                timeZone={timeZone}
                            />
                        ) : null}
                    </Stack>
                </Paper>
            ) : (
                <>
                    <TextField
                        id={passwordId}
                        label={t("signing.dialog.certificate.password")}
                        // A masked text field, not type=password: the laptop is shared and
                        // the browser must not offer to save the certificate's password.
                        type="text"
                        value={state.password}
                        onChange={(event) =>
                            dispatch({type: "password", password: event.target.value})
                        }
                        autoComplete="one-time-code"
                        disabled={!file || busy}
                        error={!!fileErrorText}
                        helperText={fileErrorText}
                        fullWidth
                        slotProps={{
                            htmlInput: {
                                "spellCheck": false,
                                "autoCapitalize": "off",
                                "data-lpignore": "true",
                                "data-1p-ignore": "true",
                                "data-testid": "certificate-password",
                                "style": {
                                    WebkitTextSecurity: state.showPassword ? "none" : "disc",
                                } as React.CSSProperties,
                            },
                            input: {
                                endAdornment: (
                                    <InputAdornment position="end">
                                        <IconButton
                                            onClick={() => dispatch({type: "togglePassword"})}
                                            aria-label={
                                                state.showPassword
                                                    ? t("signing.widget.hidePassword")
                                                    : t("signing.widget.showPassword")
                                            }
                                            edge="end"
                                            disabled={!file || busy}
                                        >
                                            {state.showPassword ? (
                                                <VisibilityOffIcon />
                                            ) : (
                                                <VisibilityIcon />
                                            )}
                                        </IconButton>
                                    </InputAdornment>
                                ),
                            },
                        }}
                    />
                    {phase === CertificatePhase.Opening ? (
                        <Stack
                            direction="row"
                            spacing={1}
                            sx={{alignItems: "center"}}
                            role="status"
                        >
                            <CircularProgress size={16} aria-hidden />
                            <Typography variant="body2">{t("signing.widget.opening")}</Typography>
                        </Stack>
                    ) : null}
                </>
            )}

            {state.closed ? (
                <Alert severity="warning" data-testid="signing-closed">
                    {t(state.closed.message, {
                        reason: state.closed.reason ? t(state.closed.reason) : undefined,
                    })}
                </Alert>
            ) : problem ? (
                <Alert severity="error">{problem}</Alert>
            ) : null}
            {state.error ? (
                <Alert
                    severity="error"
                    action={
                        state.error === "signing.widget.checkError" ? (
                            <Button color="inherit" size="small" onClick={onRetryChecks}>
                                {t("signing.widget.retry")}
                            </Button>
                        ) : undefined
                    }
                >
                    {t(state.error)}
                </Alert>
            ) : null}
        </Stack>
    )
}

const SignedStep: React.FC<{data: ISigningPanelData; state: IState}> = ({data, state}) => {
    const {t, i18n} = useTranslation()
    const format = useSigningFormat(data.time_zone)
    const {userId} = useContext(AuthContext)
    const signed = state.signed
    if (!signed) return null
    const {count, required} = signed.result
    const next = count < required ? pendingSigners(data, userId).map(signerName) : []
    const certificateName = state.certificate
        ? (state.certificate.commonName ?? commonNameOf(state.certificate.subject))
        : ""
    return (
        <Stack spacing={1.5} sx={{alignItems: "center", textAlign: "center", py: 2}}>
            <CheckCircleIcon color="success" sx={{fontSize: 56}} aria-hidden />
            <Typography variant="h5" component="p">
                {t("signing.dialog.signed.title")}
            </Typography>
            <Typography>{requestTitle(t, data)}</Typography>
            <Typography color="text.secondary">
                {format.time(signed.at)}
                {" · "}
                {t("signing.dialog.signed.withCertificate", {name: certificateName})}
            </Typography>
            <Typography sx={{fontWeight: 700}}>
                {count >= required
                    ? t("signing.dialog.signed.allIn", {total: required})
                    : t("signing.dialog.signed.count", {n: count, total: required})}
            </Typography>
            {next.length ? (
                <Typography color="text.secondary">
                    {t("signing.dialog.signed.next", {names: formatList(next, i18n.language)})}
                </Typography>
            ) : null}
        </Stack>
    )
}

export interface ISigningDialogProps {
    open: boolean
    data: ISigningPanelData
    api: ISigningApi
    onClose: () => void
    /** After the server accepted the approval. */
    onSigned?: (result: IApproveSigningRequestOutput) => void
    /** The request stopped waiting or changed while the dialog was open. */
    onRequestChanged?: () => void
}

/**
 * Signs a request in three steps: check what is signed, open the certificate
 * file in the browser (the server dry-runs its checks), sign. The file, the
 * key and the password never leave the browser; the dialog forgets them
 * when it closes.
 */
export const SigningDialog: React.FC<ISigningDialogProps> = ({
    open,
    data,
    api,
    onClose,
    onSigned,
    onRequestChanged,
}) => {
    const {t} = useTranslation()
    const titleId = useId()
    const [state, dispatch] = useReducer(reducer, INITIAL)
    // Results of work started for an earlier file or dialog are dropped.
    const generation = useRef(0)
    // One approval at a time, whatever the rendering does with fast clicks.
    const signing = useRef(false)
    const requestId = data.request.id
    const signedView = useSignedView(data)

    useEffect(() => {
        generation.current += 1
        dispatch({type: "reset"})
        if (open) {
            loadCrypto().catch(() => undefined)
        }
    }, [open, requestId])

    useEffect(() => {
        generation.current += 1
    }, [state.file])

    const runChecks = useCallback(
        async (certificate: OpenedCertificate, current: number) => {
            dispatch({type: "checking"})
            try {
                const {checks} = await api.checkCertificate(requestId, {
                    chain_pem: certificate.chainPem,
                })
                if (current === generation.current) dispatch({type: "checked", checks})
            } catch {
                if (current === generation.current) dispatch({type: "checkFailed"})
            }
        },
        [api, requestId]
    )

    const openCertificate = async () => {
        const {file, password} = state
        if (!file) return
        const current = generation.current
        dispatch({type: "opening"})
        let bytes: Uint8Array | null = null
        try {
            const crypto = await loadCrypto()
            bytes = new Uint8Array(await file.arrayBuffer())
            // Let "Opening…" paint: key derivation is synchronous and can take seconds.
            await new Promise((resolve) => setTimeout(resolve, 0))
            const {choices, ambiguous} = await crypto.openP12Choices(bytes, password)
            if (current !== generation.current) return
            dispatch({type: "opened", choices, ambiguous})
            await runChecks(choices[0], current)
        } catch (error) {
            if (current !== generation.current) return
            if (!(error instanceof CertificateFileError)) {
                // Not the file's fault (e.g. the crypto module didn't load): nothing to log.
                dispatch({type: "openError"})
                return
            }
            dispatch({type: "openFailed", code: error.code})
            // Only the file name and the reason: never the file or the password.
            api.reportOpenFailure(requestId, {
                file_name: file.name,
                reason: openFailureReason(error.code),
            }).catch(() => undefined)
        } finally {
            bytes?.fill(0)
        }
    }

    /** The request as it is now; a closed or changed one ends the attempt. */
    const refetch = async (): Promise<ISigningPanelData | IClosed> => {
        const fresh = validatePanel(await api.getRequest(requestId))
        const closed = closedOf(fresh, data)
        if (closed) onRequestChanged?.()
        return closed ?? fresh
    }

    const sign = async () => {
        const {certificate} = state
        if (!certificate || !allPassed(state.checks) || state.closed || signing.current) return
        signing.current = true
        const current = generation.current
        dispatch({type: "signing"})
        try {
            const crypto = await loadCrypto()
            // Right before signing: still waiting, still the payload shown, a fresh document URL.
            const fresh = await refetch()
            if (current !== generation.current) return
            if (!("request" in fresh)) {
                dispatch({type: "closed", closed: fresh})
                return
            }
            const result = await crypto.signAndApprove(api, fresh, certificate)
            if (current !== generation.current) return
            dispatch({type: "signed", result, at: new Date()})
            onSigned?.(result)
        } catch (error) {
            if (current !== generation.current) return
            if (
                error instanceof SigningApiError &&
                error.kind === SigningApiErrorKind.AlreadySigned
            ) {
                dispatch({type: "closed", closed: {message: "signing.widget.alreadySigned"}})
                onRequestChanged?.()
                return
            }
            if (!(error instanceof SigningApiError && error.kind === SigningApiErrorKind.Refused)) {
                // After a failed prepare or approval, the request may have moved on.
                const now = await refetch().catch(() => null)
                if (current !== generation.current) return
                if (now && !("request" in now)) {
                    dispatch({type: "closed", closed: now})
                    return
                }
            }
            dispatch({type: "signFailed", ...signErrorKey(error)})
        } finally {
            signing.current = false
        }
    }

    const busy =
        state.phase === CertificatePhase.Opening || state.phase === CertificatePhase.Signing
    const close = () => {
        if (!busy) onClose()
    }
    const needsConfirmation = documentKindOf(data.request) !== DocumentKind.NoDocument
    const steps = [
        t("signing.dialog.steps.check"),
        t("signing.dialog.steps.certificate"),
        t("signing.dialog.steps.signed"),
    ]

    return (
        <Dialog open={open} onClose={close} fullWidth maxWidth="sm" aria-labelledby={titleId}>
            <DialogTitle id={titleId} sx={{display: "flex", alignItems: "center", gap: 1}}>
                <DrawIcon aria-hidden />
                {t("signing.dialog.title", {object: actionObject(t, data.request.action)})}
            </DialogTitle>
            <DialogContent>
                <Stepper activeStep={state.signed ? steps.length : state.step} sx={{mb: 3, mt: 1}}>
                    {steps.map((label) => (
                        <Step key={label}>
                            <StepLabel>{label}</StepLabel>
                        </Step>
                    ))}
                </Stepper>
                {state.step === SigningDialogStep.Check ? (
                    <CheckStep
                        data={data}
                        api={api}
                        confirmed={state.confirmed}
                        onConfirm={(confirmed) => dispatch({type: "confirm", confirmed})}
                    />
                ) : state.step === SigningDialogStep.Certificate ? (
                    <CertificateStep
                        state={state}
                        onChoose={(index) => {
                            const choice = state.choices[index]
                            if (!choice) return
                            dispatch({type: "choose", index})
                            runChecks(choice, generation.current).catch(() => undefined)
                        }}
                        timeZone={data.time_zone}
                        dispatch={dispatch}
                        onRetryChecks={() => {
                            if (state.certificate) {
                                runChecks(state.certificate, generation.current).catch(
                                    () => undefined
                                )
                            }
                        }}
                    />
                ) : (
                    <SignedStep data={data} state={state} />
                )}
                <LocalSigningNote />
            </DialogContent>
            <DialogActions sx={{px: 3, pb: 2}}>
                {state.step === SigningDialogStep.Check ? (
                    <>
                        <Button onClick={close}>{t("signing.dialog.cancel")}</Button>
                        <Button
                            variant="contained"
                            disabled={
                                !signedView.verified || (needsConfirmation && !state.confirmed)
                            }
                            onClick={() =>
                                dispatch({type: "goto", step: SigningDialogStep.Certificate})
                            }
                        >
                            {t("signing.widget.continue")}
                        </Button>
                    </>
                ) : state.step === SigningDialogStep.Certificate ? (
                    <>
                        <Button onClick={close} disabled={busy}>
                            {t("signing.dialog.cancel")}
                        </Button>
                        {state.certificate ? (
                            <Button
                                variant="contained"
                                startIcon={
                                    state.phase === CertificatePhase.Signing ? (
                                        <CircularProgress size={16} color="inherit" aria-hidden />
                                    ) : (
                                        <DrawIcon />
                                    )
                                }
                                disabled={
                                    state.phase !== CertificatePhase.Checked ||
                                    !allPassed(state.checks) ||
                                    !!state.closed
                                }
                                onClick={() => {
                                    sign().catch(() => undefined)
                                }}
                            >
                                {state.phase === CertificatePhase.Signing
                                    ? t("signing.widget.signing")
                                    : t("signing.dialog.sign")}
                            </Button>
                        ) : (
                            <>
                                <Button
                                    variant="outlined"
                                    disabled={busy}
                                    onClick={() =>
                                        dispatch({type: "goto", step: SigningDialogStep.Check})
                                    }
                                >
                                    {t("signing.dialog.back")}
                                </Button>
                                <Button
                                    variant="contained"
                                    disabled={!state.file || busy}
                                    onClick={() => {
                                        openCertificate().catch(() => undefined)
                                    }}
                                >
                                    {t("signing.dialog.certificate.open")}
                                </Button>
                            </>
                        )}
                    </>
                ) : (
                    <Button variant="contained" onClick={onClose}>
                        {t("signing.widget.done")}
                    </Button>
                )}
            </DialogActions>
        </Dialog>
    )
}
