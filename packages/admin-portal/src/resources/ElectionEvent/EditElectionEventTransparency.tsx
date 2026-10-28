// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The election event's Transparency tab (step-integration.md, section 2.8):
// its Vitrine mode, the trust policy the Electoral Board seals, the logs with
// their witnesses and hubs, and the witness enrolment notes. Everything Harvest
// and the sidecar return is shown as text; the policy's ID and words are
// computed here with @vitrine/verify, and its labels come from the UI kit.

import React, {ChangeEvent, useEffect, useMemo, useState} from "react"
import {useMutation, useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    Alert,
    AlertTitle,
    Box,
    Button,
    Card,
    CardContent,
    Chip,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
    FormControl,
    FormControlLabel,
    FormLabel,
    Link,
    Radio,
    RadioGroup,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import * as verify from "@vitrine/verify"
import type {Policy, PolicySummary} from "@vitrine/verify"
import {wordsFor} from "@vitrine/ui"
import type {Words} from "@vitrine/ui"
import {IPermissions} from "@/types/keycloak"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {
    ENABLE_TRANSPARENCY,
    EnableTransparencyMutation,
    EnableTransparencyMutationVariables,
    GET_TRANSPARENCY,
    GetTransparencyQuery,
    GetTransparencyQueryVariables,
    ITransparencyOutput,
    UPLOAD_TRANSPARENCY_POLICY,
    UploadTransparencyPolicyMutation,
    UploadTransparencyPolicyMutationVariables,
} from "@/queries/Transparency"
import {
    boardCommands,
    draftFileName,
    ENABLED_MODES,
    EPolicyMismatch,
    EPolicyState,
    ETransparencyMode,
    ILogView,
    isKnownRefusal,
    MAX_POLICY_CHARS,
    normalisePolicyText,
    policyHex,
    policyMismatch,
    readEnrolments,
    readLogs,
    readMode,
    safeExternalUrl,
    sameDraft,
    TPolicyExamination,
    examinePolicy,
    vitrineLanguage,
} from "@/services/Transparency"

const ROLE = {headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_TRANSPARENCY_TAB}}
const TEXT_MEDIA_TYPE = "text/plain;charset=utf-8"

enum ESdkState {
    LOADING = "loading",
    READY = "ready",
    FAILED = "failed",
}

type TNotice = {severity: "success" | "error"; text: string}

const useVitrineSdk = (): ESdkState => {
    const [state, setState] = useState(ESdkState.LOADING)
    useEffect(() => {
        let live = true
        const load = async () => {
            try {
                await verify.init()
                if (live) setState(ESdkState.READY)
            } catch {
                if (live) setState(ESdkState.FAILED)
            }
        }
        load()
        return () => {
            live = false
        }
    }, [])
    return state
}

const formatTime = (seconds: number, lang: string): string => {
    const date = new Date(seconds * 1000)
    try {
        return date.toLocaleString(lang)
    } catch {
        return date.toISOString()
    }
}

const useDuration = () => {
    const {t} = useTranslation()
    return (seconds: number): string => {
        const s = Math.max(0, Math.round(seconds))
        if (s < 120) return t("transparencyTab.duration.seconds", {n: s})
        if (s < 7200) return t("transparencyTab.duration.minutes", {n: Math.round(s / 60)})
        return t("transparencyTab.duration.hours", {n: Math.round(s / 3600)})
    }
}

const downloadText = (name: string, text: string): void => {
    const url = URL.createObjectURL(new Blob([text], {type: TEXT_MEDIA_TYPE}))
    const a = document.createElement("a")
    a.href = url
    a.download = name
    a.rel = "noopener"
    document.body.appendChild(a)
    a.click()
    a.remove()
    URL.revokeObjectURL(url)
}

const monospace = {fontFamily: "monospace", overflowWrap: "anywhere"} as const

const CopyButton: React.FC<{text: string; label: string; testId?: string}> = ({
    text,
    label,
    testId,
}) => {
    const {t} = useTranslation()
    const [copied, setCopied] = useState<boolean | undefined>()
    const copy = async () => {
        try {
            await navigator.clipboard.writeText(text)
            setCopied(true)
        } catch {
            setCopied(false)
        }
    }
    return (
        <Button size="small" onClick={copy} data-testid={testId}>
            {copied === undefined
                ? label
                : copied
                  ? t("transparencyTab.copied")
                  : t("transparencyTab.copyFailed")}
        </Button>
    )
}

