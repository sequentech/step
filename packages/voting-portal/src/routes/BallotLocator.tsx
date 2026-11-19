// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState, useRef} from "react"
import {useTranslation} from "react-i18next"
import {
    BreadCrumbSteps,
    PageLimit,
    theme,
    Icon,
    InfoDataBox,
    IconButton,
    Dialog,
    ExpandableText,
} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    EShowCastVoteLogsPolicy,
    formatVotingPortalDateTime,
    translateHtml,
    stringToText,
} from "@sequentech/ui-core"
import {
    Box,
    TextField,
    Typography,
    Button,
    Stack,
    TypographyProps,
    LinearProgress,
} from "@mui/material"
import {styled} from "@mui/material/styles"
import Tabs from "@mui/material/Tabs"
import Tab from "@mui/material/Tab"
import {Link, useLocation, useNavigate, useParams} from "react-router-dom"
import {GET_CAST_VOTE} from "../queries/GetCastVote"
import {useQuery} from "@apollo/client/react"
import {
    GetCastVoteQuery,
    GetElectionsQuery,
    GetElectionEventQuery,
    ListCastVoteMessagesQuery,
    TotalBound,
} from "../gql/graphql"
import {faAngleLeft, faCircleQuestion, faCopy} from "@fortawesome/free-solid-svg-icons"
import {LIST_CAST_VOTE_MESSAGES} from "../queries/listCastVoteMessages"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {GET_ELECTIONS} from "../queries/GetElections"
import {IElectionEvent, setElectionEvent} from "../store/electionEvents/electionEventsSlice"
import Table from "@mui/material/Table"
import TableSortLabel from "@mui/material/TableSortLabel"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TablePagination from "@mui/material/TablePagination"
import TableHead from "@mui/material/TableHead"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import {ICastVoteEntry} from "../types/castVoteLogEntry"

const StyledTitle = styled(Typography)<{component?: React.ElementType}>`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 24px;
    font-weight: 500;
    line-height: 27px;
    margin-top: 20px;
    margin-bottom: 16px;
`

const StyledError = styled(Typography)<TypographyProps>`
    position: absolute;
    margin-top: -12px;
    color: ${({theme}) => theme.palette.red.main};
`

// Only one of the two ballot-id errors is shown at a time, so they can share the
// id that the input's aria-describedby points at.
const BALLOT_ID_ERROR_ID = "ballot-id-error"

const MessageSuccess = styled(Box)`
    display: flex;
    padding: 10px 22px;
    color: ${({theme}) => theme.palette.green.dark};
    background-color: ${({theme}) => theme.palette.green.light};
    gap: 8px;
    border-radius: 4px;
    border: 1px solid ${({theme}) => theme.palette.green.dark};
    align-items: center;
    margin-right: auto;
    margin-left: auto;
    overflow-wrap: anywhere;
`

const MessageFailed = styled(Box)`
    display: flex;
    padding: 10px 22px;
    color: ${({theme}) => theme.palette.red.dark};
    background-color: ${({theme}) => theme.palette.red.light};
    gap: 8px;
    border-radius: 4px;
    border: 1px solid ${({theme}) => theme.palette.red.dark};
    align-items: center;
    margin-right: auto;
    margin-left: auto;
    overflow-wrap: anywhere;
`

// A whole Ballot ID in hexadecimal characters (BALLOT_ID_LENGTH_CHARS in windmill's
// electoral_log.rs): the server rejects a longer filter.
const BALLOT_ID_MAX_LENGTH = 64

function isHex(str: string) {
    // Surrounding spaces don't count, as on the server.
    const text = str.trim()
    if (text === "") {
        return true
    }
    if (text.length % 2 !== 0 || text.length > BALLOT_ID_MAX_LENGTH) {
        return false
    }
    const regex = /^[0-9a-fA-F]+$/
    return regex.test(text)
}

interface TabPanelProps {
    className?: string
    children?: React.ReactNode
    index: number
    value: number
}

const CustomTabPanel: React.FC<TabPanelProps> = ({children, index, value, className}) => {
    return (
        <div
            className={className}
            role="tabpanel"
            hidden={value !== index}
            id={`simple-tabpanel-${index}`}
            aria-labelledby={`simple-tab-${index}`}
        >
            {value === index && <Box className="ballot-locator-panel-content">{children}</Box>}
        </div>
    )
}

