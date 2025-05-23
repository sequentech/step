// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
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
} from "@sequentech/ui-essentials"
import {stringToHtml, EShowCastVoteLogsPolicy} from "@sequentech/ui-core"
import {Box, TextField, Typography, Button, Stack} from "@mui/material"
import {styled} from "@mui/material/styles"
import Tabs from "@mui/material/Tabs"
import Tab from "@mui/material/Tab"
import {Link, useLocation, useNavigate, useParams} from "react-router-dom"
import {GET_CAST_VOTE} from "../queries/GetCastVote"
import {useQuery, useMutation} from "@apollo/client"
import {
    GetBallotStylesQuery,
    GetCastVoteQuery,
    GetElectionEventQuery,
    ListCastVoteMessagesQuery,
} from "../gql/graphql"
import {faAngleLeft, faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {LIST_CAST_VOTE_MESSAGES} from "../queries/listCastVoteMessages"
import {updateBallotStyleAndSelection} from "../services/BallotStyles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectFirstBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import useLanguage from "../hooks/useLanguage"
import {SettingsContext} from "../providers/SettingsContextProvider"
import useUpdateTranslation from "../hooks/useUpdateTranslation"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {IElectionEvent} from "../store/electionEvents/electionEventsSlice"
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

const StyledLink = styled(Link)`
    text-decoration: none;
`

const StyledTitle = styled(Typography)`
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

const StyledError = styled(Typography)`
    position: absolute;
    margin-top: -12px;
    color: ${({theme}) => theme.palette.red.main};
`

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

const StyledApp = styled(Stack)<{css: string}>`
    min-height: 100vh;
    min-width: 100vw;
    ${({css}) => css}
`

interface TabPanelProps {
    children?: React.ReactNode
    index: number
    value: number
}

const CustomTabPanel: React.FC<TabPanelProps> = ({children, index, value}) => {
    return (
        <div
            role="tabpanel"
            hidden={value !== index}
            id={`simple-tabpanel-${index}`}
            aria-labelledby={`simple-tab-${index}`}
        >
            {value === index && <Box sx={{p: 3}}>{children}</Box>}
        </div>
    )
}

const BallotLocator: React.FC = () => {
    const {t} = useTranslation()
    const location = useLocation()
    const {tenantId, eventId, electionId} = useParams()
    const allowSendRequest = useRef<boolean>(true)
    const [value, setValue] = React.useState(0)
    const [inputBallotId, setInputBallotId] = useState("")
    const [rows, setRows] = useState<ICastVoteEntry[]>([])
    const [total, setTotal] = useState(0)
    const [ballotIdNotFoundErr, setBallotIdNotFoundErr] = useState(false)
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

    const {refetch} = useQuery<ListCastVoteMessagesQuery>(LIST_CAST_VOTE_MESSAGES, {
        variables: {
            tenantId,
            electionEventId: eventId,
            electionId,
            ballotId: inputBallotId,
        },
        skip: true,
    })

    useUpdateTranslation({
        electionEvent: dataElectionEvent?.sequent_backend_election_event[0] as IElectionEvent,
    }) // Overwrite translations
    const customCss = dataElectionEvent?.sequent_backend_election_event[0]?.presentation?.css
    let fetchTimeout: any = useRef()

    const requestCVMsgs = async (headerName?: string, newOrder?: string) => {
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
                // TODO: Notify to the user.
                console.log("ERROR")
                console.log(e)
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

    const captureEnter: React.KeyboardEventHandler<HTMLDivElement> = (event) => {
        allowSendRequest.current = true
    }

    return (
        <Box width={"100%"} maxWidth={"lg"} marginTop="48px">
            <Box sx={{borderBottom: 1, borderColor: "divider"}}>
                <Tabs
                    variant="scrollable"
                    allowScrollButtonsMobile
                    scrollButtons="auto"
                    indicatorColor="primary"
                    textColor="inherit"
                    sx={{fontFamily: "Roboto"}}
                    aria-label="ballot locator tabs"
                    value={value}
                    onChange={handleChange}
                >
                    <Tab label="BALLOT LOCATOR" {...a11yProps(0)} />
                    {showCVLogsPolicy && <Tab label="LOGS" {...a11yProps(1)} />}
                </Tabs>
            </Box>
            <CustomTabPanel value={value} index={0}>
                <BallotLocatorLogic customCss={customCss} />
            </CustomTabPanel>
            <CustomTabPanel value={value} index={1}>
                <Box marginTop="48px">
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
                />
            </CustomTabPanel>
            <Box sx={{order: {xs: 1, md: 2}, marginTop: "20px"}}>
                <StyledLink
                    to={`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`}
                >
                    <Button variant="secondary" className="secondary">
                        <Icon icon={faAngleLeft} size="sm" />
                        <Box paddingLeft="12px">{t("votingScreen.backButton")}</Box>
                    </Button>
                </StyledLink>
            </Box>
        </Box>
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
}

const LogsTable: React.FC<LogsTableProps> = ({
    rows,
    total,
    onOrderBy,
    rowsPerPage,
    handleChangeRowsPerPage,
    page,
    handleChangePage,
}) => {
    const {t} = useTranslation()
    const [orderBy, setOrderBy] = useState<string>("")
    const [order, setOrder] = useState<"desc" | "asc" | undefined>("desc")

    const onClickHeader = (headerName: string) => {
        setOrderBy(headerName)
        const newOrder = order === "desc" ? "asc" : "desc"
        setOrder(newOrder)
        onOrderBy?.(headerName, newOrder)
    }

    return (
        <>
            <StyledTitle variant="h5">{t("ballotLocator.totalBallots", {total})}</StyledTitle>
            <TableContainer component={Paper}>
                <Table sx={{minWidth: 650}} aria-label="simple table">
                    <TableHead>
                        <TableRow>
                            <TableCell align="justify" sx={{fontWeight: "bold"}}>
                                <TableSortLabel
                                    active={orderBy === "username"}
                                    direction={orderBy === "username" ? order : "asc"}
                                    onClick={() => onClickHeader("username")}
                                >
                                    username
                                </TableSortLabel>
                            </TableCell>
                            <TableCell align="justify" sx={{fontWeight: "bold"}}>
                                ballot_id
                            </TableCell>
                            <TableCell align="justify" sx={{fontWeight: "bold"}}>
                                <TableSortLabel
                                    active={orderBy === "statement_kind"}
                                    direction={orderBy === "statement_kind" ? order : "asc"}
                                    onClick={() => onClickHeader("statement_kind")}
                                >
                                    statement_kind
                                </TableSortLabel>
                            </TableCell>
                            <TableCell align="justify" sx={{fontWeight: "bold"}}>
                                <TableSortLabel
                                    active={orderBy === "statement_timestamp"}
                                    direction={orderBy === "statement_timestamp" ? order : "asc"}
                                    onClick={() => onClickHeader("statement_timestamp")}
                                >
                                    statement_timestamp
                                </TableSortLabel>
                            </TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {rows.map((row, index) => (
                            <TableRow key={index}>
                                <TableCell align="justify">{row.username ?? "****"}</TableCell>
                                <TableCell align="justify">{row.ballot_id}</TableCell>
                                <TableCell align="justify">{row.statement_kind}</TableCell>
                                <TableCell align="justify">
                                    {new Date(row.statement_timestamp * 1000).toUTCString()}
                                </TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </TableContainer>
            <TablePagination
                rowsPerPageOptions={[5, 10, 25, 50]}
                component="div"
                count={total}
                rowsPerPage={rowsPerPage}
                page={page}
                onPageChange={handleChangePage}
                onRowsPerPageChange={handleChangeRowsPerPage}
            />
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

    return (
        <>
            <TextField
                onChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                    setInputBallotId(event.target.value)
                }}
                value={inputBallotId}
                InputLabelProps={{
                    shrink: true,
                }}
                label="Ballot ID"
                placeholder={t(placeholderLabel)}
                onKeyDown={captureEnter}
            />
            {!validatedBallotId && (
                <StyledError>{t("ballotLocator.wrongFormatBallotId")}</StyledError>
            )}
            {ballotIdNotFoundErr && validatedBallotId && (
                <StyledError>{t("ballotLocator.ballotIdNotFoundAtFilter")}</StyledError>
            )}
        </>
    )
}

interface BallotLocatorLogicProps {
    customCss: any
}

const BallotLocatorLogic: React.FC<BallotLocatorLogicProps> = ({customCss}) => {
    const {tenantId, eventId, electionId, ballotId} = useParams()
    const [openTitleHelp, setOpenTitleHelp] = useState<boolean>(false)
    const navigate = useNavigate()
    const location = useLocation()
    const {t} = useTranslation()
    const [inputBallotId, setInputBallotId] = useState<string>("")
    const {globalSettings} = useContext(SettingsContext)
    const hasBallotId = !!ballotId
    const {data: dataBallotStyles} = useQuery<GetBallotStylesQuery>(GET_BALLOT_STYLES)

    const dispatch = useAppDispatch()
    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    useLanguage({ballotStyle})

    const {data, loading} = useQuery<GetCastVoteQuery>(GET_CAST_VOTE, {
        variables: {
            tenantId,
            electionEventId: eventId,
            electionId,
            ballotId,
        },
        skip: globalSettings.DISABLE_AUTH || !hasBallotId, // Skip query if in demo mode
    })

    useEffect(() => {
        if (dataBallotStyles && dataBallotStyles.sequent_backend_ballot_style.length > 0) {
            updateBallotStyleAndSelection(dataBallotStyles, dispatch)
        }
    }, [dataBallotStyles, dispatch])

    const validatedBallotId = isHex(inputBallotId ?? "")

    const ballotContent =
        data?.["sequent_backend_cast_vote"]?.find((item) => item.ballot_id === ballotId)?.content ??
        null

    const locate = (withBallotId = false) => {
        let id = withBallotId ? inputBallotId : ""

        setInputBallotId("")

        navigate(
            `/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${id}${location.search}`
        )
    }

    const captureEnter: React.KeyboardEventHandler<HTMLDivElement> = (event) => {
        if ("Enter" === event.key) {
            locate(true)
        }
    }

    const ConditionalStyledApp = customCss ? StyledApp : Stack

    return (
        <ConditionalStyledApp css={customCss}>
            <PageLimit className="ballot-locator-screen screen" maxWidth="lg">
                <Box marginTop="48px">
                    <BreadCrumbSteps
                        labels={["ballotLocator.steps.lookup", "ballotLocator.steps.result"]}
                        selected={hasBallotId ? 1 : 0}
                    />
                </Box>

                <Box
                    sx={{
                        display: "flex",
                        flexDirection: {xs: "column", md: "row"},
                        justifyContent: "space-between",
                        alignItems: "flex-start",
                    }}
                >
                    <Box
                        sx={{
                            order: {xs: 2, md: 1},
                        }}
                    >
                        <StyledTitle variant="h1">
                            {!hasBallotId ? (
                                <Box>{t("ballotLocator.title")}</Box>
                            ) : (
                                <Box>{t("ballotLocator.titleResult")}</Box>
                            )}
                            <IconButton
                                icon={faCircleQuestion}
                                sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                                fontSize="16px"
                                onClick={() => setOpenTitleHelp(true)}
                            />
                            <Dialog
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
                            variant="body1"
                            sx={{color: theme.palette.customGrey.contrastText}}
                        >
                            {t("ballotLocator.description")}
                        </Typography>
                    </Box>
                </Box>

                {hasBallotId && !loading && (
                    <Box>
                        {hasBallotId && !!ballotContent ? (
                            <MessageSuccess>{t("ballotLocator.found", {ballotId})}</MessageSuccess>
                        ) : (
                            <MessageFailed>{t("ballotLocator.notFound", {ballotId})}</MessageFailed>
                        )}
                    </Box>
                )}
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
                        <Typography>{t("ballotLocator.contentDesc")}</Typography>
                        <InfoDataBox>{ballotContent}</InfoDataBox>
                    </>
                )}

                {!hasBallotId ? (
                    <Button
                        sx={{marginTop: "10px"}}
                        disabled={!validatedBallotId || inputBallotId.trim() === ""}
                        className="normal"
                        onClick={() => locate(true)}
                    >
                        <span>{t("ballotLocator.locate")}</span>
                    </Button>
                ) : (
                    <>
                        <Button
                            sx={{marginTop: "10px"}}
                            className="normal"
                            onClick={() => locate()}
                        >
                            <span>{t("ballotLocator.locateAgain")}</span>
                        </Button>
                    </>
                )}
            </PageLimit>
        </ConditionalStyledApp>
    )
}

export default BallotLocator