// -- Mode -------------------------------------------------------------------------

interface IModeCardProps {
    mode: ETransparencyMode
    refusal?: string
    locked: boolean
    enabling: boolean
    onEnable: (mode: ETransparencyMode) => void
}

const RefusalAlert: React.FC<{code: string}> = ({code}) => {
    const {t} = useTranslation()
    return (
        <Alert severity="warning" data-testid="public-refusal">
            <AlertTitle>{t("transparencyTab.mode.refusalTitle")}</AlertTitle>
            {isKnownRefusal(code)
                ? t(`transparencyTab.mode.refusals.${code}`)
                : t("transparencyTab.mode.refusalUnknown", {code})}{" "}
            {t("transparencyTab.mode.refusalOptions")}
        </Alert>
    )
}

const ModeCard: React.FC<IModeCardProps> = ({mode, refusal, locked, enabling, onEnable}) => {
    const {t} = useTranslation()
    const publicRefused = Boolean(refusal)
    const [choice, setChoice] = useState(
        publicRefused ? ETransparencyMode.PRIVATE : ETransparencyMode.PUBLIC
    )
    const [confirming, setConfirming] = useState(false)
    const chosen = publicRefused && choice === ETransparencyMode.PUBLIC ? undefined : choice

    return (
        <Card variant="outlined">
            <CardContent>
                <Stack spacing={2}>
                    <Typography variant="h6" component="h3">
                        {t("transparencyTab.mode.title")}
                    </Typography>
                    <Box>
                        <Chip
                            data-testid="transparency-mode"
                            label={t(`transparencyTab.mode.${mode}`)}
                            color={mode === ETransparencyMode.DISABLED ? "default" : "success"}
                        />
                        <Typography variant="body2" sx={{mt: 1}}>
                            {t(`transparencyTab.mode.${mode}Help`)}
                        </Typography>
                    </Box>
                    {locked ? (
                        <Alert severity="info" data-testid="transparency-locked">
                            {t("transparencyTab.mode.locked")}
                        </Alert>
                    ) : null}
                    {refusal && mode === ETransparencyMode.DISABLED ? (
                        <RefusalAlert code={refusal} />
                    ) : null}
                    {mode === ETransparencyMode.DISABLED && !locked ? (
                        <Box data-testid="enable-transparency">
                            <FormControl>
                                <FormLabel id="transparency-mode-choice">
                                    {t("transparencyTab.mode.choose")}
                                </FormLabel>
                                <RadioGroup
                                    aria-labelledby="transparency-mode-choice"
                                    value={chosen ?? ""}
                                    onChange={(e) => setChoice(e.target.value as ETransparencyMode)}
                                >
                                    {ENABLED_MODES.map((m) => (
                                        <FormControlLabel
                                            key={m}
                                            value={m}
                                            control={<Radio />}
                                            disabled={
                                                m === ETransparencyMode.PUBLIC && publicRefused
                                            }
                                            label={`${t(`transparencyTab.mode.${m}`)}: ${t(
                                                `transparencyTab.mode.${m}Help`
                                            )}`}
                                        />
                                    ))}
                                </RadioGroup>
                            </FormControl>
                            <Box>
                                <Button
                                    variant="contained"
                                    disabled={!chosen || enabling}
                                    onClick={() => setConfirming(true)}
                                    data-testid="enable-transparency-button"
                                >
                                    {t("transparencyTab.mode.enable")}
                                </Button>
                            </Box>
                            <Dialog open={confirming} onClose={() => setConfirming(false)}>
                                <DialogTitle>
                                    {t("transparencyTab.mode.confirmTitle", {
                                        mode: chosen ? t(`transparencyTab.mode.${chosen}`) : "",
                                    })}
                                </DialogTitle>
                                <DialogContent>
                                    <DialogContentText>
                                        {t("transparencyTab.mode.confirmText")}
                                    </DialogContentText>
                                </DialogContent>
                                <DialogActions>
                                    <Button onClick={() => setConfirming(false)}>
                                        {t("transparencyTab.cancel")}
                                    </Button>
                                    <Button
                                        variant="contained"
                                        data-testid="confirm-enable-transparency"
                                        onClick={() => {
                                            setConfirming(false)
                                            if (chosen) onEnable(chosen)
                                        }}
                                    >
                                        {t("transparencyTab.mode.enable")}
                                    </Button>
                                </DialogActions>
                            </Dialog>
                        </Box>
                    ) : null}
                </Stack>
            </CardContent>
        </Card>
    )
}