const BallotLocator: React.FC = () => {
    const {t, i18n} = useTranslation()
    const location = useLocation()
    const {tenantId, eventId, electionId} = useParams()
    const dispatch = useAppDispatch()
    const allowSendRequest = useRef<boolean>(true)
    const [value, setValue] = React.useState(0)
    const [inputBallotId, setInputBallotId] = useState("")
    const [rows, setRows] = useState<ICastVoteEntry[]>([])
    const [total, setTotal] = useState(0)
    const [totalBound, setTotalBound] = useState<TotalBound>(TotalBound.Exact)
    // A cast-vote request is pending: the rows shown are not its answer.
    const [loadingRows, setLoadingRows] = useState(false)
    const [ballotIdNotFoundErr, setBallotIdNotFoundErr] = useState(false)
    const [somethingWentWrongErr, setSomethingWentWrongErr] = useState(false)
    const validatedBallotId = isHex(inputBallotId ?? "")
    const [showCVLogsPolicy, setShowCVLogsPolicy] = useState(false)
    const {globalSettings} = useContext(SettingsContext)
    const [page, setPage] = React.useState(0)
    const [rowsPerPage, setRowsPerPage] = React.useState(5)
    const lastCVRequestTimestamp = useRef<number | undefined>(undefined) // Timestamp of last LIST_CAST_VOTE_MESSAGES request
    const {data: dataElectionEvent} = useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {
        variables: {
            electionEventId: eventId,
            tenantId,
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })

    useEffect(() => {
        const electionEvent = dataElectionEvent?.sequent_backend_election_event[0]
        if (electionEvent) {
            dispatch(setElectionEvent(electionEvent as IElectionEvent))
        }
    }, [dataElectionEvent, dispatch])

    const {refetch} = useQuery<ListCastVoteMessagesQuery>(LIST_CAST_VOTE_MESSAGES, {
        variables: {
            tenantId,
            electionEventId: eventId,
            electionId,
            ballotId: inputBallotId,
        },
        skip: true,
    })

    let fetchTimeout: any = useRef(undefined)
    // Only the latest cast-vote request may update the table or the error.
    const latestRequestId = useRef(0)
    // A deferred fetch doesn't outlive the page.
    useEffect(() => () => clearTimeout(fetchTimeout.current), [])

    const requestCVMsgs = async (headerName?: string, newOrder?: string) => {
        let duration = lastCVRequestTimestamp.current
            ? Date.now() - lastCVRequestTimestamp.current
            : undefined
        let tooQuick = duration ? duration < 500 : false
        lastCVRequestTimestamp.current = Date.now()
        if (!validatedBallotId) {
            // A malformed filter is never sent, and it supersedes the earlier
            // requests: a deferred one is dropped, an in-flight one is ignored.
            clearTimeout(fetchTimeout.current)
            latestRequestId.current++
            setLoadingRows(false)
            return
        }
        // With DISABLE_AUTH only the deferred path fetches.
        if (globalSettings.DISABLE_AUTH && !tooQuick) {
            return
        }
        // This request supersedes the earlier ones as soon as it is accepted,
        // even while it waits for its deferred start.
        const requestId = ++latestRequestId.current
        clearTimeout(fetchTimeout.current)
        setLoadingRows(true)
        const ballotIdFilter = inputBallotId.trim()
        async function tryFetchMessages() {
            try {
                let limit = rowsPerPage
                let offset = page * rowsPerPage
                const {data} = await refetch({
                    ballotId: ballotIdFilter,
                    orderBy: {[headerName ?? "id"]: newOrder ?? "desc"},
                    limit,
                    offset,
                })
                if (requestId !== latestRequestId.current) {
                    return
                }
                console.log(data)
                setSomethingWentWrongErr(false)
                if (data?.list_cast_vote_messages) {
                    setRows((data?.list_cast_vote_messages?.list ?? []) as ICastVoteEntry[])
                    setTotal(data?.list_cast_vote_messages?.total)
                    setTotalBound(data?.list_cast_vote_messages?.total_bound)
                    setBallotIdNotFoundErr(
                        ballotIdFilter.length > 0 &&
                            data?.list_cast_vote_messages?.list.length === 0
                    )
                }
                setLoadingRows(false)
            } catch (e) {
                if (requestId !== latestRequestId.current) {
                    return
                }
                setSomethingWentWrongErr(true)
                setLoadingRows(false)
            }
        }

        if (tooQuick) {
            fetchTimeout.current = setTimeout(async () => {
                await tryFetchMessages()
            }, 1000)
            return
        }
        await tryFetchMessages()
    }

    const onClickHeader = (headerName: string, newOrder: string) => {
        requestCVMsgs(headerName, newOrder)
    }
    useEffect(() => {
        if (validatedBallotId) {
            setBallotIdNotFoundErr(false)
        }
        const showLogs = dataElectionEvent?.sequent_backend_election_event[0]?.presentation
            ?.show_cast_vote_logs as EShowCastVoteLogsPolicy
        setShowCVLogsPolicy(showLogs === EShowCastVoteLogsPolicy.SHOW_LOGS_TAB)
        // the length must be an even number of characters
        if (showLogs && allowSendRequest.current) {
            allowSendRequest.current = false
            requestCVMsgs()
        }
    }, [inputBallotId, dataElectionEvent, page, rowsPerPage])

    const handleChangePage = (event: unknown, newPage: number) => {
        allowSendRequest.current = true
        setPage(newPage)
    }
    const handleChangeRowsPerPage = (event: React.ChangeEvent<HTMLInputElement>) => {
        allowSendRequest.current = true
        setRowsPerPage(parseInt(event.target.value, 10))
        setPage(0)
    }

    const a11yProps = (index: number) => {
        return {
            "id": `simple-tab-${index}`,
            "aria-controls": `simple-tabpanel-${index}`,
        }
    }
    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    // Every change of the filter, typed or pasted, is a new request from the first page.
    const changeFilter = (value: string) => {
        allowSendRequest.current = true
        setInputBallotId(value)
        setPage(0)
    }

    return (
        <PageLimit
            className="ballot-locator-screen screen"
            maxWidth="lg"
            sx={{marginTop: "48px", width: "100%"}}
        >
            <Box
                className="ballot-locator-tabs-container"
                sx={{borderBottom: 1, borderColor: "divider"}}
            >
                <Tabs
                    className="ballot-locator-tabs"
                    variant="scrollable"
                    allowScrollButtonsMobile
                    scrollButtons="auto"
                    indicatorColor="primary"
                    textColor="inherit"
                    sx={{fontFamily: "Roboto"}}
                    aria-label={t("a11y.ballotLocatorTabs")}
                    value={value}
                    onChange={handleChange}
                >
                    <Tab
                        className="ballot-lookup-tab"
                        label={t("ballotLocator.tabs.ballotLocator")}
                        {...a11yProps(0)}
                    />
                    {showCVLogsPolicy && (
                        <Tab
                            className="cast-vote-logs-tab"
                            label={t("ballotLocator.tabs.logs")}
                            {...a11yProps(1)}
                        />
                    )}
                </Tabs>
            </Box>
            <Box className="ballot-locator-panels" sx={{p: 3}}>
                <CustomTabPanel className="ballot-lookup-panel" value={value} index={0}>
                    <BallotLocatorLogic />
                </CustomTabPanel>
                <CustomTabPanel className="cast-vote-logs-panel" value={value} index={1}>
                    <Box className="cast-vote-logs-filter" marginTop="48px">
                        <BallotIdInput
                            inputBallotId={inputBallotId}
                            setInputBallotId={changeFilter}
                            validatedBallotId={validatedBallotId}
                            ballotIdNotFoundErr={ballotIdNotFoundErr}
                            placeholderLabel="ballotLocator.filterByBallotId"
                        />
                    </Box>
                    <LogsTable
                        rows={rows}
                        total={total}
                        totalBound={totalBound}
                        loading={loadingRows}
                        onOrderBy={onClickHeader}
                        rowsPerPage={rowsPerPage}
                        handleChangeRowsPerPage={handleChangeRowsPerPage}
                        page={page}
                        handleChangePage={handleChangePage}
                        somethingWentWrongErr={somethingWentWrongErr}
                        formatDateTime={(timestamp) =>
                            formatVotingPortalDateTime(
                                timestamp,
                                dataElectionEvent
                                    ?.sequent_backend_election_event[0] as IElectionEvent,
                                i18n.resolvedLanguage || i18n.language
                            )
                        }
                    />
                </CustomTabPanel>
                <Box
                    className="ballot-locator-actions"
                    sx={{
                        order: {xs: 1, md: 2},
                        marginTop: "20px",
                        width: "fit-content",
                    }}
                >
                    <Button
                        component={Link}
                        to={`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`}
                        variant="secondary"
                        className="secondary back-button"
                    >
                        <Icon className="back-button-icon" icon={faAngleLeft} size="sm" />
                        <Box className="back-button-label">{t("votingScreen.backButton")}</Box>
                    </Button>
                </Box>
            </Box>
        </PageLimit>
    )
}

