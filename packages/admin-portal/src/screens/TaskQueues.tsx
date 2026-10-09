// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState} from "react"
import {ApolloError, useApolloClient, useMutation, useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Card,
    CardContent,
    Chip,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
    FormControl,
    FormControlLabel,
    InputLabel,
    MenuItem,
    Select,
    Stack,
    Switch,
    Tab,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    Tabs,
    TextField,
    Typography,
} from "@mui/material"
import {DataGrid, GridColDef, GridRowSelectionModel} from "@mui/x-data-grid"
import Chart from "react-apexcharts"
import type {ApexOptions} from "apexcharts"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {
    TASK_QUEUES_DEAD_LETTERS,
    TASK_QUEUES_MESSAGES,
    TASK_QUEUES_OVERVIEW,
    TASK_QUEUES_QUERY,
    TASK_QUEUES_THROUGHPUT,
} from "@/queries/TaskQueues"
import {
    columnField,
    ConsoleQueryResult,
    ConsoleQueryRows,
    ConsoleRows,
    displayValue,
    isQueryError,
    ROW_ID,
    toGridRows,
} from "@/services/ElectoralLogConsole"
import {
    ChartSeries,
    countTicks,
    DeadLetterOperation,
    DeadLettersResult,
    durationSeries,
    eventLabel,
    formatAge,
    formatBytes,
    GRAPH_PERIODS,
    GRAPH_RANGES,
    GraphPeriod,
    hasValues,
    isOutcome,
    managesDeadLetters,
    MESSAGE_PAGE_SIZE,
    MESSAGE_STATES,
    MessageState,
    MessagesResult,
    MessageSummary,
    nextBefore,
    OUTCOME_HEADER,
    orderedOutcomes,
    outcomeColor,
    outcomeSeries,
    OverviewResult,
    processedLastHour,
    selectedMessageIds,
    ThroughputResult,
} from "@/services/TaskQueues"

const OVERVIEW_POLL_MS = 5_000
const GRAPH_POLL_MS = 30_000
// The worker applies a replay or discard shortly after Harvest queues it.
const REFRESH_AFTER_OPERATION_MS = 2_000
const GRID_HEIGHT = 560
const CHART_HEIGHT = 280
const ERROR_HEADER = "x-electoral-log-error"

const readHeaders = {headers: {"x-hasura-role": IPermissions.TASK_QUEUES_READ}}
const writeHeaders = {headers: {"x-hasura-role": IPermissions.TASK_QUEUES_WRITE}}
const queryHeaders = {headers: {"x-hasura-role": IPermissions.TASK_QUEUES_QUERY}}

const DEFAULT_QUERY = `SELECT headers->>'x-step-outcome' AS outcome, count(*) AS messages
FROM pgmq.a_short_queue
WHERE archived_at > now() - interval '1 hour'
GROUP BY 1
ORDER BY 2 DESC`

type PageTab = "queues" | "query"

const NO_SELECTION: GridRowSelectionModel = {type: "include", ids: new Set()}

const useOutcomeLabel = () => {
    const {t} = useTranslation()
    return (outcome: string) => (isOutcome(outcome) ? t(`taskQueues.outcomes.${outcome}`) : outcome)
}

const OutcomeChips: React.FC<{outcomes: Record<string, number>}> = ({outcomes}) => {
    const label = useOutcomeLabel()
    const entries = orderedOutcomes(outcomes)
    if (entries.length === 0) {
        return <>—</>
    }
    return (
        <Stack direction="row" spacing={0.5} flexWrap="wrap" useFlexGap>
            {entries.map(([outcome, count]) => (
                <Chip
                    key={outcome}
                    size="small"
                    label={`${label(outcome)} ${count.toLocaleString()}`}
                    sx={{backgroundColor: outcomeColor(outcome), color: "#fff"}}
                />
            ))}
        </Stack>
    )
}