// -- Policy -------------------------------------------------------------------------

const PolicyFacts: React.FC<{
    policy: Policy
    summary: PolicySummary
    words: Words
    testId: string
}> = ({policy, summary, words, testId}) => {
    const {t} = useTranslation()
    const sixWords = summary.words.join(" ")
    return (
        <Box
            component="dl"
            data-testid={testId}
            sx={{
                "display": "grid",
                "gridTemplateColumns": "minmax(8em, max-content) 1fr",
                "gap": "4px 16px",
                "m": 0,
                "& dt": {fontWeight: "bold"},
                "& dd": {m: 0, overflowWrap: "anywhere"},
                "& ul": {m: 0, pl: 2},
            }}
        >
            <dt>{words.t("receipt-policy-words")}</dt>
            <dd>
                <Typography component="span" lang={summary.lang} data-testid={`${testId}-words`}>
                    {sixWords}
                </Typography>{" "}
                <CopyButton text={sixWords} label={t("transparencyTab.copy")} />
            </dd>
            <dt>{words.t("receipt-policy-id")}</dt>
            <dd>
                <Box component="code" sx={monospace} data-testid={`${testId}-id`}>
                    {policy.id}
                </Box>{" "}
                <CopyButton text={policy.id} label={t("transparencyTab.copy")} />
            </dd>
            <dt>{words.t("receipt-event")}</dt>
            <dd>{summary.event}</dd>
            <dt>{words.t("receipt-witnesses")}</dt>
            <dd>
                <ul>
                    {summary.witnesses.map((w) => (
                        <li key={w.name}>
                            {w.name}
                            {w.party
                                ? `, ${words.t("receipt-witness-party", {party: w.party})}`
                                : null}
                        </li>
                    ))}
                </ul>
            </dd>
        </Box>
    )
}

type TDraftExamination = Exclude<TPolicyExamination, {state: EPolicyState.SEALED}>

const DraftSection: React.FC<{
    electionEventId: string
    text: string
    examined: TDraftExamination
    changed: boolean
    words: Words
}> = ({electionEventId, text, examined, changed, words}) => {
    const {t} = useTranslation()
    if (examined.state === EPolicyState.INVALID) {
        return (
            <Alert severity="error" data-testid="draft-invalid">
                {t("transparencyTab.policy.invalidDraft", {reason: words.reason(examined.reason)})}
            </Alert>
        )
    }
    return (
        <Stack spacing={1} data-testid="draft-policy">
            <Typography variant="subtitle1" component="h4">
                {t("transparencyTab.policy.draftTitle")}
            </Typography>
            {changed ? (
                <Alert severity="info">{t("transparencyTab.policy.changedDraft")}</Alert>
            ) : null}
            <Typography variant="body2">{t("transparencyTab.policy.draftHelp")}</Typography>
            <Box>
                <Typography variant="body2" sx={{fontWeight: "bold"}}>
                    {t("transparencyTab.policy.draftWords")}
                </Typography>
                <Typography lang={examined.policy.words} data-testid="draft-words">
                    {examined.words.join(" ")}
                </Typography>
                <Typography variant="caption">
                    {t("transparencyTab.policy.draftWordsHelp")}
                </Typography>
            </Box>
            <Box>
                <Button
                    variant="outlined"
                    data-testid="download-draft"
                    onClick={() => downloadText(draftFileName(electionEventId), text)}
                >
                    {t("transparencyTab.policy.download")}
                </Button>
            </Box>
            <Typography variant="body2" sx={{fontWeight: "bold"}}>
                {t("transparencyTab.policy.commands")}
            </Typography>
            <Box
                component="pre"
                data-testid="board-commands"
                sx={{...monospace, whiteSpace: "pre-wrap", m: 0}}
            >
                {boardCommands(electionEventId).join("\n")}
            </Box>
        </Stack>
    )
}

interface IUploadFormProps {
    electionEventId: string
    mode: ETransparencyMode
    successor: boolean
    draft?: Policy
    uploading: boolean
    words: Words
    onUpload: (signedPolicy: string) => Promise<boolean>
}