// Past a capped total the number of ballots is only known once a page comes back short.
const paginationCount = (
    totalBound: TotalBound,
    total: number,
    page: number,
    rowsPerPage: number,
    rowsOnPage: number
): number => {
    if (totalBound !== TotalBound.MoreThan) {
        return total
    }
    return rowsOnPage < rowsPerPage ? page * rowsPerPage + rowsOnPage : -1
}

const LOADING_BAR_EDGES = ["top", "bottom"] as const

/**
 * While another page, filter or sort loads, the previous rows stay shown: dimmed rows and
 * progress bars show that they are stale. The rows can be taller than the screen, so there is a
 * bar by the filter and one by the pagination.
 */
const StaleRows: React.FC<{stale: boolean; children: React.ReactNode}> = ({stale, children}) => (
    <Box className="cast-vote-logs-rows" sx={{position: "relative"}} aria-busy={stale}>
        {stale &&
            LOADING_BAR_EDGES.map((edge) => (
                <LinearProgress
                    key={edge}
                    className="cast-vote-logs-loading"
                    sx={{position: "absolute", [edge]: 0, left: 0, right: 0, zIndex: 2}}
                />
            ))}
        <Box
            className="cast-vote-logs-rows-content"
            sx={{opacity: stale ? 0.4 : 1, transition: "opacity 150ms"}}
        >
            {children}
        </Box>
    </Box>
)

