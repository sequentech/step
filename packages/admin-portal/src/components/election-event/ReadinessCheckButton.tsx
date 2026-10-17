// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useState} from "react"
import {
    Alert,
    Autocomplete,
    Box,
    Button,
    Chip,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    List,
    ListItem,
    Stack,
    TextField,
    Typography,
} from "@mui/material"
import FactCheckOutlinedIcon from "@mui/icons-material/FactCheckOutlined"
import {useMutation, useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {useRefresh} from "react-admin"
import {downloadBlob} from "@sequentech/ui-core"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {IPermissions} from "@/types/keycloak"
import {GET_READINESS_EVENTS, READINESS_CHECK} from "@/queries/ReadinessCheck"
import type {
    GetReadinessEventsQuery,
    GetReadinessEventsQueryVariables,
    ReadinessCheckMutationVariables,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {
    readinessReport,
    READINESS_CHECK_KINDS,
    type IReadinessReport,
} from "@/services/readinessReport"

type Event = Pick<Sequent_Backend_Election_Event, "id" | "presentation">
type TestedEvent = GetReadinessEventsQuery["sequent_backend_election_event"][number]

export function ReadinessDialog({
    event,
    tenantId,
    onClose,
}: {
    event: Event
    tenantId: string
    onClose: () => void
}) {
    const {t, i18n} = useTranslation()
    const alias = useAliasRenderer()
    const refresh = useRefresh()
    const [tested, setTested] = useState<TestedEvent | null>(null)
    const [report, setReport] = useState<IReadinessReport | null>(null)
    const [pending, setPending] = useState(false)
    const [failure, setFailure] = useState<string | null>(null)
    const {data, loading, error, refetch} = useQuery<
        GetReadinessEventsQuery,
        GetReadinessEventsQueryVariables
    >(GET_READINESS_EVENTS, {
        variables: {tenantId, officialEventId: event.id},
        context: {headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_READ}},
        fetchPolicy: "network-only",
    })
    const [check] = useMutation<{readiness_check: unknown}, ReadinessCheckMutationVariables>(
        READINESS_CHECK
    )
    const eventLabel = (option: TestedEvent) =>
        `${alias(option)} · ${option.id}${option.is_archived ? ` · ${t("readiness.archived")}` : ""}`
    const collator = new Intl.Collator(i18n.language, {numeric: true, sensitivity: "base"})
    const options = (data?.sequent_backend_election_event ?? [])
        .filter((option) => option.id !== event.id)
        .sort(
            (left, right) =>
                collator.compare(alias(left), alias(right)) ||
                String(left.id).localeCompare(String(right.id))
        )

    const run = async () => {
        if (!tested || tested.id === event.id || pending || loading || error) return
        setPending(true)
        setReport(null)
        setFailure(null)
        try {
            const response = await check({
                variables: {official_event_id: event.id, tested_event_id: tested.id},
                context: {headers: {"x-hasura-role": IPermissions.READINESS_CHECK}},
            })
            const result = readinessReport(response.data?.readiness_check, event.id, tested.id)
            if (response.errors?.length || !result) {
                setFailure(t("readiness.error"))
                return
            }
            setReport(result)
            if (result.passed) refresh()
        } catch {
            setFailure(t("readiness.error"))
        } finally {
            setPending(false)
        }
    }
    const download = async () => {
        if (!report) return
        try {
            await downloadBlob(
                new Blob([JSON.stringify(report, null, 2)], {type: "application/json"}),
                `readiness-${event.id}.json`
            )
        } catch {
            setFailure(t("readiness.downloadError"))
        }
    }
    const retryEvents = async () => {
        try {
            await refetch()
        } catch {
            /* The query's error state supplies the retry message. */
        }
    }

    return (
        <Dialog
            open
            fullWidth
            maxWidth="md"
            onClose={pending ? undefined : onClose}
            aria-labelledby="readiness-title"
        >
            <DialogTitle id="readiness-title">{t("readiness.title")}</DialogTitle>
            <DialogContent dividers>
                <Stack spacing={2}>
                    <Typography>{t("readiness.description")}</Typography>
                    <Box sx={{overflowWrap: "anywhere"}}>
                        <Typography variant="subtitle2" component="p">
                            {t("logsScreen.readiness.fields.official")}
                        </Typography>
                        <Typography>{alias(event)}</Typography>
                        <Typography variant="body2">{event.id}</Typography>
                    </Box>
                    <Autocomplete
                        options={options}
                        value={tested}
                        loading={loading}
                        disabled={pending || loading || !!error}
                        isOptionEqualToValue={(left, right) => left.id === right.id}
                        getOptionLabel={eventLabel}
                        onChange={(_event, value) => {
                            setTested(value)
                            setReport(null)
                            setFailure(null)
                        }}
                        noOptionsText={t("readiness.noEvents")}
                        loadingText={t("common.label.loadingData")}
                        renderInput={(params) => (
                            <TextField {...params} label={t("readiness.testedEvent")} />
                        )}
                        renderOption={(props, option) => (
                            <li {...props} key={option.id}>
                                <Typography sx={{overflowWrap: "anywhere"}}>
                                    {eventLabel(option)}
                                </Typography>
                            </li>
                        )}
                    />
                    {error && (
                        <Alert
                            severity="error"
                            action={<Button onClick={retryEvents}>{t("readiness.retry")}</Button>}
                        >
                            {t("readiness.eventsError")}
                        </Alert>
                    )}
                    {failure && <Alert severity="error">{failure}</Alert>}
                    {pending && (
                        <Stack direction="row" spacing={1} alignItems="center" role="status">
                            <CircularProgress size={20} aria-label={t("readiness.running")} />
                            <Typography>{t("readiness.running")}</Typography>
                        </Stack>
                    )}
                    {report && (
                        <>
                            <Alert severity={report.passed ? "success" : "warning"}>
                                {t(report.passed ? "readiness.success" : "readiness.failed")}
                            </Alert>
                            <Typography>
                                {t("logsScreen.readiness.fields.checked_at")}: {report.checked_at}
                            </Typography>
                            <List disablePadding aria-label={t("readiness.results")}>
                                {report.checks.map((check) => (
                                    <ListItem
                                        key={check.kind}
                                        disableGutters
                                        sx={{gap: 1, alignItems: "flex-start"}}
                                    >
                                        <Chip
                                            size="small"
                                            color={check.passed ? "success" : "error"}
                                            label={t(
                                                check.passed
                                                    ? "logsScreen.readiness.passed"
                                                    : "logsScreen.readiness.failed"
                                            )}
                                        />
                                        <Typography>
                                            {READINESS_CHECK_KINDS.some(
                                                (kind) => kind === check.kind
                                            )
                                                ? t(`logsScreen.readiness.checks.${check.kind}`)
                                                : check.message}
                                        </Typography>
                                    </ListItem>
                                ))}
                            </List>
                            {report.log_entry_id && (
                                <Box sx={{overflowWrap: "anywhere"}}>
                                    <Typography variant="subtitle2" component="p">
                                        {t("logsScreen.readiness.fields.log_entry_id")}
                                    </Typography>
                                    <Typography>{report.log_entry_id}</Typography>
                                </Box>
                            )}
                            <Typography variant="body2">{t("readiness.snapshot")}</Typography>
                        </>
                    )}
                </Stack>
            </DialogContent>
            <DialogActions sx={{flexWrap: "wrap", gap: 1}}>
                <Button disabled={pending} onClick={onClose}>
                    {t("readiness.close")}
                </Button>
                {report && <Button onClick={download}>{t("readiness.download")}</Button>}
                <Button
                    variant="contained"
                    disabled={!tested || pending || loading || !!error}
                    onClick={run}
                >
                    {t(pending ? "readiness.running" : "readiness.run")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}

/** The server checks both events' access and signs only a pass. */
export function ReadinessCheckButton({event}: {event: Event}) {
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const [open, setOpen] = useState(false)
    if (!tenantId || !auth.isAuthorized(true, tenantId, IPermissions.READINESS_CHECK)) return null
    return (
        <>
            <Button
                variant="text"
                size="small"
                startIcon={<FactCheckOutlinedIcon />}
                onClick={() => setOpen(true)}
            >
                {t("readiness.action")}
            </Button>
            {open && (
                <ReadinessDialog
                    key={`${tenantId}:${event.id}`}
                    event={event}
                    tenantId={tenantId}
                    onClose={() => setOpen(false)}
                />
            )}
        </>
    )
}