const UploadForm: React.FC<IUploadFormProps> = ({
    electionEventId,
    mode,
    successor,
    draft,
    uploading,
    words,
    onUpload,
}) => {
    const {t} = useTranslation()
    const [raw, setRaw] = useState("")
    const [fileProblem, setFileProblem] = useState<string | undefined>()
    const text = useMemo(() => normalisePolicyText(raw), [raw])
    const tooLarge = text.length > MAX_POLICY_CHARS
    const examined = useMemo(
        () => (text && !tooLarge ? examinePolicy(verify, text) : undefined),
        [text, tooLarge]
    )
    const sealed = examined?.state === EPolicyState.SEALED ? examined : undefined
    const mismatch = sealed ? policyMismatch(sealed.policy, electionEventId, mode) : undefined
    const otherDraft = sealed && draft ? !sameDraft(sealed.policy, draft) : false

    const readFile = async (e: ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0]
        e.target.value = ""
        if (!file) return
        if (file.size > MAX_POLICY_CHARS * 4) {
            setFileProblem(t("transparencyTab.upload.tooLarge"))
            return
        }
        try {
            setRaw(await file.text())
            setFileProblem(undefined)
        } catch {
            setFileProblem(t("transparencyTab.upload.readError"))
        }
    }

    const submit = async () => {
        if (!sealed || mismatch) return
        if (await onUpload(text)) setRaw("")
    }

    return (
        <Stack spacing={1} data-testid="upload-policy">
            <Typography variant="subtitle1" component="h4">
                {successor
                    ? t("transparencyTab.upload.successorTitle")
                    : t("transparencyTab.upload.title")}
            </Typography>
            <Typography variant="body2">{t("transparencyTab.upload.help")}</Typography>
            <Box>
                <Button variant="outlined" component="label">
                    {t("transparencyTab.upload.chooseFile")}
                    <input
                        hidden
                        type="file"
                        accept=".txt,text/plain"
                        data-testid="policy-file"
                        onChange={readFile}
                    />
                </Button>
            </Box>
            <TextField
                label={t("transparencyTab.upload.text")}
                multiline
                minRows={4}
                value={raw}
                onChange={(e) => setRaw(e.target.value)}
                slotProps={{htmlInput: {"data-testid": "policy-text", "spellCheck": false}}}
                sx={{"& textarea": monospace}}
            />
            {fileProblem ? <Alert severity="error">{fileProblem}</Alert> : null}
            {tooLarge ? (
                <Alert severity="error" data-testid="policy-too-large">
                    {t("transparencyTab.upload.tooLarge")}
                </Alert>
            ) : null}
            {examined?.state === EPolicyState.DRAFT ? (
                <Alert severity="warning" data-testid="policy-is-draft">
                    {t("transparencyTab.upload.draft", {words: examined.words.join(" ")})}
                </Alert>
            ) : null}
            {examined?.state === EPolicyState.INVALID ? (
                <Alert severity="error" data-testid="policy-invalid">
                    {t("transparencyTab.upload.invalid", {reason: words.reason(examined.reason)})}
                </Alert>
            ) : null}
            {sealed ? (
                <Stack spacing={1} data-testid="policy-preview">
                    <Typography variant="body2">{t("transparencyTab.upload.preview")}</Typography>
                    <PolicyFacts
                        policy={sealed.policy}
                        summary={sealed.summary}
                        words={words}
                        testId="candidate-policy"
                    />
                    {mismatch === EPolicyMismatch.EVENT ? (
                        <Alert severity="error" data-testid="policy-other-event">
                            {t("transparencyTab.upload.otherEvent")}
                        </Alert>
                    ) : null}
                    {mismatch === EPolicyMismatch.VISIBILITY ? (
                        <Alert severity="error" data-testid="policy-other-visibility">
                            {t("transparencyTab.upload.otherVisibility", {
                                policy: t(`transparencyTab.mode.${sealed.policy.visibility}`),
                                event: t(`transparencyTab.mode.${mode}`),
                            })}
                        </Alert>
                    ) : null}
                    {!mismatch && otherDraft ? (
                        <Alert severity="warning" data-testid="policy-not-from-draft">
                            {t("transparencyTab.upload.notFromDraft")}
                        </Alert>
                    ) : null}
                    <Box>
                        <Button
                            variant="contained"
                            disabled={Boolean(mismatch) || uploading}
                            onClick={submit}
                            data-testid="upload-policy-button"
                        >
                            {t("transparencyTab.upload.submit")}
                        </Button>
                    </Box>
                </Stack>
            ) : null}
        </Stack>
    )
}

interface IPolicyCardProps {
    electionEventId: string
    output: ITransparencyOutput
    mode: ETransparencyMode
    sealed?: TPolicyExamination
    draft?: TPolicyExamination
    uploading: boolean
    words: Words
    onUpload: (signedPolicy: string) => Promise<boolean>
}

