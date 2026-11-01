// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState, useRef} from "react"
import {useTranslation} from "react-i18next"
import {isBallotIdInput} from "../services/BallotIdLookup"
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
    zoneName,
    EShowCastVoteLogsPolicy,
    EBallotIdentity,
    formatVotingPortalDateTime,
    translateHtml,
    stringToText,
    getChecksPeriod,
    EChecksPeriodStatus,
    IElectionEventPresentation,
} from "@sequentech/ui-core"
import {Box, TextField, Typography, Button, Stack, TypographyProps} from "@mui/material"
import {styled} from "@mui/material/styles"
import {visuallyHidden} from "@mui/utils"
import Tabs from "@mui/material/Tabs"
import Tab from "@mui/material/Tab"
import {Link, useLocation, useNavigate, useParams} from "react-router-dom"
import {BallotIdField} from "../components/BallotLocator/BallotIdField"
import {BallotMatches, EMatchesLook, IBallotMatch} from "../components/BallotLocator/BallotMatches"
import {TrackerAudit} from "../components/BallotLocator/TrackerAudit"
import {TrackerResult} from "../components/BallotLocator/TrackerResult"
import {TrackerSearch} from "../components/BallotLocator/TrackerSearch"
import {PHONE} from "../components/BallotLocator/trackerStyle"
import {useTrackedReceipt} from "../hooks/useTrackedReceipt"
import {useVitrinePolicy} from "../hooks/useVitrinePolicy"
import {
    ballotIdChars,
    EBallotIdInput,
    groupBallotId,
    shortBallotId,
    typedBallotId,
} from "../services/vitrine/ballotId"
import {authorityContact} from "../services/vitrine/auditEvidence"
import {auditBallot, EAuditOutcome, provideAuditService} from "../services/vitrine/ballotAudit"
import {
    acceptedAtOf,
    ballotLookupOf,
    classifyRecord,
    ERecordKind,
    trackerInput,
} from "../services/vitrine/ballotTracker"
import {vitrineLanguage} from "../services/vitrine/language"
import {ITransparency, readTransparency} from "../services/vitrine/presentation"
import {ETransparencyMode} from "../services/vitrine/types"
import {ELocateBallotStatus, LOCATE_BALLOT} from "../queries/LocateBallot"
import {useQuery} from "@apollo/client/react"
import {
    LocateBallotQuery,
    GetElectionEventQuery,
    GetElectionsQuery,
    ListCastVoteMessagesQuery,
} from "../gql/graphql"
import {faAngleLeft, faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
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
import {BallotBoxLog} from "../components/BallotBoxLog"
import {ICastVoteEntry} from "../types/castVoteLogEntry"
import {selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {ballotTimeZones, formatBallotTime} from "../services/ElectionTimeZones"

const StyledTitle = styled(Typography)<{component?: React.ElementType}>`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 1.5rem;
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

function isHex(str: string) {
    if (str.trim() === "") {
        return true
    }
    if (str.length % 2 !== 0) {
        return false
    }
    const regex = /^[0-9a-fA-F]+$/
    return regex.test(str)
}

const matchOf = (vote: {
    ballot_id?: string | null
    created_at?: string | null
    content?: string | null
}): IBallotMatch[] =>
    vote.ballot_id
        ? [
              {
                  ballotId: vote.ballot_id,
                  kind: classifyRecord(vote.content).kind,
                  ...(vote.created_at ? {createdAt: vote.created_at} : {}),
              },
          ]
        : []

interface IBackButtonProps {
    to: string
}

const BackButton: React.FC<IBackButtonProps> = ({to}) => {
    const {t} = useTranslation()
    return (
        <Button component={Link} to={to} variant="secondary" className="secondary back-button">
            <Icon className="back-button-icon" icon={faAngleLeft} size="sm" />
            <Box className="back-button-label">{t("votingScreen.backButton")}</Box>
        </Button>
    )
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
    const [ballotIdNotFoundErr, setBallotIdNotFoundErr] = useState(false)
    const [somethingWentWrongErr, setSomethingWentWrongErr] = useState(false)
    const validatedBallotId = isHex(inputBallotId ?? "")
    const [showCVLogsPolicy, setShowCVLogsPolicy] = useState(false)
    const {globalSettings} = useContext(SettingsContext)
    const [page, setPage] = React.useState(0)
    const [rowsPerPage, setRowsPerPage] = React.useState(5)
    const lastCVRequestTimestamp = useRef<number | undefined>(undefined) // Timestamp of last LIST_CAST_VOTE_MESSAGES request
    const {data: dataElectionEvent, loading: electionEventLoading} =
        useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {
            variables: {
                electionEventId: eventId,
                tenantId,
            },
            skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
        })

    // The same (cached) query as the lookup panel: the election's zone.
    const {data: dataElections} = useQuery<GetElectionsQuery>(GET_ELECTIONS, {
        variables: {electionIds: electionId ? [electionId] : []},
        skip: globalSettings.DISABLE_AUTH || !electionId,
    })
    const electionEventPresentation =
        dataElectionEvent?.sequent_backend_election_event[0]?.presentation
    const transparency = useMemo(
        () => readTransparency(electionEventPresentation),
        [electionEventPresentation]
    )
    const showTracker = transparency.mode !== ETransparencyMode.DISABLED
    const policy = useVitrinePolicy(transparency)
    const backTo = `/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(electionId ?? ""))
    const logsElectionEvent = dataElectionEvent?.sequent_backend_election_event[0] as
        | IElectionEvent
        | undefined
    // Log times show in the Post's zone, named in words (as before for an
    // event without timezones).
    const zones = ballotTimeZones(
        logsElectionEvent?.presentation,
        dataElections?.sequent_backend_election.find((item) => item.id === electionId)
            ?.presentation,
        ballotStyle?.ballot_eml
    )
    const pseudonymous =
        logsElectionEvent?.presentation?.ballot_identity === EBallotIdentity.PSEUDONYM
    const formatLogTime = (timestamp: number) =>
        formatBallotTime(new Date(timestamp), {
            event: logsElectionEvent,
            zones,
            t,
            lang: i18n.resolvedLanguage || i18n.language,
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

    const requestCVMsgs = async (headerName?: string, newOrder?: string) => {
        if (pseudonymous) return
        let duration = lastCVRequestTimestamp.current
            ? Date.now() - lastCVRequestTimestamp.current
            : undefined
        let tooQuick = duration ? duration < 500 : false
        lastCVRequestTimestamp.current = Date.now()
        async function tryFetchMessages() {
            try {
                let limit = rowsPerPage
                let offset = page * rowsPerPage
                const {data} = await refetch({
                    ballotId: inputBallotId,
                    orderBy: {[headerName ?? "id"]: newOrder ?? "desc"},
                    limit,
                    offset,
                })
                console.log(data)
                if (data?.list_cast_vote_messages) {
                    setRows((data?.list_cast_vote_messages?.list ?? []) as ICastVoteEntry[])
                    setTotal(data?.list_cast_vote_messages?.total)
                    setBallotIdNotFoundErr(
                        inputBallotId.length > 0 && data?.list_cast_vote_messages?.list.length === 0
                    )
                }
            } catch (e) {
                setSomethingWentWrongErr(true)
            }
        }

        if (tooQuick) {
            // Start interval
            // if timeout is already running, destroy it and create a new one.
            clearTimeout(fetchTimeout.current)
            fetchTimeout.current = setTimeout(async () => {
                await tryFetchMessages()
            }, 1000)
        }

        if (globalSettings.DISABLE_AUTH || tooQuick || !validatedBallotId) {
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
        const presentation = dataElectionEvent?.sequent_backend_election_event[0]?.presentation as
            | IElectionEventPresentation
            | undefined
        const showLogs = presentation?.show_cast_vote_logs
        const checksEnded =
            getChecksPeriod(presentation, new Date()).status === EChecksPeriodStatus.ENDED
        setShowCVLogsPolicy(showLogs === EShowCastVoteLogsPolicy.SHOW_LOGS_TAB && !checksEnded)
        // the length must be an even number of characters
        if (
            showLogs === EShowCastVoteLogsPolicy.SHOW_LOGS_TAB &&
            !pseudonymous &&
            !checksEnded &&
            allowSendRequest.current
        ) {
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

    const captureEnter: React.KeyboardEventHandler<HTMLDivElement> = (event) => {
        allowSendRequest.current = true
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
            <Box
                className="ballot-locator-panels"
                sx={{p: 3, ...(showTracker && value === 0 ? {pt: 0} : {})}}
            >
                <CustomTabPanel className="ballot-lookup-panel" value={value} index={0}>
                    <BallotLocatorLogic
                        electionEvent={
                            dataElectionEvent?.sequent_backend_election_event[0] as
                                | IElectionEvent
                                | undefined
                        }
                        transparency={transparency}
                        backTo={backTo}
                        electionEventLoading={electionEventLoading}
                    />
                </CustomTabPanel>
                <CustomTabPanel className="cast-vote-logs-panel" value={value} index={1}>
                    {pseudonymous && showCVLogsPolicy && eventId ? (
                        <BallotBoxLog
                            policy={policy}
                            electionEventId={eventId}
                            electionId={electionId}
                            timeZoneLabel={
                                zones
                                    ? zoneName(zones.timeZone, {
                                          t,
                                          lang: i18n.resolvedLanguage || i18n.language,
                                      })
                                    : undefined
                            }
                            formatTime={(timestamp) => formatLogTime(new Date(timestamp).getTime())}
                        />
                    ) : (
                        <>
                            <Box className="cast-vote-logs-filter" marginTop="48px">
                                <BallotIdInput
                                    inputBallotId={inputBallotId}
                                    setInputBallotId={setInputBallotId}
                                    validatedBallotId={validatedBallotId}
                                    ballotIdNotFoundErr={ballotIdNotFoundErr}
                                    captureEnter={captureEnter}
                                    placeholderLabel="ballotLocator.filterByBallotId"
                                />
                            </Box>
                            <LogsTable
                                rows={rows}
                                total={total}
                                onOrderBy={onClickHeader}
                                rowsPerPage={rowsPerPage}
                                handleChangeRowsPerPage={handleChangeRowsPerPage}
                                page={page}
                                handleChangePage={handleChangePage}
                                somethingWentWrongErr={somethingWentWrongErr}
                                formatDateTime={formatLogTime}
                            />
                        </>
                    )}
                </CustomTabPanel>
                {showTracker && value === 0 ? null : (
                    <Box
                        className="ballot-locator-actions"
                        sx={{
                            order: {xs: 1, md: 2},
                            marginTop: "20px",
                            width: "fit-content",
                        }}
                    >
                        <BackButton to={backTo} />
                    </Box>
                )}
            </Box>
        </PageLimit>
    )
}

interface LogsTableProps {
    rows: ICastVoteEntry[]
    total: number
    onOrderBy?: (headerName: string, newOrder: string) => void
    rowsPerPage: number
    handleChangeRowsPerPage: (event: React.ChangeEvent<HTMLInputElement>) => void
    page: number
    handleChangePage: (event: unknown, newValue: number) => void
    somethingWentWrongErr: boolean
    formatDateTime: (timestamp: number) => string
}

const LogsTable: React.FC<LogsTableProps> = ({
    rows,
    total,
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
                {t("ballotLocator.totalBallots", {total})}
            </StyledTitle>
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
                                sortDirection={orderBy === "statement_timestamp" ? order : false}
                                sx={{fontWeight: "bold", padding: "2px 4px"}}
                            >
                                <TableSortLabel
                                    className="cast-vote-logs-sort-button"
                                    active={orderBy === "statement_timestamp"}
                                    direction={orderBy === "statement_timestamp" ? order : "asc"}
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
                        </TableRow>
                    </TableHead>
                    <TableBody className="cast-vote-logs-table-body">
                        {rows.map((row, index) => (
                            <TableRow className="cast-vote-logs-row" key={index}>
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
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </TableContainer>
            <TablePagination
                className="cast-vote-logs-pagination"
                rowsPerPageOptions={[5, 10, 25, 50]}
                component="div"
                count={total}
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
    captureEnter: React.KeyboardEventHandler<HTMLDivElement>
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

interface IBallotLocatorLogicProps {
    electionEvent?: IElectionEvent
    electionEventLoading: boolean
    transparency: ITransparency
    /** Where Back goes. */
    backTo: string
}

const BallotLocatorLogic: React.FC<IBallotLocatorLogicProps> = ({
    electionEvent,
    electionEventLoading,
    transparency,
    backTo,
}) => {
    const {tenantId, eventId, electionId, ballotId} = useParams()
    const [openTitleHelp, setOpenTitleHelp] = useState<boolean>(false)
    const navigate = useNavigate()
    const location = useLocation()
    const {t, i18n} = useTranslation()
    const [inputBallotId, setInputBallotId] = useState<string>(() => typedBallotId(ballotId ?? ""))
    const {globalSettings} = useContext(SettingsContext)

    const hasBallotId = !!ballotId
    const checksPeriod = getChecksPeriod(
        electionEvent?.presentation as IElectionEventPresentation | undefined,
        new Date()
    )

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
    const ballotLookup = ballotLookupOf(ballotId)

    const {
        data,
        loading: locating,
        error: lookupError,
    } = useQuery<LocateBallotQuery>(LOCATE_BALLOT, {
        variables: {
            electionEventId: eventId,
            electionId,
            ballotId: ballotId ?? "",
        },
        // The event's checks period decides whether the ballot is looked up at all.
        skip:
            globalSettings.DISABLE_AUTH ||
            !hasBallotId ||
            electionEventLoading ||
            loadingElections ||
            checksPeriod.status === EChecksPeriodStatus.ENDED,
    })

    const lookupLoading = loadingElections || locating
    const lookupDone = hasBallotId && !lookupLoading && !electionEventLoading
    const validatedBallotId = isBallotIdInput(inputBallotId ?? "")

    const located = data?.locate_ballot
    const checksEnded =
        checksPeriod.status === EChecksPeriodStatus.ENDED ||
        located?.status === ELocateBallotStatus.CHECKS_ENDED
    const checksUntil = located?.checks_available_until
        ? new Date(located.checks_available_until)
        : "until" in checksPeriod
          ? checksPeriod.until
          : null
    const checksUntilText = checksUntil
        ? formatVotingPortalDateTime(
              checksUntil,
              electionEvent,
              i18n.resolvedLanguage || i18n.language
          )
        : ""
    const ambiguousBallotId = located?.status === ELocateBallotStatus.AMBIGUOUS
    const ambiguousMatches = ambiguousBallotId ? (located?.matches ?? []) : []
    const foundBallot =
        located?.status === ELocateBallotStatus.FOUND && located.ballot_id
            ? {
                  ballot_id: located.ballot_id,
                  content: located.content ?? null,
                  created_at: located.cast_at ?? null,
              }
            : undefined
    const ballotContent = foundBallot?.content ?? null
    const searching = !hasBallotId || ambiguousBallotId
    const showTracker = transparency.mode !== ETransparencyMode.DISABLED
    const eventPolicy = useVitrinePolicy(transparency)
    const trackedBallotId = foundBallot?.ballot_id ?? undefined
    const showTrackerResult = showTracker && !!trackedBallotId

    const record = useMemo(() => classifyRecord(ballotContent), [ballotContent])
    const audit = useMemo(
        () =>
            showTrackerResult && record.kind === ERecordKind.AUDITED && trackedBallotId
                ? auditBallot(provideAuditService(), record, trackedBallotId)
                : undefined,
        [showTrackerResult, record, trackedBallotId]
    )

    const tracked = useTrackedReceipt(
        eventId && electionId && foundBallot?.ballot_id
            ? {
                  eventId,
                  electionId,
                  ballotId: foundBallot.ballot_id,
                  stage: record.kind === ERecordKind.AUDITED ? "audited" : "cast",
                  acceptedAt: acceptedAtOf(foundBallot.created_at),
              }
            : undefined,
        transparency
    )

    const language = i18n.resolvedLanguage || i18n.language
    const formatTime = (time: number | string) =>
        formatVotingPortalDateTime(time, electionEvent, language)
    const electionPath = `/tenant/${tenantId}/event/${eventId}/election/${electionId}`
    const locatorPath = `${electionPath}/ballot-locator`
    const canLocate = [EBallotIdInput.SHORT, EBallotIdInput.PARTIAL, EBallotIdInput.FULL].includes(
        trackerInput(inputBallotId)
    )

    const locate = (withBallotId = false) => {
        if (withBallotId && !canLocate) {
            return
        }
        if (!withBallotId) {
            setInputBallotId("")
        }

        navigate(
            `${locatorPath}/${withBallotId ? ballotIdChars(inputBallotId) : ""}${location.search}`
        )
    }

    const descriptionKey = !audit
        ? "ballotLocator.description"
        : audit.outcome === EAuditOutcome.VERIFIED
          ? "ballotLocator.descriptionAudited"
          : "ballotLocator.descriptionAuditedBrief"

    const title = (
        <StyledTitle
            className="screen-title"
            variant="h1"
            sx={showTracker ? {margin: 0} : undefined}
        >
            <Box className="screen-title-text">
                {t(searching ? "ballotLocator.title" : "ballotLocator.titleResult")}
            </Box>
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
    )

    const description = (
        <Typography
            className="screen-description"
            variant="body1"
            component="div"
            sx={{color: theme.palette.customGrey.contrastText}}
        >
            {stringToHtml(t(descriptionKey))}
        </Typography>
    )

    return (
        <Stack
            className={`ballot-locator-content ${searching ? "ballot-locator-search" : "ballot-locator-result"}`}
            sx={showTracker && !searching ? {[PHONE]: {marginX: -3}} : undefined}
        >
            <Box
                className="stepper-box"
                sx={{
                    marginTop: showTracker ? "32px" : "48px",
                    ...(showTracker && !searching ? {[PHONE]: {marginTop: "16px"}} : {}),
                }}
            >
                <BreadCrumbSteps
                    labels={["ballotLocator.steps.lookup", "ballotLocator.steps.result"]}
                    selected={searching ? 0 : 1}
                />
            </Box>

            {showTracker ? (
                <>
                    <Box
                        className="ballot-locator-header"
                        sx={{
                            display: "flex",
                            flexWrap: "wrap",
                            alignItems: "center",
                            justifyContent: "space-between",
                            gap: "16px",
                            margin: "20px 0 16px",
                        }}
                    >
                        {title}
                        <Box
                            className="ballot-locator-title-actions"
                            sx={{
                                display: "flex",
                                flexWrap: "wrap",
                                gap: "8px",
                                ...(searching
                                    ? {}
                                    : {[PHONE]: {"width": "100%", "& > *": {flex: "1 1 0"}}}),
                            }}
                        >
                            <BackButton to={backTo} />
                            {searching ? null : (
                                <Button
                                    className="normal locate-again-button"
                                    onClick={() => locate()}
                                >
                                    <span className="locate-ballot-again-label">
                                        {t("ballotLocator.locateAgain")}
                                    </span>
                                </Button>
                            )}
                        </Box>
                    </Box>
                    {searching ? null : description}
                </>
            ) : (
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
                        {title}
                        {description}
                    </Box>
                </Box>
            )}

            {/* The live region is always mounted so that the lookup result is
                announced when the text appears inside it. */}
            <Box className="ballot-lookup-status" role="status">
                {checksEnded ? (
                    <Typography className="ballot-checks-ended" component="div">
                        {t("ballotLocator.checksEnded", {date: checksUntilText})}
                    </Typography>
                ) : lookupDone ? (
                    lookupError ? (
                        <MessageFailed className="ballot-locator-failure">
                            {stringToHtml(t("errors.page.somethingWrong"))}
                        </MessageFailed>
                    ) : ambiguousBallotId ? (
                        <MessageFailed className="ballot-locator-failure">
                            {translateHtml(
                                t,
                                ambiguousMatches.length
                                    ? "ballotLocator.ambiguous"
                                    : "ballotLocator.ambiguousMany",
                                {ballotId: groupBallotId(ballotId ?? "")}
                            )}
                            {ambiguousMatches.length > 0 && (
                                <Box component="ul" className="ballot-locator-matches">
                                    {ambiguousMatches.map((match) => (
                                        <li key={match} className="ballot-locator-match-item">
                                            <Button
                                                variant="text"
                                                className="ballot-locator-match"
                                                onClick={() =>
                                                    navigate(
                                                        `${locatorPath}/${ballotIdChars(match)}${location.search}`
                                                    )
                                                }
                                            >
                                                {groupBallotId(match)}
                                            </Button>
                                        </li>
                                    ))}
                                </Box>
                            )}
                        </MessageFailed>
                    ) : ballotContent ? (
                        showTracker ? (
                            <Box
                                className="ballot-found-status-text"
                                component="span"
                                sx={visuallyHidden}
                            >
                                {translateHtml(t, "ballotLocator.found", {
                                    ballotId: shortBallotId(trackedBallotId ?? ballotId ?? ""),
                                })}
                            </Box>
                        ) : (
                            <MessageSuccess
                                className="ballot-locator-success"
                                sx={{[PHONE]: {padding: "10px 14px"}}}
                            >
                                {translateHtml(t, "ballotLocator.found", {ballotId})}
                            </MessageSuccess>
                        )
                    ) : (
                        <MessageFailed className="ballot-locator-failure">
                            {translateHtml(t, "ballotLocator.notFound", {ballotId})}
                        </MessageFailed>
                    )
                ) : null}
            </Box>
            {showTrackerResult && !searching && ballotContent && ballotLookup?.ballotIdPrefix && (
                <Typography
                    className="ballot-found-short-note"
                    sx={{
                        margin: "10px 0 0",
                        textAlign: "center",
                        fontSize: 14,
                        lineHeight: "20px",
                        color: "#475569",
                    }}
                >
                    {t("ballotLocator.foundShort")}
                </Typography>
            )}
            {searching &&
                !checksEnded &&
                (showTracker ? (
                    <TrackerSearch
                        value={inputBallotId}
                        onChange={setInputBallotId}
                        onSubmit={() => locate(true)}
                        canSubmit={canLocate}
                        policyId={eventPolicy?.id}
                    >
                        <BallotMatches
                            look={EMatchesLook.CARDS}
                            matches={[]}
                            lookedFor={ballotId ?? ""}
                            hrefOf={(id) => `${locatorPath}/${id}${location.search}`}
                            formatTime={formatTime}
                        />
                    </TrackerSearch>
                ) : (
                    <>
                        <BallotIdField
                            value={inputBallotId}
                            onChange={setInputBallotId}
                            onSubmit={() => locate(true)}
                        />
                        <BallotMatches
                            matches={[]}
                            lookedFor={ballotId ?? ""}
                            hrefOf={(id) => `${locatorPath}/${id}${location.search}`}
                            formatTime={formatTime}
                        />
                    </>
                ))}
            {!searching && ballotContent && trackedBallotId && showTrackerResult && (
                <>
                    <TrackerResult
                        ballotId={trackedBallotId}
                        content={ballotContent}
                        audit={audit}
                        tracked={tracked}
                        policy={eventPolicy}
                        verifierUrl={globalSettings.BALLOT_VERIFIER_URL}
                        language={vitrineLanguage(i18n.language)}
                        formatTime={formatTime}
                    />
                    {audit && record.kind === ERecordKind.AUDITED ? (
                        <TrackerAudit
                            audit={audit}
                            ballotStyle={record.ballot.config}
                            language={i18n.language}
                            publicBucketUrl={globalSettings.PUBLIC_BUCKET_URL}
                            ballotId={trackedBallotId}
                            record={record}
                            voteHref={`${electionPath}/start${location.search}`}
                            contact={authorityContact(tracked.view?.policy ?? eventPolicy)}
                        />
                    ) : null}
                </>
            )}
            {!searching && !checksEnded && ballotContent && !showTrackerResult && (
                <>
                    {located?.cast_at && (
                        <Typography className="ballot-cast-at" component="div">
                            {t("ballotLocator.castAt", {
                                date: formatVotingPortalDateTime(
                                    located.cast_at,
                                    electionEvent,
                                    i18n.resolvedLanguage || i18n.language
                                ),
                            })}
                        </Typography>
                    )}
                    <Typography className="ballot-content-description" component="div">
                        {stringToHtml(t("ballotLocator.contentDesc"))}
                    </Typography>
                    <InfoDataBox className="ballot-content">{ballotContent}</InfoDataBox>
                </>
            )}

            {showTracker || checksEnded ? null : searching ? (
                <Button
                    sx={{marginTop: "10px", width: "fit-content"}}
                    disabled={!canLocate}
                    className="normal locate-ballot-button"
                    onClick={() => locate(true)}
                >
                    <span className="locate-ballot-label">{t("ballotLocator.locate")}</span>
                </Button>
            ) : (
                <Button
                    sx={{marginTop: "10px", width: "fit-content"}}
                    className="normal locate-again-button"
                    onClick={() => locate()}
                >
                    <span className="locate-ballot-again-label">
                        {t("ballotLocator.locateAgain")}
                    </span>
                </Button>
            )}
        </Stack>
    )
}

export default BallotLocator