interface LogsTableProps {
    rows: ICastVoteEntry[]
    total: number
    totalBound: TotalBound
    loading: boolean
    onOrderBy?: (headerName: string, newOrder: string) => void
    rowsPerPage: number
    handleChangeRowsPerPage: (event: React.ChangeEvent<HTMLInputElement>) => void
    page: number
    handleChangePage: (event: unknown, newValue: number) => void
    somethingWentWrongErr: boolean
    formatDateTime: (timestamp: number) => string
}

interface MessageCellProps {
    message: string | null
    initialLength: number
}

const MessageCell: React.FC<MessageCellProps> = ({message, initialLength}) => {
    const {t} = useTranslation()
    const formatJson = (json: string) => {
        try {
            return JSON.stringify(JSON.parse(json), null, 2)
        } catch {
            return json
        }
    }

    if (!message) {
        // The message column is justified; center the dash as in the other columns.
        return (
            <Box className="cast-vote-log-empty-message" sx={{textAlign: "center"}}>
                -
            </Box>
        )
    }

    const formattedMessage = formatJson(message)

    return (
        <Box className="cast-vote-log-message" sx={{position: "relative", width: "100%"}}>
            <IconButton
                buttonClassName="cast-vote-log-copy-button"
                icon={faCopy}
                size="xs"
                onClick={() => navigator.clipboard.writeText(formattedMessage)}
                ariaLabel={t("a11y.copyToClipboard", {
                    label: t("ballotLocator.column.message"),
                })}
                sx={{
                    "position": "absolute",
                    "top": "4px",
                    "left": "4px",
                    "zIndex": 1,
                    "backgroundColor": "rgba(255, 255, 255, 0.8)",
                    "&:hover": {
                        backgroundColor: "rgba(255, 255, 255, 0.9)",
                    },
                    "minWidth": "20px",
                    "minHeight": "20px",
                    "padding": "2px",
                }}
            />
            <Box
                className="cast-vote-log-message-content"
                sx={{
                    "paddingLeft": "28px",
                    "& > div > div:last-child": {
                        justifyContent: "flex-start !important",
                    },
                }}
            >
                <ExpandableText
                    text={formattedMessage}
                    initialLength={initialLength}
                    showMoreLabel={t("common.showMore")}
                    showLessLabel={t("common.showLess")}
                    preformatted={true}
                />
            </Box>
        </Box>
    )
}

