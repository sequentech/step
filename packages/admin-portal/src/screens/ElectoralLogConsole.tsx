// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useRef, useState} from "react"
import {useApolloClient, useQuery} from "@apollo/client"
import {useGetList} from "react-admin"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Card,
    CardContent,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    FormControl,
    IconButton,
    InputLabel,
    MenuItem,
    Select,
    Stack,
    Tab,
    Table,
    TableBody,
    TableCell,
    TableRow,
    Tabs,
    TextField,
    Typography,
} from "@mui/material"
import VisibilityIcon from "@mui/icons-material/Visibility"
import {DataGrid, GridColDef, GridPaginationModel} from "@mui/x-data-grid"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {Sequent_Backend_Election, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {
    ELECTORAL_LOG_CONSOLE_PAGE,
    ELECTORAL_LOG_CONSOLE_QUERY,
    ELECTORAL_LOG_CONSOLE_RECORD,
    ELECTORAL_LOG_CONSOLE_TENANTS,
} from "@/queries/ElectoralLogConsole"
import {
    BALLOT_STATUSES,
    CONSOLE_TABLES,
    ConsoleElection,
    ConsolePage,
    ConsoleQueryResult,
    ConsoleQueryRows,
    ConsoleRows,
    ConsoleTable,
    ConsoleTenants,
    eventElections,
    FILTERS_BY_TABLE,
    FilterField,
    FilterValues,
    PAGE_ORDERS,
    PageOrder,
    ROW_ID,
    columnField,
    displayValue,
    invalidFilters,
    isQueryError,
    tenantEvents,
    toConsoleFilters,
    toGridRows,
} from "@/services/ElectoralLogConsole"

const PAGE_SIZES = [25, 50, 100, 200]
const DEFAULT_QUERY = `SELECT statement_kind, count(*) AS records
FROM electoral_log_messages
GROUP BY statement_kind
ORDER BY records DESC`
const GRID_HEIGHT = 620
const MONOSPACE = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"

const readHeaders = {headers: {"x-hasura-role": IPermissions.ELECTORAL_LOG_CONSOLE_READ}}
const queryHeaders = {headers: {"x-hasura-role": IPermissions.ELECTORAL_LOG_CONSOLE_QUERY}}

const gridColumns = ({columns}: ConsoleRows): GridColDef[] =>
    columns.map((column, index) => ({
        field: columnField(index),
        headerName: column,
        minWidth: 120,
        flex: 1,
        sortable: false,
        valueFormatter: (value: unknown) => displayValue(value),
    }))

interface PagePosition {
    page: number
    after: string | null
}

const FIRST_PAGE: PagePosition = {page: 0, after: null}

const FilterInput: React.FC<{
    field: FilterField
    value: string
    invalid: boolean
    elections: ConsoleElection[]
    onChange: (value: string) => void
}> = ({field, value, invalid, elections, onChange}) => {
    const {t} = useTranslation()
    const aliasRenderer = useAliasRenderer()
    const label = t(`electoralLogConsole.filters.${field}`)
    if (field === "election_id" || field === "status") {
        const options =
            field === "status"
                ? BALLOT_STATUSES.map((status) => ({
                      value: status,
                      label: t(`electoralLogConsole.statuses.${status}`),
                  }))
                : elections.map((election) => ({
                      value: election.id,
                      label: aliasRenderer(election),
                  }))
        return (
            <FormControl size="small" fullWidth>
                <InputLabel id={`electoral-log-filter-${field}`}>{label}</InputLabel>
                <Select
                    labelId={`electoral-log-filter-${field}`}
                    label={label}
                    value={value}
                    onChange={(e) => onChange(e.target.value)}
                >
                    <MenuItem value="">{t("electoralLogConsole.filters.any")}</MenuItem>
                    {options.map((option) => (
                        <MenuItem key={option.value} value={option.value}>
                            {option.label}
                        </MenuItem>
                    ))}
                </Select>
            </FormControl>
        )
    }
    const isDate = field === "created_after" || field === "created_before"
    return (
        <TextField
            size="small"
            label={label}
            value={value}
            error={invalid}
            type={isDate ? "datetime-local" : "text"}
            InputLabelProps={isDate ? {shrink: true} : undefined}
            onChange={(e) => onChange(e.target.value)}
            fullWidth
        />
    )
}

const RecordDialog: React.FC<{
    tenantId?: string
    electionEventId: string
    position: number | null
    onClose: () => void
}> = ({tenantId, electionEventId, position, onClose}) => {
    const {t} = useTranslation()
    const client = useApolloClient()
    const [record, setRecord] = useState<Record<string, unknown> | null>(null)
    const [error, setError] = useState<string | null>(null)

    useEffect(() => {
        if (position === null) {
            return
        }
        let cancelled = false
        setRecord(null)
        setError(null)
        client
            .query({
                query: ELECTORAL_LOG_CONSOLE_RECORD,
                variables: {tenantId, electionEventId, position},
                fetchPolicy: "no-cache",
                context: readHeaders,
            })
            .then(({data}) => {
                if (!cancelled) {
                    setRecord(data?.electoral_log_console_record ?? null)
                }
            })
            .catch((failure) => {
                if (!cancelled) {
                    setError(
                        getGraphQLActionErrorReason(failure) ??
                            t("electoralLogConsole.record.loadError")
                    )
                }
            })
        return () => {
            cancelled = true
        }
    }, [client, tenantId, electionEventId, position, t])

    const fields = record
        ? Object.entries(record).filter(([key]) => key !== "message" && key !== "personal_data")
        : []

    return (
        <Dialog open={position !== null} onClose={onClose} maxWidth="md" fullWidth>
            <DialogTitle>{t("electoralLogConsole.record.title", {position})}</DialogTitle>
            <DialogContent dividers>
                {error && <Alert severity="error">{error}</Alert>}
                {record?.personal_data === "hidden" && (
                    <Alert severity="info" sx={{mb: 2}}>
                        {t("electoralLogConsole.personalDataHidden")}
                    </Alert>
                )}
                {record && (
                    <>
                        <Table size="small">
                            <TableBody>
                                {fields.map(([key, value]) => (
                                    <TableRow key={key}>
                                        <TableCell sx={{fontWeight: 600, width: 160}}>
                                            {key}
                                        </TableCell>
                                        <TableCell
                                            sx={{fontFamily: MONOSPACE, wordBreak: "break-all"}}
                                        >
                                            {displayValue(value)}
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                        <Typography variant="subtitle2" sx={{mt: 2, mb: 1}}>
                            {t("electoralLogConsole.record.message")}
                        </Typography>
                        <Box
                            component="pre"
                            sx={{
                                fontFamily: MONOSPACE,
                                fontSize: 12,
                                bgcolor: "grey.100",
                                p: 2,
                                borderRadius: 1,
                                maxHeight: 360,
                                overflow: "auto",
                                m: 0,
                            }}
                        >
                            {JSON.stringify(record.message, null, 2)}
                        </Box>
                    </>
                )}
            </DialogContent>
            <DialogActions>
                <Button
                    disabled={!record}
                    onClick={() => navigator.clipboard?.writeText(JSON.stringify(record, null, 2))}
                >
                    {t("electoralLogConsole.record.copy")}
                </Button>
                <Button onClick={onClose}>{t("electoralLogConsole.record.close")}</Button>
            </DialogActions>
        </Dialog>
    )
}

// Users of the super-admin tenant choose the tenant to browse; others browse their
// own, whose events and elections Hasura lists.
const TablesTab: React.FC<{superAdmin: boolean}> = ({superAdmin}) => {
    const {t} = useTranslation()
    const client = useApolloClient()
    const [ownTenant] = useTenantStore()
    const aliasRenderer = useAliasRenderer()
    const [chosenTenant, setChosenTenant] = useState("")
    const {data: tenantsData, error: tenantsError} = useQuery(ELECTORAL_LOG_CONSOLE_TENANTS, {
        skip: !superAdmin,
        fetchPolicy: "network-only",
        context: readHeaders,
    })
    const tenants =
        (tenantsData?.electoral_log_console_tenants as ConsoleTenants | undefined)?.tenants ?? []
    const tenantId = (superAdmin && chosenTenant) || ownTenant || ""
    const {data: ownEvents} = useGetList<Sequent_Backend_Election_Event>(
        "sequent_backend_election_event",
        {
            pagination: {page: 1, perPage: 500},
            sort: {field: "created_at", order: "DESC"},
            filter: {tenant_id: tenantId},
        },
        {enabled: !superAdmin && !!tenantId}
    )
    const events: ConsoleElection[] = superAdmin
        ? tenantEvents(tenants, tenantId)
        : (ownEvents ?? [])
    const [eventId, setEventId] = useState("")
    const {data: ownElections} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            pagination: {page: 1, perPage: 500},
            sort: {field: "created_at", order: "ASC"},
            filter: {tenant_id: tenantId, election_event_id: eventId},
        },
        {enabled: !superAdmin && !!tenantId && !!eventId}
    )
    const elections: ConsoleElection[] = superAdmin
        ? eventElections(tenants, tenantId, eventId)
        : (ownElections ?? [])
    const requestTenant = superAdmin ? tenantId : undefined
    const [table, setTable] = useState<ConsoleTable>("records")
    const [order, setOrder] = useState<PageOrder>("newest-first")
    const [draft, setDraft] = useState<FilterValues>({})
    const [filters, setFilters] = useState<FilterValues>({})
    const [invalid, setInvalid] = useState<FilterField[]>([])
    const [pageSize, setPageSize] = useState(PAGE_SIZES[0])
    const [position, setPosition] = useState<PagePosition>(FIRST_PAGE)
    // Cursor each page starts after, as the pages read so far tell.
    const cursors = useRef<Array<string | null>>([null])
    const [page, setPage] = useState<ConsolePage | null>(null)
    const [loading, setLoading] = useState(false)
    const [error, setError] = useState<string | null>(null)
    const [recordPosition, setRecordPosition] = useState<number | null>(null)

    useEffect(() => {
        if (!eventId && events?.length) {
            setEventId(events[0].id)
        }
    }, [events, eventId])

    const restart = () => {
        cursors.current = [null]
        setPosition(FIRST_PAGE)
    }

    useEffect(() => {
        if (!eventId) {
            return
        }
        let cancelled = false
        setLoading(true)
        setError(null)
        client
            .query({
                query: ELECTORAL_LOG_CONSOLE_PAGE,
                variables: {
                    tenantId: requestTenant,
                    electionEventId: eventId,
                    table,
                    filters: toConsoleFilters(table, filters),
                    order,
                    after: position.after,
                    limit: pageSize,
                },
                fetchPolicy: "no-cache",
                context: readHeaders,
            })
            .then(({data}) => {
                if (cancelled) {
                    return
                }
                const result = data?.electoral_log_console_page as ConsolePage
                cursors.current = cursors.current.slice(0, position.page + 1)
                cursors.current[position.page + 1] = result.next
                setPage(result)
            })
            .catch((failure) => {
                if (!cancelled) {
                    setPage(null)
                    setError(
                        getGraphQLActionErrorReason(failure) ?? t("electoralLogConsole.loadError")
                    )
                }
            })
            .finally(() => {
                if (!cancelled) {
                    setLoading(false)
                }
            })
        return () => {
            cancelled = true
        }
    }, [client, requestTenant, eventId, table, order, filters, position, pageSize, t])

    const changePagination = (model: GridPaginationModel) => {
        if (model.pageSize !== pageSize) {
            setPageSize(model.pageSize)
            restart()
            return
        }
        const after = cursors.current[model.page]
        if (after !== undefined) {
            setPosition({page: model.page, after})
        }
    }

    const applyFilters = () => {
        const wrong = invalidFilters(table, draft)
        setInvalid(wrong)
        if (wrong.length === 0) {
            setFilters({...draft})
            restart()
        }
    }

    const clearFilters = () => {
        setDraft({})
        setInvalid([])
        setFilters({})
        restart()
    }

    const positionIndex = page?.columns.indexOf("position") ?? -1
    const columns = useMemo<GridColDef[]>(() => {
        if (!page) {
            return []
        }
        const data = gridColumns(page)
        if (table !== "records" || positionIndex < 0) {
            return data
        }
        const view: GridColDef = {
            field: "__view",
            headerName: "",
            width: 56,
            sortable: false,
            disableColumnMenu: true,
            disableExport: true,
            renderCell: (params) => (
                <IconButton
                    size="small"
                    aria-label={t("electoralLogConsole.record.view")}
                    onClick={() =>
                        setRecordPosition(Number(params.row[columnField(positionIndex)]))
                    }
                >
                    <VisibilityIcon fontSize="small" />
                </IconButton>
            ),
        }
        return [...data, view]
    }, [page, table, positionIndex, t])
    const rows = useMemo(
        () => (page ? toGridRows(page, position.page * pageSize) : []),
        [page, position.page, pageSize]
    )

    return (
        <Stack spacing={2}>
            {tenantsError && (
                <Alert severity="error">
                    {getGraphQLActionErrorReason(tenantsError) ??
                        t("electoralLogConsole.loadError")}
                </Alert>
            )}
            <Stack direction={{xs: "column", md: "row"}} spacing={2}>
                {superAdmin && (
                    <FormControl size="small" sx={{minWidth: 220}}>
                        <InputLabel id="electoral-log-tenant">
                            {t("electoralLogConsole.tenant")}
                        </InputLabel>
                        <Select
                            labelId="electoral-log-tenant"
                            label={t("electoralLogConsole.tenant")}
                            value={tenants.some((tenant) => tenant.id === tenantId) ? tenantId : ""}
                            onChange={(e) => {
                                setChosenTenant(e.target.value)
                                setEventId("")
                                setDraft({})
                                setFilters({})
                                restart()
                            }}
                        >
                            {tenants.map((tenant) => (
                                <MenuItem key={tenant.id} value={tenant.id}>
                                    {tenant.slug}
                                </MenuItem>
                            ))}
                        </Select>
                    </FormControl>
                )}
                <FormControl size="small" sx={{minWidth: 280}}>
                    <InputLabel id="electoral-log-event">
                        {t("electoralLogConsole.electionEvent")}
                    </InputLabel>
                    <Select
                        labelId="electoral-log-event"
                        label={t("electoralLogConsole.electionEvent")}
                        value={eventId}
                        onChange={(e) => {
                            setEventId(e.target.value)
                            setDraft({})
                            setFilters({})
                            restart()
                        }}
                    >
                        {events.map((event) => (
                            <MenuItem key={event.id} value={event.id}>
                                {aliasRenderer(event)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <FormControl size="small" sx={{minWidth: 200}}>
                    <InputLabel id="electoral-log-table">
                        {t("electoralLogConsole.table")}
                    </InputLabel>
                    <Select
                        labelId="electoral-log-table"
                        label={t("electoralLogConsole.table")}
                        value={table}
                        onChange={(e) => {
                            setTable(e.target.value as ConsoleTable)
                            setInvalid([])
                            restart()
                        }}
                    >
                        {CONSOLE_TABLES.map((name) => (
                            <MenuItem key={name} value={name}>
                                {t(`electoralLogConsole.tables.${name}`)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <FormControl size="small" sx={{minWidth: 180}}>
                    <InputLabel id="electoral-log-order">
                        {t("electoralLogConsole.order.label")}
                    </InputLabel>
                    <Select
                        labelId="electoral-log-order"
                        label={t("electoralLogConsole.order.label")}
                        value={order}
                        onChange={(e) => {
                            setOrder(e.target.value as PageOrder)
                            restart()
                        }}
                    >
                        {PAGE_ORDERS.map((name) => (
                            <MenuItem key={name} value={name}>
                                {t(`electoralLogConsole.order.${name}`)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
            </Stack>
            <Box
                sx={{
                    display: "grid",
                    gridTemplateColumns: {xs: "1fr", sm: "repeat(auto-fill, minmax(220px, 1fr))"},
                    gap: 2,
                    alignItems: "center",
                }}
            >
                {FILTERS_BY_TABLE[table].map((field) => (
                    <FilterInput
                        key={field}
                        field={field}
                        value={draft[field] ?? ""}
                        invalid={invalid.includes(field)}
                        elections={elections}
                        onChange={(value) => setDraft({...draft, [field]: value})}
                    />
                ))}
            </Box>
            <Stack direction="row" spacing={2}>
                <Button variant="contained" onClick={applyFilters}>
                    {t("electoralLogConsole.filters.apply")}
                </Button>
                <Button onClick={clearFilters}>{t("electoralLogConsole.filters.clear")}</Button>
            </Stack>
            {invalid.length > 0 && (
                <Alert severity="warning">{t("electoralLogConsole.filters.invalid")}</Alert>
            )}
            {error && <Alert severity="error">{error}</Alert>}
            {page && (
                <Typography variant="body2" color="text.secondary">
                    {t("electoralLogConsole.estimatedRows", {
                        rows: page.estimated_rows.toLocaleString(),
                    })}
                </Typography>
            )}
            {page?.personal_data === "hidden" && page.personal_columns.length > 0 && (
                <Alert severity="info">{t("electoralLogConsole.personalDataHidden")}</Alert>
            )}
            <Box sx={{height: GRID_HEIGHT, width: "100%"}}>
                <DataGrid
                    rows={rows}
                    columns={columns}
                    getRowId={(row) => row[ROW_ID] as number}
                    loading={loading}
                    paginationMode="server"
                    rowCount={-1}
                    paginationMeta={{hasNextPage: !!page?.next}}
                    paginationModel={{page: position.page, pageSize}}
                    onPaginationModelChange={changePagination}
                    pageSizeOptions={PAGE_SIZES}
                    disableColumnFilter
                    disableColumnSorting
                    disableRowSelectionOnClick
                    showToolbar
                    slotProps={{
                        toolbar: {
                            showQuickFilter: false,
                            csvOptions: {fileName: `electoral-log-${table}`},
                            printOptions: {disableToolbarButton: true},
                        },
                    }}
                />
            </Box>
            <RecordDialog
                tenantId={requestTenant}
                electionEventId={eventId}
                position={recordPosition}
                onClose={() => setRecordPosition(null)}
            />
        </Stack>
    )
}

// Each election event has a database of its own: a query reads the chosen event's.
const QueryTab: React.FC = () => {
    const {t} = useTranslation()
    const client = useApolloClient()
    const aliasRenderer = useAliasRenderer()
    const [sql, setSql] = useState(DEFAULT_QUERY)
    const [result, setResult] = useState<ConsoleQueryRows | null>(null)
    const [error, setError] = useState<string | null>(null)
    const [running, setRunning] = useState(false)
    const [tenantId, setTenantId] = useState("")
    const [eventId, setEventId] = useState("")
    const {data: tenantsData, error: tenantsError} = useQuery(ELECTORAL_LOG_CONSOLE_TENANTS, {
        fetchPolicy: "network-only",
        context: readHeaders,
    })
    const tenants =
        (tenantsData?.electoral_log_console_tenants as ConsoleTenants | undefined)?.tenants ?? []
    const events = tenantEvents(tenants, tenantId)
    const ready = !!tenantId && !!eventId && !!sql.trim()

    const run = async () => {
        if (running || !ready) {
            return
        }
        setRunning(true)
        setError(null)
        try {
            const {data} = await client.query({
                query: ELECTORAL_LOG_CONSOLE_QUERY,
                variables: {tenantId, electionEventId: eventId, sql},
                fetchPolicy: "no-cache",
                context: queryHeaders,
            })
            const answer = data?.electoral_log_console_query as ConsoleQueryResult
            if (isQueryError(answer)) {
                setResult(null)
                setError(answer.error)
            } else {
                setResult(answer)
            }
        } catch (failure) {
            setResult(null)
            setError(getGraphQLActionErrorReason(failure) ?? t("electoralLogConsole.query.error"))
        } finally {
            setRunning(false)
        }
    }

    const columns = useMemo(() => (result ? gridColumns(result) : []), [result])
    const rows = useMemo(() => (result ? toGridRows(result) : []), [result])

    return (
        <Stack spacing={2}>
            <Typography variant="body2" color="text.secondary">
                {t("electoralLogConsole.query.help")} {t("electoralLogConsole.query.scope")}
            </Typography>
            {tenantsError && (
                <Alert severity="error">
                    {getGraphQLActionErrorReason(tenantsError) ??
                        t("electoralLogConsole.loadError")}
                </Alert>
            )}
            <Stack direction={{xs: "column", md: "row"}} spacing={2}>
                <FormControl size="small" sx={{minWidth: 220}}>
                    <InputLabel id="electoral-log-query-tenant">
                        {t("electoralLogConsole.tenant")}
                    </InputLabel>
                    <Select
                        labelId="electoral-log-query-tenant"
                        label={t("electoralLogConsole.tenant")}
                        value={tenants.some((tenant) => tenant.id === tenantId) ? tenantId : ""}
                        onChange={(e) => {
                            setTenantId(e.target.value)
                            setEventId("")
                        }}
                    >
                        {tenants.map((tenant) => (
                            <MenuItem key={tenant.id} value={tenant.id}>
                                {tenant.slug}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <FormControl size="small" sx={{minWidth: 280}}>
                    <InputLabel id="electoral-log-query-event">
                        {t("electoralLogConsole.electionEvent")}
                    </InputLabel>
                    <Select
                        labelId="electoral-log-query-event"
                        label={t("electoralLogConsole.electionEvent")}
                        value={events.some((event) => event.id === eventId) ? eventId : ""}
                        onChange={(e) => setEventId(e.target.value)}
                    >
                        {events.map((event) => (
                            <MenuItem key={event.id} value={event.id}>
                                {aliasRenderer(event)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
            </Stack>
            {!eventId && (
                <Alert severity="info">{t("electoralLogConsole.query.chooseEvent")}</Alert>
            )}
            <TextField
                multiline
                minRows={6}
                fullWidth
                value={sql}
                placeholder={t("electoralLogConsole.query.placeholder")}
                onChange={(e) => setSql(e.target.value)}
                onKeyDown={(e) => {
                    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
                        run()
                    }
                }}
                inputProps={{spellCheck: false, style: {fontFamily: MONOSPACE, fontSize: 13}}}
            />
            <Stack direction="row" spacing={2} alignItems="center" justifyContent="space-between">
                <Typography variant="caption" color="text.secondary">
                    {t("electoralLogConsole.query.limits")}
                </Typography>
                <Button variant="contained" onClick={run} disabled={running || !ready}>
                    {t("electoralLogConsole.query.run")}
                </Button>
            </Stack>
            {error && (
                <Alert severity="error" sx={{whiteSpace: "pre-wrap", fontFamily: MONOSPACE}}>
                    {error}
                </Alert>
            )}
            {result && (
                <>
                    <Typography variant="body2" color="text.secondary">
                        {t("electoralLogConsole.query.summary", {
                            rows: result.rows.length.toLocaleString(),
                            ms: result.elapsed_ms.toLocaleString(),
                        })}
                    </Typography>
                    {result.truncated && (
                        <Alert severity="info">
                            {t("electoralLogConsole.query.truncated", {
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
                                    csvOptions: {fileName: "electoral-log-query"},
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

type ConsoleTab = "tables" | "query"

export const ElectoralLogConsole: React.FC = () => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [tab, setTab] = useState<ConsoleTab>("tables")
    const allowed = (permission: IPermissions) =>
        authContext.isAuthorized(true, authContext.tenantId, permission)
    // Only users of the super-admin tenant browse other tenants' events or query an
    // event's electoral-log database.
    const superAdminAllowed = (permission: IPermissions) =>
        authContext.isAuthorized(true, null, permission)
    const canRead = allowed(IPermissions.ELECTORAL_LOG_CONSOLE_READ)
    const superAdmin = superAdminAllowed(IPermissions.ELECTORAL_LOG_CONSOLE_READ)
    const canQuery =
        superAdminAllowed(IPermissions.ELECTORAL_LOG_CONSOLE_QUERY) &&
        superAdminAllowed(IPermissions.ELECTORAL_LOG_PERSONAL_DATA_READ)

    if (!canRead) {
        return <Alert severity="warning">{t("electoralLogConsole.notAllowed")}</Alert>
    }
    return (
        // The page takes the width the layout gives it, whatever its tables' width.
        <Card sx={{contain: "inline-size"}}>
            <CardContent>
                <Typography variant="h4" sx={{mb: 1}}>
                    {t("electoralLogConsole.title")}
                </Typography>
                <Typography variant="body2" color="text.secondary" sx={{mb: 2}}>
                    {t("electoralLogConsole.subtitle")}
                </Typography>
                <Tabs value={tab} onChange={(_, value: ConsoleTab) => setTab(value)} sx={{mb: 2}}>
                    <Tab value="tables" label={t("electoralLogConsole.tabs.tables")} />
                    {canQuery && <Tab value="query" label={t("electoralLogConsole.tabs.query")} />}
                </Tabs>
                {tab === "query" && canQuery ? <QueryTab /> : <TablesTab superAdmin={superAdmin} />}
            </CardContent>
        </Card>
    )
}