const PolicyCard: React.FC<IPolicyCardProps> = ({
    electionEventId,
    output,
    mode,
    sealed,
    draft,
    uploading,
    words,
    onUpload,
}) => {
    const {t} = useTranslation()
    const sealedPolicy = sealed?.state === EPolicyState.SEALED ? sealed : undefined
    const draftPolicy = draft && draft.state !== EPolicyState.INVALID ? draft.policy : undefined
    const changed = Boolean(
        sealedPolicy && draftPolicy && !sameDraft(sealedPolicy.policy, draftPolicy)
    )
    const draftToShow =
        draft && draft.state !== EPolicyState.SEALED && (!sealedPolicy || changed)
            ? draft
            : undefined
    const loggedId = policyHex(output.policy_id)

    return (
        <Card variant="outlined">
            <CardContent>
                <Stack spacing={2}>
                    <Typography variant="h6" component="h3">
                        {t("transparencyTab.policy.title")}
                    </Typography>
                    {sealedPolicy ? (
                        <Stack spacing={1} data-testid="sealed-policy">
                            <Typography variant="subtitle1" component="h4">
                                {t("transparencyTab.policy.sealedTitle")}
                            </Typography>
                            <Typography variant="body2">
                                {t("transparencyTab.policy.sealedHelp")}
                            </Typography>
                            <PolicyFacts
                                policy={sealedPolicy.policy}
                                summary={sealedPolicy.summary}
                                words={words}
                                testId="event-policy"
                            />
                            {output.policy_id && loggedId !== policyHex(sealedPolicy.policy.id) ? (
                                <Alert severity="error" data-testid="policy-id-mismatch">
                                    {t("transparencyTab.policy.idMismatch", {id: output.policy_id})}
                                </Alert>
                            ) : null}
                        </Stack>
                    ) : null}
                    {sealed && sealed.state !== EPolicyState.SEALED ? (
                        <Alert severity="error" data-testid="stored-policy-invalid">
                            {t("transparencyTab.policy.invalidStored", {
                                reason: words.reason(
                                    sealed.state === EPolicyState.INVALID
                                        ? sealed.reason
                                        : {code: "policy-draft"}
                                ),
                            })}
                        </Alert>
                    ) : null}
                    {draftToShow && output.draft_policy ? (
                        <DraftSection
                            electionEventId={electionEventId}
                            text={output.draft_policy}
                            examined={draftToShow}
                            changed={changed}
                            words={words}
                        />
                    ) : null}
                    {!sealed && !output.draft_policy ? (
                        <Typography variant="body2" data-testid="no-draft">
                            {t("transparencyTab.policy.none")}
                        </Typography>
                    ) : null}
                    {!output.locked ? (
                        <UploadForm
                            electionEventId={electionEventId}
                            mode={mode}
                            successor={Boolean(sealedPolicy)}
                            draft={draftPolicy}
                            uploading={uploading}
                            words={words}
                            onUpload={onUpload}
                        />
                    ) : null}
                </Stack>
            </CardContent>
        </Card>
    )
}

// -- Logs ---------------------------------------------------------------------------