const LogsTable: React.FC<LogsTableProps> = ({
    rows,
    total,
    totalBound,
    loading,
    onOrderBy,
    rowsPerPage,
    handleChangeRowsPerPage,
    page,
    handleChangePage,
    somethingWentWrongErr = false,
    formatDateTime,
}) => {
    const {t} = useTranslation()
    const [orderBy, setOrderBy] = useState<string>("")
    const [order, setOrder] = useState<"desc" | "asc" | undefined>("desc")
    const [windowWidth, setWindowWidth] = useState<number>(window.innerWidth)

    useEffect(() => {
        const handleResize = () => setWindowWidth(window.innerWidth)
        window.addEventListener("resize", handleResize)
        return () => window.removeEventListener("resize", handleResize)
    }, [])

    // Calculate initialLength: 20 at 1200px+, scale down faster (quadratic) below 1200px
    const ratio = windowWidth / 1200
    const initialLength =
        windowWidth >= 1200 ? 20 : Math.max(3, Math.floor(ratio * ratio * ratio * 20))

    const onClickHeader = (headerName: string) => {
        setOrderBy(headerName)
        const newOrder = order === "desc" ? "asc" : "desc"
        setOrder(newOrder)
        onOrderBy?.(headerName, newOrder)
    }

    return (
        <>
            <StyledTitle className="cast-vote-logs-title" variant="h5" component="h2">
                {t(
                    totalBound === TotalBound.MoreThan
                        ? "ballotLocator.totalBallotsMoreThan"
                        : "ballotLocator.totalBallots",
                    {total}
                )}
            </StyledTitle>
            <StaleRows stale={loading}>
                <TableContainer
                    className="cast-vote-logs-table-container"
                    component={Paper}
                    sx={{overflowX: "auto", width: "100%"}}
                >
                    <Table
                        className="cast-vote-logs-table"
                        sx={{
                            "tableLayout": "auto",
                            "& .MuiTableCell-root": {
                                border: "1px solid #e0e0e0",
                            },
                        }}
                        aria-label={t("a11y.ballotsTable")}
                    >
                        <TableHead className="cast-vote-logs-table-head">
                            <TableRow className="cast-vote-logs-header-row">
                                <TableCell
                                    className="cast-vote-logs-username-header"
                                    align="center"
                                    sx={{fontWeight: "bold", padding: "2px 4px"}}
                                >
                                    {t("ballotLocator.column.username")}
                                </TableCell>
                                <TableCell
                                    className="cast-vote-logs-ballot-id-header"
                                    align="center"
                                    sortDirection={orderBy === "ballot_id" ? order : false}
                                    sx={{fontWeight: "bold", padding: "2px 4px"}}
                                >
                                    <TableSortLabel
                                        className="cast-vote-logs-sort-button"
                                        active={orderBy === "ballot_id"}
                                        direction={orderBy === "ballot_id" ? order : "asc"}
                                        onClick={() => onClickHeader("ballot_id")}
                                        sx={{
                                            display: "flex",
                                            flexDirection: "column",
                                            alignItems: "center",
                                            justifyContent: "center",
                                            textAlign: "center",
                                        }}
                                    >
                                        {t("ballotLocator.column.ballot_id")}
                                    </TableSortLabel>
                                </TableCell>
                                <TableCell
                                    className="cast-vote-logs-kind-header"
                                    align="center"
                                    sortDirection={orderBy === "statement_kind" ? order : false}
                                    sx={{fontWeight: "bold", padding: "2px 4px"}}
                                >
                                    <TableSortLabel
                                        className="cast-vote-logs-sort-button"
                                        active={orderBy === "statement_kind"}
                                        direction={orderBy === "statement_kind" ? order : "asc"}
                                        onClick={() => onClickHeader("statement_kind")}
                                        sx={{
                                            display: "flex",
                                            flexDirection: "column",
                                            alignItems: "center",
                                            justifyContent: "center",
                                            textAlign: "center",
                                        }}
                                    >
                                        {t("ballotLocator.column.statement_kind")}
                                    </TableSortLabel>
                                </TableCell>
                                <TableCell
                                    className="cast-vote-logs-timestamp-header"
                                    align="center"
                                    sortDirection={
                                        orderBy === "statement_timestamp" ? order : false
                                    }
                                    sx={{fontWeight: "bold", padding: "2px 4px"}}
                                >
                                    <TableSortLabel
                                        className="cast-vote-logs-sort-button"
                                        active={orderBy === "statement_timestamp"}
                                        direction={
                                            orderBy === "statement_timestamp" ? order : "asc"
                                        }
                                        onClick={() => onClickHeader("statement_timestamp")}
                                        sx={{
                                            display: "flex",
                                            flexDirection: "column",
                                            alignItems: "center",
                                            justifyContent: "center",
                                            textAlign: "center",
                                        }}
                                    >
                                        {t("ballotLocator.column.statement_timestamp")}
                                    </TableSortLabel>
                                </TableCell>
                                <TableCell
                                    className="cast-vote-logs-message-header"
                                    align="center"
                                    sx={{fontWeight: "bold", padding: "2px 4px"}}
                                >
                                    <Box
                                        className="cast-vote-logs-message-heading"
                                        sx={{
                                            display: "flex",
                                            flexDirection: "column",
                                            alignItems: "center",
                                            justifyContent: "center",
                                            textAlign: "center",
                                        }}
                                    >
                                        {t("ballotLocator.column.message")}
                                    </Box>
                                </TableCell>
                            </TableRow>
                        </TableHead>
                        <TableBody className="cast-vote-logs-table-body">
                            {rows.map((row, index) => (
                                <TableRow className="cast-vote-logs-row" key={index}>
                                    <TableCell
                                        className="cast-vote-logs-username-cell"
                                        align="center"
                                        sx={{
                                            wordBreak: "break-all",
                                            whiteSpace: "normal",
                                            padding: "2px 4px",
                                        }}
                                    >
                                        {row.username ?? "-"}
                                    </TableCell>
                                    <TableCell
                                        className="cast-vote-logs-ballot-id-cell"
                                        align="center"
                                        sx={{
                                            wordBreak: "break-all",
                                            whiteSpace: "normal",
                                            padding: "2px 4px",
                                        }}
                                    >
                                        {initialLength < 10 ? (
                                            <ExpandableText
                                                text={row.ballot_id}
                                                initialLength={initialLength}
                                                showMoreLabel={t("common.showMore")}
                                                showLessLabel={t("common.showLess")}
                                                preformatted={true}
                                            />
                                        ) : (
                                            row.ballot_id
                                        )}
                                    </TableCell>
                                    <TableCell
                                        className="cast-vote-logs-kind-cell"
                                        align="center"
                                        sx={{
                                            wordBreak: "break-all",
                                            whiteSpace: "normal",
                                            padding: "2px 4px",
                                        }}
                                    >
                                        {row.statement_kind}
                                    </TableCell>
                                    <TableCell
                                        className="cast-vote-logs-timestamp-cell"
                                        align="center"
                                        sx={{
                                            wordBreak: "break-all",
                                            whiteSpace: "normal",
                                            padding: "2px 4px",
                                        }}
                                    >
                                        {formatDateTime(row.statement_timestamp * 1000)}
                                    </TableCell>
                                    <TableCell
                                        className="cast-vote-logs-message-cell"
                                        align="justify"
                                        sx={{
                                            wordBreak: "break-all",
                                            whiteSpace: "normal",
                                            padding: "2px 4px",
                                        }}
                                    >
                                        <MessageCell
                                            message={row.message}
                                            initialLength={initialLength}
                                        />
                                    </TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </TableContainer>
            </StaleRows>
            <TablePagination
                className="cast-vote-logs-pagination"
                rowsPerPageOptions={[5, 10, 25, 50]}
                component="div"
                count={paginationCount(totalBound, total, page, rowsPerPage, rows.length)}
                rowsPerPage={rowsPerPage}
                page={page}
                onPageChange={handleChangePage}
                onRowsPerPageChange={handleChangeRowsPerPage}
            />
            {somethingWentWrongErr && (
                <StyledError className="cast-vote-logs-error" component="div">
                    {stringToHtml(t("errors.page.somethingWrong"))}
                </StyledError>
            )}
        </>
    )
}

interface BallotIdInputProps {
    inputBallotId: string
    setInputBallotId: (value: string) => void
    validatedBallotId: boolean
    ballotIdNotFoundErr?: boolean
    captureEnter?: React.KeyboardEventHandler<HTMLDivElement>
    placeholderLabel: string
}

const BallotIdInput: React.FC<BallotIdInputProps> = ({
    inputBallotId,
    setInputBallotId,
    validatedBallotId,
    ballotIdNotFoundErr = false,
    captureEnter,
    placeholderLabel,
}) => {
    const {t} = useTranslation()
    const hasBallotIdError = !validatedBallotId || ballotIdNotFoundErr

    return (
        <>
            <TextField
                className="ballot-id-field"
                onChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                    setInputBallotId(event.target.value)
                }}
                value={inputBallotId}
                label={t("a11y.ballotIdLabel")}
                placeholder={stringToText(t(placeholderLabel))}
                onKeyDown={captureEnter}
                error={hasBallotIdError}
                slotProps={{
                    htmlInput: {
                        "className": "ballot-id-input",
                        "aria-describedby": hasBallotIdError ? BALLOT_ID_ERROR_ID : undefined,
                    },
                    inputLabel: {shrink: true},
                }}
            />
            {/* The live region stays mounted and only its text changes: a region
                inserted at the same moment as its text is not reliably read. */}
            <StyledError
                className="ballot-id-error"
                id={BALLOT_ID_ERROR_ID}
                role="alert"
                component="div"
            >
                {!validatedBallotId
                    ? stringToHtml(t("ballotLocator.wrongFormatBallotId"))
                    : ballotIdNotFoundErr
                      ? stringToHtml(t("ballotLocator.ballotIdNotFoundAtFilter"))
                      : ""}
            </StyledError>
        </>
    )
}

const BallotLocatorLogic = () => {
    const {tenantId, eventId, electionId, ballotId} = useParams()
    const [openTitleHelp, setOpenTitleHelp] = useState<boolean>(false)
    const navigate = useNavigate()
    const location = useLocation()
    const {t} = useTranslation()
    const [inputBallotId, setInputBallotId] = useState<string>("")
    const {globalSettings} = useContext(SettingsContext)

    const hasBallotId = !!ballotId
    const {data: dataElections, loading: loadingElections} = useQuery<GetElectionsQuery>(
        GET_ELECTIONS,
        {
            variables: {
                electionIds: electionId ? [electionId] : [],
            },
            skip: globalSettings.DISABLE_AUTH || !electionId,
        }
    )

    const election = dataElections?.sequent_backend_election.find((item) => item.id === electionId)
    const telephoneVotingEnabled = election?.voting_channels?.telephone === true
    const normalizedBallotId = ballotId?.toLowerCase() ?? ""
    const ballotIdPattern = /^[0-9a-f]+$/.test(normalizedBallotId)
        ? telephoneVotingEnabled && normalizedBallotId.length === 4
            ? `${normalizedBallotId}%`
            : normalizedBallotId
        : ""

    const dispatch = useAppDispatch()

    const {data, loading} = useQuery<GetCastVoteQuery>(GET_CAST_VOTE, {
        variables: {
            tenantId,
            electionEventId: eventId,
            electionId,
            ballotIdPattern,
        },
        skip: globalSettings.DISABLE_AUTH || !hasBallotId || loadingElections,
    })

    const validatedBallotId = isHex(inputBallotId ?? "")

    const matchingBallots = data?.["sequent_backend_cast_vote"] ?? []
    const ambiguousBallotId = matchingBallots.length > 1
    const ballotContent = matchingBallots.length === 1 ? matchingBallots[0].content : null
    const lookupLoading = loadingElections || loading

    const locate = (withBallotId = false) => {
        let id = withBallotId ? inputBallotId.trim() : ""

        setInputBallotId("")

        navigate(
            `/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${id}${location.search}`
        )
    }

    const captureEnter: React.KeyboardEventHandler<HTMLDivElement> = (event) => {
        // Same condition as the locate button: a rejected id stays in the input
        // with its error.
        if ("Enter" === event.key && validatedBallotId && inputBallotId.trim() !== "") {
            locate(true)
        }
    }

    return (
        <Stack
            className={`ballot-locator-content ${hasBallotId ? "ballot-locator-result" : "ballot-locator-search"}`}
        >
            <Box className="stepper-box" marginTop="48px">
                <BreadCrumbSteps
                    labels={["ballotLocator.steps.lookup", "ballotLocator.steps.result"]}
                    selected={hasBallotId ? 1 : 0}
                />
            </Box>

            <Box
                className="ballot-locator-header"
                sx={{
                    display: "flex",
                    flexDirection: {xs: "column", md: "row"},
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                }}
            >
                <Box
                    className="ballot-locator-heading"
                    sx={{
                        order: {xs: 2, md: 1},
                    }}
                >
                    <StyledTitle className="screen-title" variant="h1">
                        {!hasBallotId ? (
                            <Box className="screen-title-text">{t("ballotLocator.title")}</Box>
                        ) : (
                            <Box className="screen-title-text">
                                {t("ballotLocator.titleResult")}
                            </Box>
                        )}
                        <IconButton
                            buttonClassName="screen-help-button"
                            icon={faCircleQuestion}
                            sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                            fontSize="16px"
                            onClick={() => setOpenTitleHelp(true)}
                            ariaLabel={t("a11y.helpAbout", {
                                topic: t("ballotLocator.titleHelpDialog.title"),
                            })}
                        />
                        <Dialog
                            className="screen-help-dialog ballot-locator-help-dialog"
                            handleClose={() => setOpenTitleHelp(false)}
                            open={openTitleHelp}
                            title={t("ballotLocator.titleHelpDialog.title")}
                            ok={t("ballotLocator.titleHelpDialog.ok")}
                            variant="info"
                        >
                            {stringToHtml(t("ballotLocator.titleHelpDialog.content"))}
                        </Dialog>
                    </StyledTitle>

                    <Typography
                        className="screen-description"
                        variant="body1"
                        component="div"
                        sx={{color: theme.palette.customGrey.contrastText}}
                    >
                        {stringToHtml(t("ballotLocator.description"))}
                    </Typography>
                </Box>
            </Box>

            {/* The live region is always mounted so that the lookup result is
                announced when the text appears inside it. */}
            <Box className="ballot-lookup-status" role="status">
                {hasBallotId && !lookupLoading ? (
                    ambiguousBallotId ? (
                        <MessageFailed className="ballot-locator-failure">
                            {translateHtml(t, "ballotLocator.ambiguous", {ballotId})}
                        </MessageFailed>
                    ) : ballotContent ? (
                        <MessageSuccess className="ballot-locator-success">
                            {translateHtml(t, "ballotLocator.found", {ballotId})}
                        </MessageSuccess>
                    ) : (
                        <MessageFailed className="ballot-locator-failure">
                            {translateHtml(t, "ballotLocator.notFound", {ballotId})}
                        </MessageFailed>
                    )
                ) : null}
            </Box>
            {!hasBallotId && (
                <BallotIdInput
                    inputBallotId={inputBallotId}
                    setInputBallotId={setInputBallotId}
                    validatedBallotId={validatedBallotId}
                    captureEnter={captureEnter}
                    placeholderLabel="ballotLocator.description"
                />
            )}
            {hasBallotId && ballotContent && (
                <>
                    <Typography className="ballot-content-description" component="div">
                        {stringToHtml(t("ballotLocator.contentDesc"))}
                    </Typography>
                    <InfoDataBox className="ballot-content">{ballotContent}</InfoDataBox>
                </>
            )}

            {!hasBallotId ? (
                <Button
                    sx={{marginTop: "10px", width: "fit-content"}}
                    disabled={!validatedBallotId || inputBallotId.trim() === ""}
                    className="normal locate-ballot-button"
                    onClick={() => locate(true)}
                >
                    <span className="locate-ballot-label">{t("ballotLocator.locate")}</span>
                </Button>
            ) : (
                <>
                    <Button
                        sx={{marginTop: "10px", width: "fit-content"}}
                        className="normal locate-again-button"
                        onClick={() => locate()}
                    >
                        <span className="locate-ballot-again-label">
                            {t("ballotLocator.locateAgain")}
                        </span>
                    </Button>
                </>
            )}
        </Stack>
    )
}

export default BallotLocator