const Overview: React.FC<{
    result: OverviewResult | undefined
    selected: string | null
    onSelect: (queue: string) => void
}> = ({result, selected, onSelect}) => {
    const {t} = useTranslation()
    return (
        <Box sx={{overflowX: "auto"}}>
            <Table size="small">
                <TableHead>
                    <TableRow>
                        <TableCell>{t("taskQueues.columns.queue")}</TableCell>
                        <TableCell align="right">{t("taskQueues.columns.ready")}</TableCell>
                        <TableCell align="right">
                            {t("taskQueues.columns.runningOrScheduled")}
                        </TableCell>
                        <TableCell align="right">{t("taskQueues.columns.oldest")}</TableCell>
                        <TableCell align="right">{t("taskQueues.columns.processed")}</TableCell>
                        <TableCell>{t("taskQueues.columns.lastHour")}</TableCell>
                        <TableCell align="right">{t("taskQueues.columns.sent")}</TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {(result?.queues ?? []).map((queue) => (
                        <TableRow
                            key={queue.queue}
                            hover
                            selected={queue.queue === selected}
                            onClick={() => onSelect(queue.queue)}
                            sx={{cursor: "pointer"}}
                        >
                            <TableCell sx={{fontFamily: "monospace"}}>{queue.queue}</TableCell>
                            <TableCell align="right">{queue.ready.toLocaleString()}</TableCell>
                            <TableCell align="right">
                                {queue.running_or_scheduled.toLocaleString()}
                            </TableCell>
                            <TableCell align="right">{formatAge(queue.oldest_age_secs)}</TableCell>
                            <TableCell align="right">
                                {processedLastHour(queue).toLocaleString()}
                            </TableCell>
                            <TableCell>
                                <OutcomeChips outcomes={queue.last_hour} />
                            </TableCell>
                            <TableCell align="right">{queue.total_sent.toLocaleString()}</TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </Box>
    )
}

const chartOptions = (base: ApexOptions, locale: string): ApexOptions => ({
    ...base,
    chart: {
        ...base.chart,
        toolbar: {show: false},
        zoom: {enabled: false},
        animations: {enabled: false},
    },
    dataLabels: {enabled: false},
    legend: {position: "bottom"},
    xaxis: {
        type: "datetime",
        labels: {
            datetimeUTC: false,
            formatter: (value: string) =>
                new Date(Number(value)).toLocaleTimeString(locale, {
                    hour: "2-digit",
                    minute: "2-digit",
                }),
        },
    },
    tooltip: {
        x: {
            formatter: (value: number) => new Date(value).toLocaleString(locale),
        },
    },
})

const Graphs: React.FC<{queue: string; live: boolean}> = ({queue, live}) => {
    const {t, i18n} = useTranslation()
    const outcomeLabel = useOutcomeLabel()
    const locale = i18n.resolvedLanguage ?? i18n.language
    const [period, setPeriod] = useState<GraphPeriod>("day")
    const range = GRAPH_RANGES[period]
    const {data, error, loading} = useQuery(TASK_QUEUES_THROUGHPUT, {
        variables: {queue, hours: range.hours, bucketMinutes: range.bucketMinutes},
        fetchPolicy: "network-only",
        pollInterval: live ? GRAPH_POLL_MS : 0,
        context: readHeaders,
    })
    const result = data?.task_queues_throughput as ThroughputResult | undefined
    const now = useMemo(() => new Date(), [result])
    const outcomes: ChartSeries[] = result ? outcomeSeries(result, now) : []
    const durations = result ? durationSeries(result, now) : null
    const showsDurations =
        durations !== null && (hasValues(durations.wait) || hasValues(durations.processing))

    return (
        <Stack spacing={2}>
            <Stack direction="row" spacing={2} alignItems="center" justifyContent="space-between">
                <Typography variant="h6">{t("taskQueues.graphs.title")}</Typography>
                <FormControl size="small" sx={{minWidth: 160}}>
                    <InputLabel id="task-queues-period">{t("taskQueues.graphs.period")}</InputLabel>
                    <Select
                        labelId="task-queues-period"
                        label={t("taskQueues.graphs.period")}
                        value={period}
                        onChange={(e) => setPeriod(e.target.value as GraphPeriod)}
                    >
                        {GRAPH_PERIODS.map((name) => (
                            <MenuItem key={name} value={name}>
                                {t(`taskQueues.graphs.periods.${name}`)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
            </Stack>
            {error && (
                <Alert severity="error">
                    {getGraphQLActionErrorReason(error) ?? t("taskQueues.error")}
                </Alert>
            )}
            {result && outcomes.length === 0 && !loading && (
                <Alert severity="info">{t("taskQueues.graphs.empty")}</Alert>
            )}
            {result && outcomes.length > 0 && durations && (
                <Box
                    sx={{
                        display: "grid",
                        gridTemplateColumns: {xs: "1fr", lg: showsDurations ? "1fr 1fr" : "1fr"},
                        gap: 2,
                    }}
                >
                    <Box>
                        <Typography variant="subtitle2">
                            {t("taskQueues.graphs.outcomes")}
                        </Typography>
                        <Chart
                            type="bar"
                            height={CHART_HEIGHT}
                            series={outcomes.map((series) => ({
                                ...series,
                                name: outcomeLabel(series.name),
                            }))}
                            options={chartOptions(
                                {
                                    chart: {stacked: true},
                                    colors: outcomes.map((series) => outcomeColor(series.name)),
                                    plotOptions: {bar: {columnWidth: "90%"}},
                                    yaxis: {
                                        min: 0,
                                        tickAmount: countTicks(outcomes),
                                        forceNiceScale: true,
                                        labels: {formatter: (value) => String(Math.round(value))},
                                    },
                                },
                                locale
                            )}
                        />
                    </Box>
                    {showsDurations && (
                        <Box>
                            <Typography variant="subtitle2">
                                {t("taskQueues.graphs.durations")}
                            </Typography>
                            <Chart
                                type="line"
                                height={CHART_HEIGHT}
                                series={[
                                    {name: t("taskQueues.graphs.wait"), data: durations.wait},
                                    {
                                        name: t("taskQueues.graphs.processing"),
                                        data: durations.processing,
                                    },
                                ]}
                                options={chartOptions(
                                    {
                                        stroke: {width: 2},
                                        markers: {size: 3},
                                        yaxis: {
                                            title: {text: t("taskQueues.graphs.seconds")},
                                            labels: {
                                                formatter: (value) =>
                                                    value === null
                                                        ? ""
                                                        : value.toLocaleString(locale),
                                            },
                                        },
                                    },
                                    locale
                                )}
                            />
                        </Box>
                    )}
                </Box>
            )}
        </Stack>
    )
}

const ConfirmDialog: React.FC<{
    operation: DeadLetterOperation | null
    count: number
    onCancel: () => void
    onConfirm: () => void
}> = ({operation, count, onCancel, onConfirm}) => {
    const {t} = useTranslation()
    return (
        <Dialog open={operation !== null} onClose={onCancel}>
            {operation && (
                <>
                    <DialogTitle>
                        {t(`taskQueues.deadLetters.confirmTitle.${operation}`)}
                    </DialogTitle>
                    <DialogContent>
                        <DialogContentText>
                            {t(`taskQueues.deadLetters.confirmBody.${operation}`, {
                                count,
                            })}
                        </DialogContentText>
                    </DialogContent>
                    <DialogActions>
                        <Button onClick={onCancel}>{t("taskQueues.deadLetters.cancel")}</Button>
                        <Button
                            variant="contained"
                            color={operation === "discard" ? "error" : "primary"}
                            onClick={onConfirm}
                        >
                            {t(`taskQueues.deadLetters.${operation}`)}
                        </Button>
                    </DialogActions>
                </>
            )}
        </Dialog>
    )
}

const Messages: React.FC<{queue: string; canWrite: boolean}> = ({queue, canWrite}) => {
    const {t, i18n} = useTranslation()
    const outcomeLabel = useOutcomeLabel()
    const locale = i18n.resolvedLanguage ?? i18n.language
    const [state, setState] = useState<MessageState>("queued")
    // The `before` cursor of each page shown so far; the last is the current page.
    const [cursors, setCursors] = useState<Array<string | null>>([null])
    const [selection, setSelection] = useState<GridRowSelectionModel>(NO_SELECTION)
    const [operation, setOperation] = useState<DeadLetterOperation | null>(null)
    const [notice, setNotice] = useState<string | null>(null)
    const [failure, setFailure] = useState<string | null>(null)
    const before = cursors[cursors.length - 1]

    useEffect(() => {
        setCursors([null])
        setSelection(NO_SELECTION)
        setNotice(null)
        setFailure(null)
    }, [queue, state])

    const {data, error, loading, refetch} = useQuery(TASK_QUEUES_MESSAGES, {
        variables: {queue, state, before, limit: MESSAGE_PAGE_SIZE},
        fetchPolicy: "network-only",
        context: readHeaders,
    })
    const [applyOperation, {loading: applying}] = useMutation(TASK_QUEUES_DEAD_LETTERS, {
        context: writeHeaders,
    })
    const messages = (data?.task_queues_messages as MessagesResult | undefined)?.messages ?? []
    const manages = managesDeadLetters(queue, state)
    const selected = selectedMessageIds(selection, messages)
    const older = nextBefore(messages, MESSAGE_PAGE_SIZE)
    const showsEvents = messages.some((message) => Object.keys(message.event).length > 0)
    const showsErrors = messages.some((message) => message.headers[ERROR_HEADER])

    const refresh = async () => {
        setSelection(NO_SELECTION)
        try {
            await refetch()
        } catch (refreshFailure) {
            setFailure(getGraphQLActionErrorReason(refreshFailure) ?? t("taskQueues.error"))
        }
    }

    const confirm = async () => {
        if (!operation) {
            return
        }
        const ids = selected
        setOperation(null)
        setNotice(null)
        setFailure(null)
        try {
            const {data: answer} = await applyOperation({
                variables: {operation, messageIds: ids.map(String)},
            })
            const result = answer?.task_queues_dead_letters as DeadLettersResult | undefined
            setNotice(
                t(`taskQueues.deadLetters.queued.${operation}`, {
                    count: result?.requested ?? ids.length,
                    taskId: result?.task_id ?? "",
                })
            )
            setSelection(NO_SELECTION)
            setTimeout(() => {
                refresh()
            }, REFRESH_AFTER_OPERATION_MS)
        } catch (operationFailure) {
            setFailure(
                getGraphQLActionErrorReason(operationFailure) ?? t("taskQueues.deadLetters.failed")
            )
        }
    }

    const date = (value: string | null) => (value ? new Date(value).toLocaleString(locale) : "—")
    const columns: GridColDef<MessageSummary>[] = [
        {field: "msg_id", headerName: t("taskQueues.messages.columns.id"), width: 90},
        {
            field: "task",
            headerName: t("taskQueues.messages.columns.task"),
            minWidth: 240,
            flex: 1,
            valueGetter: (_, row) => row.task ?? t("taskQueues.messages.unreadable"),
        },
        ...(state === "archived"
            ? [
                  {
                      field: "outcome",
                      headerName: t("taskQueues.messages.columns.outcome"),
                      width: 130,
                      valueGetter: (_: unknown, row: MessageSummary) =>
                          row.headers[OUTCOME_HEADER] ?? "unknown",
                      renderCell: ({value}: {value?: string}) => (
                          <Chip
                              size="small"
                              label={outcomeLabel(value ?? "unknown")}
                              sx={{
                                  backgroundColor: outcomeColor(value ?? "unknown"),
                                  color: "#fff",
                              }}
                          />
                      ),
                  } as GridColDef<MessageSummary>,
              ]
            : []),
        ...(showsEvents
            ? [
                  {
                      field: "event",
                      headerName: t("taskQueues.messages.columns.event"),
                      minWidth: 260,
                      flex: 1,
                      valueGetter: (_: unknown, row: MessageSummary) => eventLabel(row.event),
                  } as GridColDef<MessageSummary>,
              ]
            : []),
        ...(showsErrors
            ? [
                  {
                      field: "error",
                      headerName: t("taskQueues.messages.columns.error"),
                      minWidth: 240,
                      flex: 1,
                      valueGetter: (_: unknown, row: MessageSummary) =>
                          row.headers[ERROR_HEADER] ?? "",
                      renderCell: ({value}: {value?: string}) => <span title={value}>{value}</span>,
                  } as GridColDef<MessageSummary>,
              ]
            : []),
        {
            field: "enqueued_at",
            headerName: t("taskQueues.messages.columns.enqueued"),
            width: 180,
            valueFormatter: (value: string) => date(value),
        },
        {
            field: "read_count",
            headerName: t("taskQueues.messages.columns.reads"),
            width: 80,
            type: "number",
        },
        {
            field: "retries",
            headerName: t("taskQueues.messages.columns.retries"),
            width: 80,
            type: "number",
        },
        state === "archived"
            ? {
                  field: "archived_at",
                  headerName: t("taskQueues.messages.columns.archived"),
                  width: 180,
                  valueFormatter: (value: string | null) => date(value),
              }
            : {
                  field: "visible_at",
                  headerName: t("taskQueues.messages.columns.visible"),
                  width: 180,
                  valueFormatter: (value: string) => date(value),
              },
        {
            field: "size_bytes",
            headerName: t("taskQueues.messages.columns.size"),
            width: 100,
            type: "number",
            valueFormatter: (value: number) => formatBytes(value),
        },
        {
            field: "task_id",
            headerName: t("taskQueues.messages.columns.taskId"),
            minWidth: 290,
        },
    ]

    return (
        <Stack spacing={2}>
            <Stack
                direction={{xs: "column", sm: "row"}}
                spacing={2}
                alignItems={{sm: "center"}}
                justifyContent="space-between"
            >
                <Typography variant="h6">{t("taskQueues.messages.title")}</Typography>
                <Tabs value={state} onChange={(_, value: MessageState) => setState(value)}>
                    {MESSAGE_STATES.map((name) => (
                        <Tab
                            key={name}
                            value={name}
                            label={t(`taskQueues.messages.states.${name}`)}
                        />
                    ))}
                </Tabs>
            </Stack>
            <Typography variant="body2" color="text.secondary">
                {t("taskQueues.messages.argumentsHidden")}
            </Typography>
            {manages && (
                <Alert severity="info">
                    {canWrite
                        ? t("taskQueues.deadLetters.help")
                        : t("taskQueues.deadLetters.noWrite")}
                </Alert>
            )}
            {notice && (
                <Alert severity="success" onClose={() => setNotice(null)}>
                    {notice}
                </Alert>
            )}
            {(failure || error) && (
                <Alert severity="error">
                    {failure ?? getGraphQLActionErrorReason(error) ?? t("taskQueues.error")}
                </Alert>
            )}
            <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap>
                <Button onClick={refresh}>{t("taskQueues.messages.refresh")}</Button>
                <Button disabled={cursors.length === 1} onClick={() => setCursors([null])}>
                    {t("taskQueues.messages.newest")}
                </Button>
                <Button
                    disabled={cursors.length === 1}
                    onClick={() => setCursors(cursors.slice(0, -1))}
                >
                    {t("taskQueues.messages.newer")}
                </Button>
                <Button disabled={!older} onClick={() => setCursors([...cursors, older])}>
                    {t("taskQueues.messages.older")}
                </Button>
                {manages && canWrite && (
                    <>
                        <Button
                            variant="contained"
                            disabled={selected.length === 0 || applying}
                            onClick={() => setOperation("replay")}
                        >
                            {t("taskQueues.deadLetters.replay")}
                        </Button>
                        <Button
                            variant="outlined"
                            color="error"
                            disabled={selected.length === 0 || applying}
                            onClick={() => setOperation("discard")}
                        >
                            {t("taskQueues.deadLetters.discard")}
                        </Button>
                    </>
                )}
            </Stack>
            <Box sx={{height: GRID_HEIGHT, width: "100%"}}>
                <DataGrid
                    rows={messages}
                    columns={columns}
                    getRowId={(row) => row.msg_id}
                    loading={loading}
                    checkboxSelection={manages && canWrite}
                    rowSelectionModel={selection}
                    onRowSelectionModelChange={setSelection}
                    disableRowSelectionOnClick
                    hideFooterPagination
                    initialState={{columns: {columnVisibilityModel: {task_id: false}}}}
                    localeText={{noRowsLabel: t("taskQueues.messages.empty")}}
                />
            </Box>
            <ConfirmDialog
                operation={operation}
                count={selected.length}
                onCancel={() => setOperation(null)}
                onConfirm={confirm}
            />
        </Stack>
    )
}

const queryColumns = ({columns}: ConsoleRows): GridColDef[] =>
    columns.map((column, index) => ({
        field: columnField(index),
        headerName: column,
        minWidth: 120,
        flex: 1,
        sortable: false,
        valueFormatter: (value: unknown) => displayValue(value),
    }))

const QueryPanel: React.FC = () => {
    const {t} = useTranslation()
    const client = useApolloClient()
    const [sql, setSql] = useState(DEFAULT_QUERY)
    const [result, setResult] = useState<ConsoleQueryRows | null>(null)
    const [error, setError] = useState<string | null>(null)
    const [running, setRunning] = useState(false)

    const run = async () => {
        if (running || !sql.trim()) {
            return
        }
        setRunning(true)
        setError(null)
        try {
            const {data} = await client.query({
                query: TASK_QUEUES_QUERY,
                variables: {sql},
                fetchPolicy: "no-cache",
                context: queryHeaders,
            })
            const answer = data?.task_queues_query as ConsoleQueryResult
            if (isQueryError(answer)) {
                setResult(null)
                setError(answer.error)
            } else {
                setResult(answer)
            }
        } catch (failure) {
            setResult(null)
            setError(getGraphQLActionErrorReason(failure) ?? t("taskQueues.query.error"))
        } finally {
            setRunning(false)
        }
    }

    const columns = useMemo(() => (result ? queryColumns(result) : []), [result])
    const rows = useMemo(() => (result ? toGridRows(result) : []), [result])

    return (
        <Stack spacing={2}>
            <Typography variant="body2" color="text.secondary">
                {t("taskQueues.query.help")}
            </Typography>
            <Alert severity="warning">{t("taskQueues.query.arguments")}</Alert>
            <TextField
                multiline
                minRows={6}
                fullWidth
                value={sql}
                placeholder={t("taskQueues.query.placeholder")}
                onChange={(e) => setSql(e.target.value)}
                onKeyDown={(e) => {
                    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
                        run()
                    }
                }}
                inputProps={{spellCheck: false, style: {fontFamily: "monospace", fontSize: 13}}}
            />
            <Stack direction="row" spacing={2} alignItems="center" justifyContent="space-between">
                <Typography variant="caption" color="text.secondary">
                    {t("taskQueues.query.limits")}
                </Typography>
                <Button variant="contained" onClick={run} disabled={running || !sql.trim()}>
                    {t("taskQueues.query.run")}
                </Button>
            </Stack>
            {error && (
                <Alert severity="error" sx={{whiteSpace: "pre-wrap", fontFamily: "monospace"}}>
                    {error}
                </Alert>
            )}
            {result && (
                <>
                    <Typography variant="body2" color="text.secondary">
                        {t("taskQueues.query.summary", {
                            rows: result.rows.length.toLocaleString(),
                            ms: result.elapsed_ms.toLocaleString(),
                        })}
                    </Typography>
                    {result.truncated && (
                        <Alert severity="info">
                            {t("taskQueues.query.truncated", {
                                rows: result.rows.length.toLocaleString(),
                            })}
                        </Alert>
                    )}
                    <Box sx={{height: GRID_HEIGHT, width: "100%"}}>
                        <DataGrid
                            rows={rows}
                            columns={columns}
                            getRowId={(row) => row[ROW_ID] as number}
                            loading={running}
                            initialState={{pagination: {paginationModel: {pageSize: 25}}}}
                            pageSizeOptions={[25, 50, 100]}
                            disableRowSelectionOnClick
                            showToolbar
                            slotProps={{
                                toolbar: {
                                    showQuickFilter: false,
                                    csvOptions: {fileName: "task-queues-query"},
                                    printOptions: {disableToolbarButton: true},
                                },
                            }}
                        />
                    </Box>
                </>
            )}
        </Stack>
    )
}

export const TaskQueues: React.FC = () => {
    const {t, i18n} = useTranslation()
    const authContext = useContext(AuthContext)
    const locale = i18n.resolvedLanguage ?? i18n.language
    const canRead = authContext.isAuthorized(true, null, IPermissions.TASK_QUEUES_READ)
    const canWrite = authContext.isAuthorized(true, null, IPermissions.TASK_QUEUES_WRITE)
    const canQuery = authContext.isAuthorized(true, null, IPermissions.TASK_QUEUES_QUERY)
    const [tab, setTab] = useState<PageTab>("queues")
    const [live, setLive] = useState(true)
    const [queue, setQueue] = useState<string | null>(null)
    const {data, error} = useQuery(TASK_QUEUES_OVERVIEW, {
        fetchPolicy: "network-only",
        pollInterval: live && tab === "queues" ? OVERVIEW_POLL_MS : 0,
        skip: !canRead || tab !== "queues",
        context: readHeaders,
    })
    const overview = data?.task_queues_overview as OverviewResult | undefined
    const updatedAt = useMemo(() => (overview ? new Date() : null), [overview])

    useEffect(() => {
        if (!queue && overview && overview.queues.length > 0) {
            setQueue(overview.queues[0].queue)
        }
    }, [queue, overview])

    if (!canRead) {
        return <Alert severity="warning">{t("taskQueues.notAllowed")}</Alert>
    }
    return (
        // The page takes the width the layout gives it, whatever its tables' width.
        <Card sx={{contain: "inline-size"}}>
            <CardContent>
                <Stack spacing={3}>
                    <Box>
                        <Typography variant="h4" sx={{mb: 1}}>
                            {t("taskQueues.title")}
                        </Typography>
                        <Typography variant="body2" color="text.secondary">
                            {t("taskQueues.subtitle")}
                        </Typography>
                    </Box>
                    {canQuery && (
                        <Tabs value={tab} onChange={(_, value: PageTab) => setTab(value)}>
                            <Tab value="queues" label={t("taskQueues.tabs.queues")} />
                            <Tab value="query" label={t("taskQueues.tabs.query")} />
                        </Tabs>
                    )}
                    {tab === "query" && canQuery ? (
                        <QueryPanel />
                    ) : (
                        <QueuesPanel
                            live={live}
                            setLive={setLive}
                            updatedAt={updatedAt}
                            locale={locale}
                            error={error}
                            overview={overview}
                            queue={queue}
                            setQueue={setQueue}
                            canWrite={canWrite}
                        />
                    )}
                </Stack>
            </CardContent>
        </Card>
    )
}

const QueuesPanel: React.FC<{
    live: boolean
    setLive: (live: boolean) => void
    updatedAt: Date | null
    locale: string
    error: ApolloError | undefined
    overview: OverviewResult | undefined
    queue: string | null
    setQueue: (queue: string) => void
    canWrite: boolean
}> = ({live, setLive, updatedAt, locale, error, overview, queue, setQueue, canWrite}) => {
    const {t} = useTranslation()
    return (
        <Stack spacing={3}>
            <Stack direction="row" spacing={2} alignItems="center" justifyContent="space-between">
                <FormControlLabel
                    control={<Switch checked={live} onChange={(e) => setLive(e.target.checked)} />}
                    label={t("taskQueues.live")}
                />
                {updatedAt && (
                    <Typography variant="caption" color="text.secondary">
                        {t("taskQueues.updated", {
                            time: updatedAt.toLocaleTimeString(locale),
                        })}
                    </Typography>
                )}
            </Stack>
            {error && (
                <Alert severity="error">
                    {getGraphQLActionErrorReason(error) ?? t("taskQueues.error")}
                </Alert>
            )}
            <Overview result={overview} selected={queue} onSelect={setQueue} />
            {queue && (
                <>
                    <Typography variant="h5" sx={{fontFamily: "monospace"}}>
                        {queue}
                    </Typography>
                    <Graphs queue={queue} live={live} />
                    <Messages queue={queue} canWrite={canWrite} />
                </>
            )}
        </Stack>
    )
}