const LogCard: React.FC<{log: ILogView; policyId?: string; lang: string}> = ({
    log,
    policyId,
    lang,
}) => {
    const {t} = useTranslation()
    const duration = useDuration()
    const none = t("transparencyTab.logs.none")
    const unknown = t("transparencyTab.logs.unknown")
    const policyState = !log.policyHex
        ? t("transparencyTab.logs.policyNone")
        : log.policyHex === policyId
          ? t("transparencyTab.logs.policyCurrent")
          : t("transparencyTab.logs.policyOther")
    const rows: Array<[string, React.ReactNode]> = [
        [t("transparencyTab.logs.origin"), <code key="origin">{log.origin ?? unknown}</code>],
        [t("transparencyTab.logs.size"), log.size?.toLocaleString(lang) ?? unknown],
        [
            t("transparencyTab.logs.checkpointSize"),
            log.checkpoint?.size.toLocaleString(lang) ?? t("transparencyTab.logs.noCheckpoint"),
        ],
        [
            t("transparencyTab.logs.checkpointTime"),
            log.checkpoint?.time !== undefined ? formatTime(log.checkpoint.time, lang) : unknown,
        ],
        [t("transparencyTab.logs.policy"), policyState],
    ]
    if (log.queue) {
        rows.push([t("transparencyTab.logs.queueRows"), log.queue.rows.toLocaleString(lang)])
        if (log.queue.oldestSeconds !== undefined) {
            rows.push([t("transparencyTab.logs.queueOldest"), duration(log.queue.oldestSeconds)])
        }
    }

    return (
        <Card variant="outlined" data-testid={`log-${log.key}`}>
            <CardContent>
                <Stack spacing={1}>
                    <Stack
                        direction="row"
                        spacing={1}
                        alignItems="center"
                        flexWrap="wrap"
                        useFlexGap
                    >
                        <Typography variant="subtitle1" component="h4">
                            {log.key}
                        </Typography>
                        {log.visibility ? <Chip size="small" label={log.visibility} /> : null}
                        {log.frozen ? (
                            <Chip size="small" label={t("transparencyTab.logs.frozen")} />
                        ) : null}
                        {log.alert ? (
                            <Chip
                                size="small"
                                color="error"
                                data-testid={`log-${log.key}-alert`}
                                label={t("transparencyTab.logs.alert", {alert: log.alert})}
                            />
                        ) : null}
                    </Stack>
                    <Table size="small">
                        <TableBody>
                            {rows.map(([label, value]) => (
                                <TableRow key={label}>
                                    <TableCell component="th" scope="row">
                                        {label}
                                    </TableCell>
                                    <TableCell sx={{overflowWrap: "anywhere"}}>{value}</TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                    <Typography variant="subtitle2" component="h5">
                        {t("transparencyTab.logs.witnesses")}
                    </Typography>
                    {log.witnesses.length === 0 ? (
                        <Typography variant="body2">
                            {t("transparencyTab.logs.noWitnesses")}
                        </Typography>
                    ) : (
                        <Table size="small" data-testid={`log-${log.key}-witnesses`}>
                            <TableHead>
                                <TableRow>
                                    <TableCell>{t("transparencyTab.logs.witness")}</TableCell>
                                    <TableCell>{t("transparencyTab.logs.witnessState")}</TableCell>
                                    <TableCell>{t("transparencyTab.logs.witnessLag")}</TableCell>
                                    <TableCell>
                                        {t("transparencyTab.logs.witnessCosigned")}
                                    </TableCell>
                                    <TableCell>{t("transparencyTab.logs.witnessError")}</TableCell>
                                </TableRow>
                            </TableHead>
                            <TableBody>
                                {log.witnesses.map((w) => (
                                    <TableRow key={w.name} data-testid={`witness-${w.name}`}>
                                        <TableCell>{w.name}</TableCell>
                                        <TableCell>
                                            <Chip
                                                size="small"
                                                color={
                                                    w.alert
                                                        ? "error"
                                                        : w.current
                                                          ? "success"
                                                          : "warning"
                                                }
                                                label={
                                                    w.alert
                                                        ? t("transparencyTab.logs.witnessAlert")
                                                        : w.current
                                                          ? t("transparencyTab.logs.witnessCurrent")
                                                          : t("transparencyTab.logs.witnessBehind")
                                                }
                                            />
                                        </TableCell>
                                        <TableCell data-testid={`witness-${w.name}-lag`}>
                                            {w.behind !== undefined
                                                ? w.behind.toLocaleString(lang)
                                                : w.current
                                                  ? (0).toLocaleString(lang)
                                                  : unknown}
                                        </TableCell>
                                        <TableCell>
                                            {w.cosignedAgeSeconds !== undefined
                                                ? t("transparencyTab.logs.ago", {
                                                      duration: duration(w.cosignedAgeSeconds),
                                                  })
                                                : unknown}
                                        </TableCell>
                                        <TableCell sx={{overflowWrap: "anywhere"}}>
                                            {w.lastError ?? none}
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    )}
                    <Typography variant="subtitle2" component="h5">
                        {t("transparencyTab.logs.hubs")}
                    </Typography>
                    {log.hubs.length === 0 ? (
                        <Typography variant="body2">{t("transparencyTab.logs.noHubs")}</Typography>
                    ) : (
                        <Table size="small" data-testid={`log-${log.key}-hubs`}>
                            <TableHead>
                                <TableRow>
                                    <TableCell>{t("transparencyTab.logs.hub")}</TableCell>
                                    <TableCell>{t("transparencyTab.logs.hubSize")}</TableCell>
                                    <TableCell>{t("transparencyTab.logs.hubTime")}</TableCell>
                                </TableRow>
                            </TableHead>
                            <TableBody>
                                {log.hubs.map((h) => (
                                    <TableRow key={h.name} data-testid={`hub-${h.name}`}>
                                        <TableCell>{h.name}</TableCell>
                                        <TableCell>
                                            {h.anchor?.size !== undefined
                                                ? h.anchor.size.toLocaleString(lang)
                                                : h.anchor
                                                  ? unknown
                                                  : t("transparencyTab.logs.hubNoAnchor")}
                                        </TableCell>
                                        <TableCell>
                                            {h.anchor?.time !== undefined
                                                ? formatTime(h.anchor.time, lang)
                                                : h.anchor
                                                  ? unknown
                                                  : null}
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    )}
                </Stack>
            </CardContent>
        </Card>
    )
}

// -- The tab --------------------------------------------------------------------------

export interface IEditElectionEventTransparencyProps {
    electionEventId?: string
    /** Called once the event's transparency changed, so that its record is
     * read again: the event's presentation changes with it. */
    onChanged?: () => void
}

export const EditElectionEventTransparency: React.FC<IEditElectionEventTransparencyProps> = ({
    electionEventId,
    onChanged,
}) => {
    const {t, i18n} = useTranslation()
    const lang = vitrineLanguage(i18n.language)
    const words = useMemo(() => wordsFor(lang), [lang])
    const sdk = useVitrineSdk()
    const ready = sdk === ESdkState.READY
    const [latest, setLatest] = useState<ITransparencyOutput | undefined>()
    const [notice, setNotice] = useState<TNotice | undefined>()

    const {data, loading, error, refetch} = useQuery<
        GetTransparencyQuery,
        GetTransparencyQueryVariables
    >(GET_TRANSPARENCY, {
        variables: {electionEventId: electionEventId ?? ""},
        skip: !electionEventId,
        fetchPolicy: "network-only",
        context: ROLE,
    })
    const [enableTransparency, {loading: enabling}] = useMutation<
        EnableTransparencyMutation,
        EnableTransparencyMutationVariables
    >(ENABLE_TRANSPARENCY, {context: ROLE})
    const [uploadTransparencyPolicy, {loading: uploading}] = useMutation<
        UploadTransparencyPolicyMutation,
        UploadTransparencyPolicyMutationVariables
    >(UPLOAD_TRANSPARENCY_POLICY, {context: ROLE})

    const output = latest ?? data?.get_transparency ?? undefined
    const sealed = useMemo(
        () => (ready && output?.policy ? examinePolicy(verify, output.policy) : undefined),
        [ready, output?.policy]
    )
    const draft = useMemo(
        () =>
            ready && output?.draft_policy ? examinePolicy(verify, output.draft_policy) : undefined,
        [ready, output?.draft_policy]
    )
    const sealedSummary = sealed?.state === EPolicyState.SEALED ? sealed.summary : undefined
    const logs = useMemo(
        () => readLogs(output?.logs, output?.status, sealedSummary?.hubs ?? []),
        [output?.logs, output?.status, sealedSummary]
    )
    const enrolments = useMemo(() => readEnrolments(output?.enrolments), [output?.enrolments])
    const explorer = safeExternalUrl(output?.explorer_url)

    if (!electionEventId) return null

    const failure = (key: string, e: unknown): TNotice => ({
        severity: "error",
        text: t(key, {reason: getGraphQLActionErrorReason(e) ?? t("transparencyTab.unknownError")}),
    })

    const retry = async () => {
        setLatest(undefined)
        try {
            await refetch()
        } catch {
            // The query's error is shown.
        }
    }

    const enable = async (mode: ETransparencyMode) => {
        setNotice(undefined)
        try {
            const result = await enableTransparency({variables: {electionEventId, mode}})
            const next = result.data?.enable_transparency
            if (!next) return
            setLatest(next)
            if (readMode(next.mode) !== ETransparencyMode.DISABLED) {
                setNotice({severity: "success", text: t("transparencyTab.mode.enabled")})
                onChanged?.()
            }
        } catch (e) {
            setNotice(failure("transparencyTab.mode.enableError", e))
        }
    }

    const upload = async (signedPolicy: string): Promise<boolean> => {
        setNotice(undefined)
        try {
            const result = await uploadTransparencyPolicy({
                variables: {electionEventId, signedPolicy},
            })
            const next = result.data?.upload_transparency_policy
            if (!next) return false
            setLatest(next)
            setNotice({severity: "success", text: t("transparencyTab.upload.uploaded")})
            onChanged?.()
            return true
        } catch (e) {
            setNotice(failure("transparencyTab.upload.error", e))
            return false
        }
    }

    if (!output) {
        if (loading) {
            return (
                <Box sx={{p: 2}} data-testid="transparency-loading">
                    <CircularProgress size={24} /> {t("transparencyTab.loading")}
                </Box>
            )
        }
        return (
            <Box sx={{p: 2}}>
                <Alert
                    severity="error"
                    data-testid="transparency-load-error"
                    action={
                        <Button color="inherit" size="small" onClick={retry}>
                            {t("transparencyTab.retry")}
                        </Button>
                    }
                >
                    {t("transparencyTab.loadError", {
                        reason:
                            (error && getGraphQLActionErrorReason(error)) ??
                            t("transparencyTab.unknownError"),
                    })}
                </Alert>
            </Box>
        )
    }

    const mode = readMode(output.mode)

    return (
        <Stack spacing={2} sx={{p: 2}} data-testid="transparency-tab">
            <Box>
                <Typography variant="h5" component="h2">
                    {t("transparencyTab.title")}
                </Typography>
                <Typography variant="body2">{t("transparencyTab.help")}</Typography>
            </Box>
            {notice ? (
                <Alert
                    severity={notice.severity}
                    data-testid="transparency-notice"
                    onClose={() => setNotice(undefined)}
                >
                    {notice.text}
                </Alert>
            ) : null}
            <ModeCard
                mode={mode}
                refusal={output.refusal ?? undefined}
                locked={output.locked}
                enabling={enabling}
                onEnable={enable}
            />
            {mode !== ETransparencyMode.DISABLED ? (
                sdk === ESdkState.FAILED ? (
                    <Alert severity="error" data-testid="vitrine-sdk-failed">
                        {t("transparencyTab.sdkError")}
                    </Alert>
                ) : !ready ? (
                    <Box data-testid="vitrine-sdk-loading">
                        <CircularProgress size={24} /> {t("transparencyTab.sdkLoading")}
                    </Box>
                ) : (
                    <PolicyCard
                        electionEventId={electionEventId}
                        output={output}
                        mode={mode}
                        sealed={sealed}
                        draft={draft}
                        uploading={uploading}
                        words={words}
                        onUpload={upload}
                    />
                )
            ) : null}
            {mode !== ETransparencyMode.DISABLED ? (
                <Stack spacing={1} data-testid="transparency-logs">
                    <Typography variant="h6" component="h3">
                        {t("transparencyTab.logs.title")}
                    </Typography>
                    {explorer ? (
                        <Box>
                            <Link
                                href={explorer}
                                target="_blank"
                                rel="noopener noreferrer"
                                data-testid="explorer-link"
                            >
                                {t("transparencyTab.logs.explorer")}
                            </Link>
                        </Box>
                    ) : null}
                    {logs.length === 0 ? (
                        <Typography variant="body2" data-testid="no-logs">
                            {t("transparencyTab.logs.unreported")}
                        </Typography>
                    ) : (
                        logs.map((log) => (
                            <LogCard
                                key={log.key}
                                log={log}
                                policyId={policyHex(sealedSummary?.id)}
                                lang={lang}
                            />
                        ))
                    )}
                </Stack>
            ) : null}
            {enrolments.length > 0 ? (
                <Card variant="outlined" data-testid="witness-enrolments">
                    <CardContent>
                        <Stack spacing={1}>
                            <Typography variant="h6" component="h3">
                                {t("transparencyTab.enrolments.title")}
                            </Typography>
                            <Typography variant="body2">
                                {t("transparencyTab.enrolments.help")}
                            </Typography>
                            {enrolments.map((e) => (
                                <Box key={e.log} data-testid={`enrolment-${e.log}`}>
                                    <Typography variant="subtitle2" component="h4">
                                        {t("transparencyTab.enrolments.log", {log: e.log})}
                                    </Typography>
                                    <Box
                                        component="pre"
                                        sx={{...monospace, whiteSpace: "pre-wrap", m: 0}}
                                    >
                                        {e.note}
                                    </Box>
                                    <CopyButton
                                        text={e.note}
                                        label={t("transparencyTab.enrolments.copy")}
                                        testId={`copy-enrolment-${e.log}`}
                                    />
                                </Box>
                            ))}
                        </Stack>
                    </CardContent>
                </Card>
            ) : null}
        </Stack>
    )
}
