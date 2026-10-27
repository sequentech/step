// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useContext, useEffect, useRef, useState} from "react"
import {gql, useApolloClient} from "@apollo/client"
import {useNotify, useRecordContext} from "react-admin"
import {useTranslation} from "react-i18next"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
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
    MenuItem,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useEventZonedFormat} from "@/hooks/useZonedFormat"

export const GET_SECURITY_TESTS = gql`
    mutation GetSecurityTests($event: String!) {
        get_self_tests(election_event_id: $event) {
            settings
            runs
            baseline_present
            notifications
        }
    }
`
export const GET_SECURITY_LIFECYCLE = gql`
    query GetSecurityLifecycle($event: String!) {
        get_election_lifecycle(election_event_id: $event)
    }
`
export const RUN_SECURITY_TESTS = gql`
    mutation RunSecurityTests($event: String!, $trigger: String!) {
        run_self_tests(election_event_id: $event, trigger: $trigger) {
            task_id
            state
        }
    }
`
export const SAVE_SECURITY_SETTINGS = gql`
    mutation SaveSecuritySettings($event: String!, $settings: jsonb!) {
        update_self_test_settings(election_event_id: $event, settings: $settings) {
            settings
        }
    }
`
export const START_POST_PROCESSING = gql`
    mutation StartElectionPostProcessing($event: String!) {
        start_election_post_processing(election_event_id: $event)
    }
`

interface Settings {
    interval_minutes: number
    timeout_seconds: number
    additional_conditions: string[]
}
interface Run {
    run_id: string
    generation: number
    trigger: string
    started_at: string
    finished_at?: string | null
    outcome: string
    checks: {check: string; outcome: string; detail: string}[]
}
interface Notice {
    id: string
    kind: string
    state: string
    created_at: string
    failure?: string | null
}
interface Tests {
    settings: Settings
    runs: Run[]
    baseline_present: boolean
    notifications: Notice[]
}
interface Lifecycle {
    phase: string
    safety: string
    generation: number
    reason?: string | null
}
const asRole = (role: IPermissions) => ({headers: {"x-hasura-role": role}})
const CHECKS = [
    "ballot-box",
    "election-data",
    "audit-generation",
    "trusted-channel",
    "access-control",
    "clock",
]
const EXTRA_TRIGGERS = ["after-import", "after-pause", "before-counting"]

