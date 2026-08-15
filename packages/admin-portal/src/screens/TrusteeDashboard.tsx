// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useEffect, useRef, useCallback} from "react"
import {
    Card,
    CardContent,
    CardHeader,
    Grid,
    TextField,
    Button,
    Typography,
    LinearProgress,
    Alert,
    Box,
    Paper,
    Divider,
    Chip,
    List,
    ListItem,
    ListItemButton,
    ListItemText,
    IconButton,
    Collapse,
    Tooltip,
} from "@mui/material"
import {
    PlayArrow as PlayIcon,
    Pause as PauseIcon,
    Refresh as RefreshIcon,
    Clear as ClearIcon,
    Terminal as TerminalIcon,
} from "@mui/icons-material"
import {useInterval} from "react-use"

// Import the WASM package (already in your dependencies)
import init, {initThreadPool, WasmSession, WasmVerifier} from "braid-wasm"

let trustee: any = null

interface Config {
    name: string
    signing_key_sk: string
    signing_key_pk: string
    encryption_key: string
    b4_url: string
}

interface Board {
    name: string
}
interface State {
    current_messages?: number
    max_messages?: number
    board_name?: string
}

interface Action {
    ts: string
    msg: string
    added: number
    posted: number
}

export const TrusteeDashboard = () => {
    const [config, setConfig] = useState<Config>({
        name: "browser-trustee-1",
        signing_key_sk: "",
        signing_key_pk: "",
        encryption_key: "",
        b4_url: "http://127.0.0.1:50051",
    })

    const [initialized, setInitialized] = useState(false)
    const [connected, setConnected] = useState(false)
    const [boards, setBoards] = useState<Board[]>([])
    const [selectedBoard, setSelectedBoard] = useState("")
    const [manualBoard, setManualBoard] = useState("")
    const [state, setState] = useState<State>({})
    const [lastAction, setLastAction] = useState("—")
    const [boardSummary, setBoardSummary] = useState<any[]>([])
    const [actionHistory, setActionHistory] = useState<Array<Action>>([])
    const [consoleLog, setConsoleLog] = useState<string[]>([])
    const [autoRun, setAutoRun] = useState(false)
    const [loading, setLoading] = useState(false)
    const [storageInfo, setStorageInfo] = useState<any>(null)

    const consoleEndRef = useRef<HTMLDivElement>(null)

    const log = useCallback((msg: string, type: "info" | "warn" | "error" = "info") => {
        const timestamp = new Date().toLocaleTimeString()
        setConsoleLog((prev) => [...prev.slice(-500), `[${timestamp}] ${msg}`])
    }, [])

    useEffect(() => {
        consoleEndRef.current?.scrollIntoView({behavior: "smooth"})
    }, [consoleLog])

    // Load WASM on mount
    useEffect(() => {
        async function load() {
            try {
                await init({})
                await initThreadPool(navigator.hardwareConcurrency || 4)
                log("braid-wasm loaded and thread pool initialized")
            } catch (e: any) {
                log(`WASM init failed: ${e.message}`, "error")
            }
        }
        load()
    }, [])

    // Override console
    useEffect(() => {
        const origLog = console.log
        const origWarn = console.warn
        const origError = console.error

        console.log = (...args) => {
            origLog(...args)
            log(args.join(" "))
        }
        console.warn = (...args) => {
            origWarn(...args)
            log(args.join(" "), "warn")
        }
        console.error = (...args) => {
            origError(...args)
            log(args.join(" "), "error")
        }

        return () => {
            console.log = origLog
            console.warn = origWarn
            console.error = origError
        }
    }, [log])

    const handleInit = async () => {
        if (Object.values(config).some((v) => !v.trim())) {
            log("All fields required", "error")
            return
        }

        try {
            trustee = new WasmSession(JSON.stringify(config))
            setInitialized(true)
            log(`Trustee initialized: ${config.name}`)
        } catch (e: any) {
            log(`Init failed: ${e.message}`, "error")
        }
    }

    const fetchBoards = async () => {
        if (!trustee) return
        setLoading(true)
        try {
            const list = await trustee.fetch_boards()
            setBoards(list)
            log(`Found ${list.length} board(s)`)
        } catch (e: any) {
            log(`Fetch failed: ${e.message}`, "error")
        } finally {
            setLoading(false)
        }
    }

    const connect = async (boardName: string) => {
        if (!trustee || !boardName) return
        try {
            await trustee.init_session(boardName)
            const res = await trustee.connect_to_board()
            setConnected(true)
            setSelectedBoard(boardName)

            const pendingMsg =
                res.pending > 0
                    ? `${res.pending} remote message${res.pending === 1 ? "" : "s"} pending`
                    : "no new messages"
            log(`Connected to ${boardName} (${pendingMsg})`)

            updateState()
            updateBoard()
            updateStorageInfo()
        } catch (e: any) {
            log(`Connect failed: ${e.message}`, "error")
        }
    }

    const updateState = async () => {
        if (!trustee) return
        try {
            const s = await trustee.get_state()
            setState(s)
        } catch (e) {
            console.log(e)
        }
    }

    const updateBoard = async () => {
        if (!trustee) return
        try {
            const summary = await trustee.get_board_summary()
            setBoardSummary(summary)
        } catch (e: any) {
            log(`Board update failed: ${e.message}`, "error")
        }
    }

    const updateStorageInfo = async () => {
        if (!trustee) return
        try {
            const info = await trustee.get_storage_info()
            setStorageInfo(info)
        } catch (e: any) {
            log(`Storage info update failed: ${e.message}`, "error")
        }
    }

    const step = async () => {
        if (!trustee || loading) return
        setLoading(true)
        try {
            const res = await trustee.step()
            await updateState()
            await updateBoard()
            await updateStorageInfo()

            const action = res.actions?.length
                ? res.actions.join(", ")
                : `${res.added || 0} added, ${res.posted || 0} posted`

            setLastAction(action === "0 added, 0 posted" ? "Idle" : action)

            if (action !== "Idle" && action !== "0 added, 0 posted") {
                setActionHistory((prev) => [
                    {
                        ts: new Date().toLocaleTimeString(),
                        msg: action,
                        added: res.added || 0,
                        posted: res.posted || 0,
                    },
                    ...prev.slice(0, 99),
                ])
            }
        } catch (e: any) {
            log(`Step error: ${e.message}`, "error")
        } finally {
            setLoading(false)
        }
    }

    useInterval(step, autoRun ? 1000 : null)

    const progress = state.max_messages
        ? Math.round(((state.current_messages || 0) / state.max_messages) * 100)
        : 0

    return (
        <Box p={3}>
            <Typography variant="h4" gutterBottom>
                Braid Trustee Node
            </Typography>

            <Grid container spacing={3}>
                {/* Config */}
                <Grid size={{xs: 12, lg: 6}}>
                    <Card>
                        <CardHeader title="Trustee Configuration" />
                        <CardContent>
                            <Grid container spacing={2}>
                                <Grid size={12}>
                                    <TextField
                                        label="Trustee Name"
                                        fullWidth
                                        value={config.name}
                                        onChange={(e) =>
                                            setConfig({...config, name: e.target.value})
                                        }
                                    />
                                </Grid>
                                <Grid size={12}>
                                    <TextField
                                        label="Signing Key SK (Base64 DER)"
                                        multiline
                                        rows={3}
                                        fullWidth
                                        value={config.signing_key_sk}
                                        onChange={(e) =>
                                            setConfig({...config, signing_key_sk: e.target.value})
                                        }
                                    />
                                </Grid>
                                <Grid size={12}>
                                    <TextField
                                        label="Signing Key PK (Base64 DER)"
                                        multiline
                                        rows={3}
                                        fullWidth
                                        value={config.signing_key_pk}
                                        onChange={(e) =>
                                            setConfig({...config, signing_key_pk: e.target.value})
                                        }
                                    />
                                </Grid>
                                <Grid size={12}>
                                    <TextField
                                        label="Encryption Key (Base64)"
                                        fullWidth
                                        value={config.encryption_key}
                                        onChange={(e) =>
                                            setConfig({...config, encryption_key: e.target.value})
                                        }
                                    />
                                </Grid>
                                <Grid size={12}>
                                    <TextField
                                        label="B4 Server URL"
                                        fullWidth
                                        value={config.b4_url}
                                        onChange={(e) =>
                                            setConfig({...config, b4_url: e.target.value})
                                        }
                                    />
                                </Grid>
                                <Grid size={12}>
                                    <Button
                                        variant="contained"
                                        color="success"
                                        fullWidth
                                        onClick={handleInit}
                                        disabled={initialized}
                                    >
                                        {initialized ? "Initialized" : "Initialize Trustee"}
                                    </Button>
                                </Grid>
                            </Grid>
                        </CardContent>
                    </Card>
                </Grid>

                {/* Execution */}
                <Grid size={{xs: 12, lg: 6}}>
                    <Card>
                        <CardHeader title="Protocol Execution" />
                        <CardContent>
                            <Box display="flex" gap={2} mb={2}>
                                <Button
                                    variant="contained"
                                    startIcon={<PlayIcon />}
                                    onClick={step}
                                    disabled={!connected || loading || autoRun}
                                >
                                    Execute Step
                                </Button>
                                <Button
                                    variant={autoRun ? "contained" : "outlined"}
                                    color={autoRun ? "error" : "primary"}
                                    startIcon={autoRun ? <PauseIcon /> : <PlayIcon />}
                                    onClick={() => setAutoRun(!autoRun)}
                                    disabled={!connected}
                                >
                                    {autoRun ? "Pause" : "Auto Run"}
                                </Button>
                            </Box>

                            <Grid container spacing={2} mb={2}>
                                <Grid size={4}>
                                    <Paper variant="outlined" sx={{p: 2}}>
                                        <Typography variant="caption">Messages</Typography>
                                        <Typography variant="h5">
                                            {state.current_messages || 0} /{" "}
                                            {state.max_messages || 0}
                                        </Typography>
                                    </Paper>
                                </Grid>
                                <Grid size={4}>
                                    <Paper variant="outlined" sx={{p: 2}}>
                                        <Typography variant="caption">Last Action</Typography>
                                        <Typography variant="body2" noWrap>
                                            {lastAction}
                                        </Typography>
                                    </Paper>
                                </Grid>
                                <Grid size={4}>
                                    <Paper variant="outlined" sx={{p: 2}}>
                                        <Typography variant="caption">Board</Typography>
                                        <Typography variant="body2">
                                            {state.board_name || "-"}
                                        </Typography>
                                    </Paper>
                                </Grid>
                            </Grid>

                            <LinearProgress variant="determinate" value={progress} />
                            <Typography variant="body2" align="right" mt={1}>
                                {progress}%
                            </Typography>

                            <Collapse in={boardSummary.length > 0}>
                                <Box mt={3}>
                                    <Typography variant="subtitle2">
                                        Local Board ({boardSummary.length})
                                    </Typography>
                                    <List dense>
                                        {Object.entries(
                                            boardSummary.reduce<Record<string, any[]>>(
                                                (acc, s: any) => {
                                                    ;(acc[s.kind] ??= []).push(s)
                                                    return acc
                                                },
                                                {}
                                            )
                                        ).map(([kind, items]) => (
                                            <ListItem key={kind} disablePadding>
                                                <ListItemButton>
                                                    <ListItemText
                                                        primary={kind}
                                                        secondary={items
                                                            .map((s: any) =>
                                                                s.signer === 4294967295
                                                                    ? "PM"
                                                                    : `T${s.signer}`
                                                            )
                                                            .join(", ")}
                                                    />
                                                </ListItemButton>
                                            </ListItem>
                                        ))}
                                    </List>
                                </Box>
                            </Collapse>
                        </CardContent>
                    </Card>
                </Grid>

                {/* Board Selection */}
                <Grid size={{xs: 12, lg: 6}}>
                    <Card>
                        <CardHeader title="Board Selection" />
                        <CardContent>
                            <Box display="flex" gap={2}>
                                <Button
                                    variant="outlined"
                                    startIcon={<RefreshIcon />}
                                    onClick={fetchBoards}
                                    disabled={!initialized}
                                >
                                    Fetch Boards
                                </Button>
                            </Box>

                            {boards.length > 0 && (
                                <List>
                                    {boards.map((b) => (
                                        <ListItem key={b.name} disablePadding>
                                            <ListItemButton
                                                selected={selectedBoard === b.name}
                                                onClick={() => {
                                                    setManualBoard(b.name)
                                                    setSelectedBoard(b.name)
                                                }}
                                            >
                                                <ListItemText primary={b.name} />
                                            </ListItemButton>
                                        </ListItem>
                                    ))}
                                </List>
                            )}

                            <Box display="flex" gap={2} mt={2}>
                                <TextField
                                    label="Board name"
                                    fullWidth
                                    value={manualBoard}
                                    onChange={(e) => setManualBoard(e.target.value)}
                                    placeholder="Or type manually"
                                />
                                <Button
                                    variant="contained"
                                    onClick={() => connect(manualBoard || selectedBoard)}
                                    disabled={!initialized || !(manualBoard || selectedBoard)}
                                >
                                    Connect
                                </Button>
                            </Box>
                        </CardContent>
                    </Card>
                </Grid>

                {/* Storage Diagnostics */}
                <Grid size={{xs: 12, lg: 6}}>
                    <Card>
                        <CardHeader
                            title="💾 Storage Diagnostics"
                            action={
                                <Button
                                    size="small"
                                    startIcon={<RefreshIcon />}
                                    onClick={updateStorageInfo}
                                    disabled={!connected}
                                >
                                    Refresh
                                </Button>
                            }
                        />
                        <CardContent>
                            {storageInfo ? (
                                <Paper variant="outlined" sx={{p: 2}}>
                                    <Grid container spacing={2}>
                                        <Grid size={12}>
                                            <Typography variant="caption" color="text.secondary">
                                                Backend:
                                            </Typography>
                                            <Typography variant="body2">
                                                {storageInfo.backend_type}
                                            </Typography>
                                        </Grid>
                                        <Grid size={6}>
                                            <Typography variant="caption" color="text.secondary">
                                                Total Messages:
                                            </Typography>
                                            <Typography variant="body2">
                                                {storageInfo.total_messages}
                                            </Typography>
                                        </Grid>
                                        <Grid size={6}>
                                            <Typography variant="caption" color="text.secondary">
                                                Max Internal ID:
                                            </Typography>
                                            <Typography variant="body2" color="success.main">
                                                {storageInfo.max_internal_id}
                                            </Typography>
                                            <Typography variant="caption" sx={{fontSize: "0.7rem"}}>
                                                (locally-controlled, security-critical)
                                            </Typography>
                                        </Grid>
                                        <Grid size={12}>
                                            <Typography variant="caption" color="text.secondary">
                                                Max External ID:
                                            </Typography>
                                            <Typography variant="body2" color="warning.main">
                                                {storageInfo.max_external_id}
                                            </Typography>
                                            <Typography variant="caption" sx={{fontSize: "0.7rem"}}>
                                                (bulletin board ID, optimization only)
                                            </Typography>
                                        </Grid>
                                        {storageInfo.extra_info && (
                                            <Grid size={12}>
                                                <Divider sx={{my: 1}} />
                                                <Typography
                                                    variant="caption"
                                                    color="text.secondary"
                                                >
                                                    {storageInfo.extra_info}
                                                </Typography>
                                            </Grid>
                                        )}
                                    </Grid>
                                </Paper>
                            ) : (
                                <Typography color="text.secondary" sx={{fontStyle: "italic"}}>
                                    Connect to a board to see storage information
                                </Typography>
                            )}
                        </CardContent>
                    </Card>
                </Grid>

                {/* Console */}
                <Grid size={{xs: 12, lg: 6}}>
                    <Card>
                        <CardHeader
                            title="Console Output"
                            avatar={<TerminalIcon />}
                            action={
                                <IconButton onClick={() => setConsoleLog([])}>
                                    <ClearIcon />
                                </IconButton>
                            }
                        />
                        <CardContent>
                            <Paper
                                variant="outlined"
                                sx={{
                                    bgcolor: "#1e1e1e",
                                    color: "#d4d4d4",
                                    p: 2,
                                    fontFamily: "Monospace",
                                    fontSize: "0.8rem",
                                    maxHeight: 400,
                                    overflow: "auto",
                                }}
                            >
                                {consoleLog.length === 0 ? (
                                    <Typography color="gray">No logs yet</Typography>
                                ) : (
                                    consoleLog.map((line, i) => <div key={i}>{line}</div>)
                                )}
                                <div ref={consoleEndRef} />
                            </Paper>
                        </CardContent>
                    </Card>
                </Grid>
            </Grid>
        </Box>
    )
}