export default function SecurityTab() {
    const record = useRecordContext()
    const event = String(record?.id ?? "")
    const client = useApolloClient()
    const auth = useContext(AuthContext)
    const {t} = useTranslation()
    const notify = useNotify()
    const {format} = useEventZonedFormat(event)
    const [tests, setTests] = useState<Tests | null>(null)
    const [lifecycle, setLifecycle] = useState<Lifecycle | null>(null)
    const [draft, setDraft] = useState<Settings | null>(null)
    const [readError, setReadError] = useState(false)
    const [busy, setBusy] = useState(false)
    const [trigger, setTrigger] = useState("requested")
    const [postProcessing, setPostProcessing] = useState(false)
    const active = useRef(true)
    const currentEvent = useRef(event)
    const reading = useRef(false)
    const holds = (permission: IPermissions) => auth.isAuthorized(true, auth.tenantId, permission)
    const canRead = holds(IPermissions.SELF_TEST_READ)
    const canReadLifecycle = canRead
    const canRun = holds(IPermissions.SELF_TEST_RUN)
    const canConfigure = holds(IPermissions.SELF_TEST_SETTINGS_WRITE)
    const canPostProcess = holds(IPermissions.ELECTION_STATE_WRITE)
    const refresh = useCallback(async () => {
        if (!canRead || !event || reading.current) return
        reading.current = true
        try {
            const [testResult, stateResult] = await Promise.all([
                client.mutate<{get_self_tests: Tests}>({
                    mutation: GET_SECURITY_TESTS,
                    variables: {event},
                    context: asRole(IPermissions.SELF_TEST_READ),
                    fetchPolicy: "no-cache",
                }),
                canReadLifecycle
                    ? client.query<{get_election_lifecycle: Lifecycle}>({
                          query: GET_SECURITY_LIFECYCLE,
                          variables: {event},
                          context: asRole(IPermissions.SELF_TEST_READ),
                          fetchPolicy: "no-cache",
                      })
                    : null,
            ])
            if (!active.current || currentEvent.current !== event) return
            const result = testResult.data?.get_self_tests
            if (!result) throw new Error("Missing self-test status")
            setTests(result)
            setDraft((current) => current ?? result.settings)
            setLifecycle(stateResult?.data.get_election_lifecycle ?? null)
            setReadError(false)
        } catch {
            if (active.current) setReadError(true)
        } finally {
            reading.current = false
        }
    }, [canRead, canReadLifecycle, client, event])
    useEffect(() => {
        active.current = true
        currentEvent.current = event
        setTests(null)
        setLifecycle(null)
        setDraft(null)
        void refresh()
        const interval = window.setInterval(() => {
            void refresh()
        }, 5000)
        return () => {
            active.current = false
            window.clearInterval(interval)
        }
    }, [refresh])
    const pending = tests?.runs.some((run) => !run.finished_at) ?? false
    const writable = canConfigure && lifecycle?.phase === "preparation" && !readError
    const validDraft =
        draft &&
        Number.isInteger(draft.interval_minutes) &&
        draft.interval_minutes >= 1 &&
        draft.interval_minutes <= 60 &&
        Number.isInteger(draft.timeout_seconds) &&
        draft.timeout_seconds >= 10 &&
        draft.timeout_seconds <= Math.min(600, draft.interval_minutes * 60 - 30)
    const perform = async (operation: () => Promise<unknown>, success: string) => {
        setBusy(true)
        try {
            await operation()
            notify(t(success), {type: "success"})
            await refresh()
        } catch {
            notify(t("securityTests.actionFailed"), {type: "error"})
        } finally {
            setBusy(false)
        }
    }
    const run = () =>
        perform(
            () =>
                client.mutate({
                    mutation: RUN_SECURITY_TESTS,
                    variables: {event, trigger},
                    context: asRole(IPermissions.SELF_TEST_RUN),
                }),
            "securityTests.queued"
        )
    const save = () =>
        perform(
            () =>
                client.mutate({
                    mutation: SAVE_SECURITY_SETTINGS,
                    variables: {event, settings: draft},
                    context: asRole(IPermissions.SELF_TEST_SETTINGS_WRITE),
                }),
            "securityTests.saved"
        )
    const finish = async () => {
        if (!auth.isFreshBoardAuthentication()) {
            const url = new URL(window.location.href)
            url.searchParams.set("tabId", "security")
            await auth.reauthenticateBoard(url.toString())
            return
        }
        setPostProcessing(false)
        await perform(
            () =>
                client.mutate({
                    mutation: START_POST_PROCESSING,
                    variables: {event},
                    context: asRole(IPermissions.ELECTION_STATE_WRITE),
                }),
            "securityTests.postProcessingStarted"
        )
    }
    if (!canRead) return <Alert severity="info">{t("securityTests.readRequired")}</Alert>
    return (
        <Stack spacing={3} sx={{p: 2, maxWidth: "85rem"}}>
            <Typography variant="h5">{t("securityTests.title")}</Typography>
            <Typography>{t("securityTests.intro")}</Typography>
            {readError && <Alert severity="error">{t("securityTests.readFailed")}</Alert>}
            {!tests && !readError && (
                <CircularProgress aria-label={t("common.label.loadingData")} />
            )}
            {lifecycle && (
                <Alert severity={lifecycle.safety === "operational" ? "info" : "warning"}>
                    <Typography sx={{fontWeight: 600}}>
                        {t(`securityTests.phase.${lifecycle.phase}`)} ·{" "}
                        {t(`securityTests.safety.${lifecycle.safety}`)}
                    </Typography>
                    {lifecycle.reason && <Typography>{lifecycle.reason}</Typography>}
                    <Typography>{t("securityTests.recovery")}</Typography>
                </Alert>
            )}
            {tests && !tests.baseline_present && (
                <Alert severity="warning">{t("securityTests.noBaseline")}</Alert>
            )}
            <Stack direction={{xs: "column", sm: "row"}} spacing={2} alignItems="center">
                <TextField
                    select
                    label={t("securityTests.purpose")}
                    value={trigger}
                    onChange={(e) => setTrigger(e.target.value)}
                    sx={{minWidth: 260}}
                    disabled={!canRun || busy || pending}
                >
                    {["requested", "before-start", "before-resume", "before-counting"].map(
                        (value) => (
                            <MenuItem key={value} value={value}>
                                {t(`securityTests.trigger.${value}`)}
                            </MenuItem>
                        )
                    )}
                </TextField>
                <Button
                    variant="contained"
                    disabled={!canRun || busy || pending || !tests || readError}
                    onClick={() => void run()}
                >
                    {t(pending ? "securityTests.running" : "securityTests.run")}
                </Button>
                <Button disabled={busy} onClick={() => void refresh()}>
                    {t("securityTests.refresh")}
                </Button>
            </Stack>
            {draft && (
                <Box component="fieldset" sx={{border: "1px solid", borderColor: "divider", p: 2}}>
                    <Typography component="legend">{t("securityTests.settings")}</Typography>
                    <Typography sx={{mb: 2}}>
                        {t(
                            writable
                                ? "securityTests.settingsHelp"
                                : "securityTests.settingsReadOnly"
                        )}
                    </Typography>
                    <Stack direction={{xs: "column", sm: "row"}} spacing={2}>
                        <TextField
                            type="number"
                            label={t("securityTests.interval")}
                            value={draft.interval_minutes}
                            disabled={!writable || busy}
                            inputProps={{min: 1, max: 60, step: 1}}
                            onChange={(e) =>
                                setDraft({...draft, interval_minutes: Number(e.target.value)})
                            }
                        />
                        <TextField
                            type="number"
                            label={t("securityTests.timeout")}
                            value={draft.timeout_seconds}
                            disabled={!writable || busy}
                            inputProps={{
                                min: 10,
                                max: Math.min(600, draft.interval_minutes * 60 - 30),
                                step: 1,
                            }}
                            onChange={(e) =>
                                setDraft({...draft, timeout_seconds: Number(e.target.value)})
                            }
                        />
                    </Stack>
                    <Stack sx={{my: 1}}>
                        {EXTRA_TRIGGERS.map((condition) => (
                            <FormControlLabel
                                key={condition}
                                label={t(`securityTests.trigger.${condition}`)}
                                control={
                                    <Checkbox
                                        checked={draft.additional_conditions.includes(condition)}
                                        disabled={!writable || busy}
                                        onChange={(_, checked) =>
                                            setDraft({
                                                ...draft,
                                                additional_conditions: checked
                                                    ? [...draft.additional_conditions, condition]
                                                    : draft.additional_conditions.filter(
                                                          (value) => value !== condition
                                                      ),
                                            })
                                        }
                                    />
                                }
                            />
                        ))}
                    </Stack>
                    <Button
                        disabled={!writable || busy || !validDraft}
                        variant="outlined"
                        onClick={() => void save()}
                    >
                        {t("securityTests.save")}
                    </Button>
                </Box>
            )}
            <Box>
                <Typography variant="h6" sx={{mb: 1}}>
                    {t("securityTests.history")}
                </Typography>
                {tests?.runs.length === 0 && (
                    <Alert severity="info">{t("securityTests.noRuns")}</Alert>
                )}
                {tests?.runs.map((item, index) => (
                    <Accordion key={item.run_id} defaultExpanded={index === 0}>
                        <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                            <Stack spacing={0.5}>
                                <Typography>
                                    {format(item.started_at)} ·{" "}
                                    {t(`securityTests.trigger.${item.trigger}`)}
                                </Typography>
                                <Typography sx={{fontWeight: 600}}>
                                    {t(
                                        `securityTests.outcome.${item.finished_at ? item.outcome : "running"}`
                                    )}
                                    {lifecycle && item.generation !== lifecycle.generation
                                        ? ` · ${t("securityTests.superseded")}`
                                        : ""}
                                </Typography>
                            </Stack>
                        </AccordionSummary>
                        <AccordionDetails>
                            <Typography variant="body2" sx={{mb: 2}}>
                                {t("securityTests.recordedOnly")}
                            </Typography>
                            <Box sx={{overflowX: "auto"}}>
                                <Table size="small" aria-label={t("securityTests.checks")}>
                                    <TableHead>
                                        <TableRow>
                                            <TableCell>{t("securityTests.check")}</TableCell>
                                            <TableCell>{t("securityTests.result")}</TableCell>
                                            <TableCell>{t("securityTests.details")}</TableCell>
                                        </TableRow>
                                    </TableHead>
                                    <TableBody>
                                        {CHECKS.map((name) => {
                                            const result = item.checks.find(
                                                (check) => check.check === name
                                            )
                                            return (
                                                <TableRow key={name}>
                                                    <TableCell>
                                                        {t(`securityTests.checkName.${name}`)}
                                                    </TableCell>
                                                    <TableCell>
                                                        {t(
                                                            `securityTests.outcome.${result?.outcome ?? (item.finished_at ? "not-completed" : "running")}`
                                                        )}
                                                    </TableCell>
                                                    <TableCell
                                                        sx={{
                                                            maxWidth: 600,
                                                            whiteSpace: "pre-wrap",
                                                            overflowWrap: "anywhere",
                                                        }}
                                                    >
                                                        {result?.detail ?? "—"}
                                                    </TableCell>
                                                </TableRow>
                                            )
                                        })}
                                    </TableBody>
                                </Table>
                            </Box>
                        </AccordionDetails>
                    </Accordion>
                ))}
            </Box>
            {!!tests?.notifications.length && (
                <Box>
                    <Typography variant="h6">{t("securityTests.notifications")}</Typography>
                    <Typography sx={{mb: 1}}>{t("securityTests.notificationHelp")}</Typography>
                    <Table size="small">
                        <TableHead>
                            <TableRow>
                                <TableCell>{t("securityTests.when")}</TableCell>
                                <TableCell>{t("securityTests.details")}</TableCell>
                                <TableCell>{t("securityTests.result")}</TableCell>
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            {tests.notifications.map((notice) => (
                                <TableRow key={notice.id}>
                                    <TableCell>{format(notice.created_at)}</TableCell>
                                    <TableCell>
                                        {notice.kind}
                                        <Typography
                                            variant="caption"
                                            component="div"
                                            sx={{fontFamily: "monospace", overflowWrap: "anywhere"}}
                                        >
                                            {notice.id}
                                        </Typography>
                                        {notice.failure && (
                                            <Typography variant="body2">
                                                {notice.failure}
                                            </Typography>
                                        )}
                                    </TableCell>
                                    <TableCell>
                                        {t(`securityTests.delivery.${notice.state}`)}
                                    </TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </Box>
            )}
            {canPostProcess && lifecycle?.phase === "evaluation" && (
                <Button
                    variant="outlined"
                    disabled={busy || readError}
                    onClick={() => setPostProcessing(true)}
                >
                    {t("securityTests.startPostProcessing")}
                </Button>
            )}
            <Dialog open={postProcessing} onClose={() => !busy && setPostProcessing(false)}>
                <DialogTitle>{t("securityTests.startPostProcessing")}</DialogTitle>
                <DialogContent>{t("securityTests.postProcessingHelp")}</DialogContent>
                <DialogActions>
                    <Button disabled={busy} onClick={() => setPostProcessing(false)}>
                        {t("securityTests.cancel")}
                    </Button>
                    <Button disabled={busy} onClick={() => void finish()}>
                        {t("securityTests.confirm")}
                    </Button>
                </DialogActions>
            </Dialog>
        </Stack>
    )
}
