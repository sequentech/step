// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useState, useRef, useEffect} from "react"
import {
    Accordion,
    AccordionSummary,
    AccordionDetails,
    Alert,
    Box,
    Button,
    Card,
    CardContent,
    TextField,
    Typography,
} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {trusteeFilename} from "@/services/trusteeFilename"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {
    createBrowserTrustee,
    loadBrowserTrustee,
    listBrowserTrusteeBoards,
    runBrowserCycle,
    runExclusiveBrowserTrustee,
    type TrusteeEngine,
    type TrusteeKeys,
    type TrusteeHistory,
} from "@/services/BrowserTrustee"

interface TrusteeBoard {
    name: string
    kind: "DKG" | "TALLY"
    parent: string | null
}

const actionGrid = {
    "display": "grid",
    "gridTemplateColumns": {
        xs: "1fr",
        sm: "repeat(2, minmax(0, 1fr))",
        lg: "repeat(3, minmax(0, 1fr))",
    },
    "gap": 1.5,
    "& .MuiButton-root": {minHeight: 44},
}

const codeExample = {
    my: 2,
    p: 2,
    bgcolor: "action.hover",
    border: "1px solid",
    borderColor: "divider",
    borderRadius: 1,
    fontFamily: "monospace",
    fontSize: "0.875rem",
    lineHeight: 1.6,
    whiteSpace: "pre-wrap",
    overflowWrap: "anywhere",
}

export const TrusteeDashboard = () => {
    const auth = useContext(AuthContext)
    // The in-memory token Keycloak keeps fresh, as for every portal request.
    const accessToken = () => auth.getAccessToken() ?? ""
    const {globalSettings} = useContext(SettingsContext)
    const platform = globalSettings.HASURA_URL
    const trustee = auth.trustee || auth.username
    const keysFilename = trusteeFilename(trustee, "private-keys.toml")
    const requestFilename = trusteeFilename(trustee, "custody.json")
    const proofFilename = trusteeFilename(trustee, "signed-custody-proof.json")
    const [boards, setBoards] = useState<TrusteeBoard[]>([])
    const [error, setError] = useState("")
    const [loading, setLoading] = useState(false)
    const [loaded, setLoaded] = useState(false)
    const refresh = async () => {
        setLoading(true)
        setError("")
        try {
            const token = accessToken()
            if (!token) throw new Error("Sign in as a trustee to list assigned boards.")
            const assigned = await listBrowserTrusteeBoards(platform, token)
            setBoards(assigned)
            setLoaded(true)
            if (engine.current && !active.current)
                await perform(async () => readHistory(engine.current, assigned))
        } catch (error) {
            setError(error instanceof Error ? error.message : String(error))
        } finally {
            setLoading(false)
        }
    }
    const [boardUrl, setBoardUrl] = useState(globalSettings.WBRAID_BOARD_URL || "/b4")
    const [keys, setKeys] = useState<TrusteeKeys | null>(null)
    const [history, setHistory] = useState<TrusteeHistory | null>(null)
    const [historyError, setHistoryError] = useState("")
    const [running, setRunning] = useState(false)
    const [busy, setBusy] = useState(false)
    const [browserStatus, setBrowserStatus] = useState("")
    const [tallyId, setTallyId] = useState("")
    const keysInput = useRef<HTMLInputElement>(null)
    const backupInput = useRef<HTMLInputElement>(null)
    const custodyInput = useRef<HTMLInputElement>(null)
    const engine = useRef<TrusteeEngine | null>(null)
    const privateKeys = useRef("")
    const active = useRef(false)
    const occupied = useRef(false)
    const mounted = useRef(true)
    const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const catalog = useRef<string[]>([])
    const catalogKey = useRef("")
    const releaseLock = useRef<(() => void) | null>(null)
    const release = () => {
        releaseLock.current?.()
        releaseLock.current = null
    }
    useEffect(() => {
        mounted.current = true
        return () => {
            mounted.current = false
            active.current = false
            clearTimeout(timer.current)
            if (!occupied.current) {
                engine.current?.free()
                engine.current = null
                release()
            }
        }
    }, [])
    const download = (name: string, contents: string, type: string) => {
        const url = URL.createObjectURL(new Blob([contents], {type}))
        const anchor = document.createElement("a")
        anchor.href = url
        anchor.download = trusteeFilename(trustee, name)
        anchor.click()
        setTimeout(() => URL.revokeObjectURL(url), 0)
    }
    const perform = async (action: () => Promise<void>) => {
        if (occupied.current) return
        occupied.current = true
        setBusy(true)
        setError("")
        try {
            await action()
        } catch (error: unknown) {
            if (mounted.current) setError(error instanceof Error ? error.message : String(error))
        } finally {
            occupied.current = false
            if (mounted.current) setBusy(false)
            else {
                engine.current?.free()
                engine.current = null
            }
        }
    }
    const readHistory = async (
        current: TrusteeEngine | null,
        assigned: TrusteeBoard[] = boards,
        known: string[] = catalog.current
    ) => {
        if (!current) return
        try {
            const names = Array.from(
                new Set([
                    ...known,
                    ...assigned.flatMap((board) =>
                        board.parent ? [board.name, board.parent] : [board.name]
                    ),
                ])
            )
            const result: TrusteeHistory = JSON.parse(
                await current.sessionHistory(JSON.stringify(names))
            )
            if (mounted.current) {
                setHistory(result)
                setHistoryError("")
            }
        } catch (error: unknown) {
            if (mounted.current) {
                setHistory(null)
                setHistoryError(error instanceof Error ? error.message : String(error))
            }
        }
    }
    const install = async (contents: string, backup?: string) => {
        const module = await loadBrowserTrustee()
        if (!mounted.current) return
        const candidate = createBrowserTrustee(
            module,
            auth.trustee || "browser trustee",
            contents,
            boardUrl,
            accessToken
        )
        try {
            const publicKeys: TrusteeKeys = JSON.parse(candidate.publicKeys())
            const storageKey = `sequent-wbraid-catalog:${boardUrl.replace(/\/+$/, "")}:${publicKeys.signing_public_key}`
            const saved: unknown = JSON.parse(localStorage.getItem(storageKey) || "[]")
            if (!Array.isArray(saved) || !saved.every((name: unknown) => typeof name === "string"))
                throw new Error("Invalid local session catalogue.")
            const restored: string[] = backup
                ? JSON.parse(await candidate.restoreBackup(backup))
                : []
            const nextCatalog = Array.from(new Set([...(saved as string[]), ...restored]))
            localStorage.setItem(storageKey, JSON.stringify(nextCatalog))
            engine.current?.free()
            engine.current = candidate
            privateKeys.current = contents
            catalog.current = nextCatalog
            catalogKey.current = storageKey
            setHistory(null)
            setHistoryError("")
            setKeys(publicKeys)
            await readHistory(candidate, boards, nextCatalog)
            setBrowserStatus("Keys ready. Register both public keys before starting a ceremony.")
        } catch (error: unknown) {
            candidate.free()
            throw error
        }
    }
    const importKeys = (file: File | undefined) => {
        if (file)
            void perform(async () => {
                await install(await file.text())
            })
    }
    const importBackup = (file: File | undefined) => {
        if (file)
            void perform(async () => {
                const contents = await file.text()
                const backup: unknown = JSON.parse(contents)
                if (
                    typeof backup !== "object" ||
                    backup === null ||
                    !("keys" in backup) ||
                    typeof backup.keys !== "string"
                )
                    throw new Error("Invalid browser trustee backup.")
                await install(backup.keys, contents)
            })
    }
    const cycle = async () => {
        if (!active.current || !engine.current || occupied.current) return
        occupied.current = true
        setBusy(true)
        try {
            const result = await runBrowserCycle(
                engine.current,
                platform,
                accessToken,
                (assigned) => {
                    catalog.current = Array.from(
                        new Set([
                            ...catalog.current,
                            ...assigned.flatMap((board) =>
                                board.parent ? [board.name, board.parent] : [board.name]
                            ),
                        ])
                    )
                    localStorage.setItem(catalogKey.current, JSON.stringify(catalog.current))
                }
            )
            if (mounted.current) {
                setBoards(result)
                setLoaded(true)
                if (active.current)
                    setBrowserStatus(
                        `Browser trustee running · ${result.length} assigned boards. Keep this page open.`
                    )
            }
        } catch (error: unknown) {
            active.current = false
            if (mounted.current) {
                setRunning(false)
                setBrowserStatus(
                    "Browser trustee stopped after an error. Resolve it before restarting."
                )
                setError(error instanceof Error ? error.message : String(error))
            }
        } finally {
            await readHistory(engine.current)
            occupied.current = false
            if (!active.current) release()
            if (mounted.current) setBusy(false)
            else {
                engine.current?.free()
                engine.current = null
            }
        }
        if (active.current)
            timer.current = setTimeout(() => {
                void cycle()
            }, 2000)
    }
    const stop = () => {
        active.current = false
        clearTimeout(timer.current)
        setRunning(false)
        if (!occupied.current) release()
        setBrowserStatus("Stopped. Local session commitments are retained for resuming.")
    }
    const start = async () => {
        if (!keys || !engine.current || active.current || occupied.current) return
        active.current = true
        setRunning(true)
        setError("")
        try {
            await runExclusiveBrowserTrustee(catalogKey.current, async () => {
                if (!mounted.current || !active.current) return
                const stopped = new Promise<void>((resolve) => {
                    releaseLock.current = resolve
                })
                void cycle()
                await stopped
            })
        } catch (error: unknown) {
            active.current = false
            if (mounted.current) {
                setRunning(false)
                setError(error instanceof Error ? error.message : String(error))
            }
        }
    }
    const prove = (file: File | undefined) => {
        if (file)
            void perform(async () => {
                if (!engine.current) throw new Error("Import your trustee keys first.")
                const request: Record<string, unknown> = JSON.parse(await file.text())
                if (!auth.trustee || request.trustee_name !== auth.trustee)
                    throw new Error("The custody file belongs to another trustee.")
                if (tallyId.trim()) request.tally_session_id = tallyId.trim()
                const proof = engine.current.attest(JSON.stringify(request))
                download("signed-custody-proof.json", proof, "application/json")
                setBrowserStatus(
                    "Signed custody proof ready to upload to the key or tally ceremony."
                )
            })
    }
    return (
        <Box sx={{p: {xs: 2, sm: 3}}}>
            <Typography variant="h4" component="h1">
                Trustee dashboard
            </Typography>
            <Typography sx={{my: 2}}>
                Use the browser trustee below or run the native trustee tool with your local keys
                file. It discovers your assigned boards, verifies their messages and resumes from
                its local session stores.
            </Typography>
            <Card sx={{mb: 2}}>
                <CardContent>
                    <Typography variant="h6" component="h2">
                        Browser trustee
                    </Typography>
                    <Typography sx={{my: 2}}>
                        Generate or import your local trustee keys. Your private keys stay in this
                        browser's memory. Back up the keys file and browser sessions, and keep this
                        page open while participating.
                    </Typography>
                    <Typography sx={{my: 2}}>
                        For manual custody, start this browser trustee and keep this tab open.
                        Upload the saved custody file in the key ceremony or tally tab; this browser
                        creates the scoped proof automatically. Complete the custody confirmation
                        and any required signing in that tab.
                    </Typography>
                    <TextField
                        label="Board service URL"
                        value={boardUrl}
                        disabled={Boolean(keys) || busy}
                        onChange={(event) => setBoardUrl(event.target.value)}
                        fullWidth
                        sx={{mb: 2}}
                    />
                    <Box sx={actionGrid}>
                        <Button
                            variant="outlined"
                            disabled={Boolean(keys) || busy}
                            onClick={() => {
                                void perform(async () => {
                                    const module = await loadBrowserTrustee()
                                    await install(module.BrowserTrustee.generateKeys())
                                })
                            }}
                        >
                            Generate browser keys
                        </Button>
                        <Button
                            variant="outlined"
                            disabled={busy || running}
                            onClick={() => keysInput.current?.click()}
                        >
                            Import private keys
                        </Button>
                        <input
                            ref={keysInput}
                            aria-label="Import private keys"
                            type="file"
                            hidden
                            accept=".toml"
                            onChange={(event) => {
                                importKeys(event.target.files?.[0])
                                event.target.value = ""
                            }}
                        />
                        <Button
                            variant="outlined"
                            disabled={busy || running}
                            onClick={() => backupInput.current?.click()}
                        >
                            Restore browser backup
                        </Button>
                        <input
                            ref={backupInput}
                            aria-label="Restore browser backup"
                            type="file"
                            hidden
                            accept=".json"
                            onChange={(event) => {
                                importBackup(event.target.files?.[0])
                                event.target.value = ""
                            }}
                        />
                        <Button
                            variant="outlined"
                            disabled={!keys || busy}
                            onClick={() =>
                                download("private-keys.toml", privateKeys.current, "text/plain")
                            }
                        >
                            Download private keys
                        </Button>
                        <Button
                            variant="outlined"
                            disabled={!keys || busy || running}
                            onClick={() => {
                                void perform(async () => {
                                    if (engine.current)
                                        download(
                                            "browser-sessions.json",
                                            await engine.current.backup(
                                                JSON.stringify(catalog.current)
                                            ),
                                            "application/json"
                                        )
                                })
                            }}
                        >
                            Back up browser sessions
                        </Button>
                        <Button
                            variant="outlined"
                            disabled={!keys || busy || running}
                            onClick={() => {
                                engine.current?.free()
                                engine.current = null
                                privateKeys.current = ""
                                setKeys(null)
                                setHistory(null)
                                setHistoryError("")
                                setBrowserStatus("")
                            }}
                        >
                            Lock keys
                        </Button>
                    </Box>
                    {keys && (
                        <Box sx={{my: 2}}>
                            <TextField
                                label="Signing public key"
                                value={keys.signing_public_key}
                                InputProps={{readOnly: true}}
                                fullWidth
                                sx={{mb: 2}}
                            />
                            <TextField
                                label="Share encryption public key"
                                value={keys.share_encryption_public_key}
                                InputProps={{readOnly: true}}
                                fullWidth
                            />
                        </Box>
                    )}
                    <Box sx={{...actionGrid, mt: 2}}>
                        <Button
                            variant="contained"
                            disabled={!keys || busy || running || !auth.trustee}
                            onClick={() => {
                                void start()
                            }}
                        >
                            Start browser trustee
                        </Button>
                        <Button variant="outlined" disabled={!running} onClick={stop}>
                            Stop browser trustee
                        </Button>
                    </Box>
                    <Typography role="status" sx={{my: 2}}>
                        {browserStatus}
                    </Typography>
                    <Box component="section" aria-labelledby="trustee-history-heading" sx={{my: 2}}>
                        <Typography id="trustee-history-heading" variant="h6" component="h3">
                            Local session history
                        </Typography>
                        <Typography sx={{my: 1}}>
                            History is read from this browser using the loaded signing key and board
                            service URL. These are the records the trustee reads when resuming a
                            session. Counts confirm readable history, not a completed ceremony.
                        </Typography>
                        <Button
                            variant="outlined"
                            disabled={!keys || busy || running}
                            onClick={() => {
                                void perform(async () => readHistory(engine.current))
                            }}
                            sx={{minHeight: 44, my: 1}}
                        >
                            Refresh local history
                        </Button>
                        {!keys && (
                            <Typography>
                                Import your private keys or restore a browser backup to inspect
                                history.
                            </Typography>
                        )}
                        {historyError && (
                            <Alert severity="error">
                                Unable to read local history: {historyError}
                            </Alert>
                        )}
                        {history && (
                            <>
                                <Typography sx={{overflowWrap: "anywhere", my: 1}}>
                                    Browser: {window.location.origin} · Board service:{" "}
                                    {history.board_url}
                                </Typography>
                                {history.stores.length === 0 && (
                                    <Typography>
                                        No known sessions. Refresh boards to include assigned
                                        sessions.
                                    </Typography>
                                )}
                                {history.stores.map((store) => (
                                    <Box
                                        key={store.board}
                                        sx={{
                                            border: "1px solid",
                                            borderColor: "divider",
                                            borderRadius: 1,
                                            p: 2,
                                            my: 1,
                                            overflowWrap: "anywhere",
                                        }}
                                    >
                                        <Typography>{store.board}</Typography>
                                        <Typography>
                                            {store.predicates === 0
                                                ? "Empty history"
                                                : "History available"}{" "}
                                            · {store.predicates} records · {store.own_posts} own
                                            posts
                                        </Typography>
                                        {store.predicates === 0 &&
                                            boards.some(
                                                (board) => board.parent === store.board
                                            ) && (
                                                <Alert severity="warning" sx={{my: 1}}>
                                                    Parent DKG history is empty. Restore the session
                                                    backup for this identity before tallying.
                                                    Imported elections may use history under the
                                                    original DKG board.
                                                </Alert>
                                            )}
                                        <Typography variant="body2">{store.database}</Typography>
                                    </Box>
                                ))}
                            </>
                        )}
                    </Box>
                    <Accordion
                        disableGutters
                        sx={{mt: 2}}
                        slotProps={{transition: {unmountOnExit: true}}}
                    >
                        <AccordionSummary
                            id="browser-trustee-advanced-heading"
                            aria-controls="browser-trustee-advanced-content"
                            expandIcon={<ExpandMoreIcon />}
                        >
                            <Typography component="h3">Advanced</Typography>
                        </AccordionSummary>
                        <AccordionDetails>
                            <Typography sx={{mb: 2}}>
                                A running browser trustee generates signed custody proofs
                                automatically when you upload the saved custody file in the ceremony
                                or tally wizard. Use this option only when you need to create and
                                upload a proof file yourself. Stop the browser trustee before
                                generating a file.
                            </Typography>
                            <TextField
                                label="Tally session ID"
                                helperText="For a tally proof, copy the tally ID from the tally wizard. Leave blank for key-ceremony custody."
                                value={tallyId}
                                disabled={busy || running}
                                onChange={(event) => setTallyId(event.target.value)}
                                fullWidth
                                sx={{mb: 2}}
                            />
                            <Box sx={actionGrid}>
                                <Button
                                    variant="outlined"
                                    disabled={!keys || busy || running}
                                    onClick={() => custodyInput.current?.click()}
                                >
                                    Generate signed custody proof manually
                                </Button>
                            </Box>
                            <input
                                ref={custodyInput}
                                aria-label="Custody file"
                                type="file"
                                hidden
                                accept=".json"
                                onChange={(event) => {
                                    prove(event.target.files?.[0])
                                    event.target.value = ""
                                }}
                            />
                        </AccordionDetails>
                    </Accordion>
                </CardContent>
            </Card>
            <Card>
                <CardContent>
                    <Typography variant="h6" component="h2">
                        Local trustee tool
                    </Typography>
                    <Box component="pre" sx={codeExample}>
                        <code>{`trustee generate --trustee-config ${keysFilename}
trustee start --trustee-config ${keysFilename}`}</code>
                    </Box>
                    <Typography>
                        Register both public keys printed by generate. Keep a private backup of{" "}
                        {keysFilename} and the message_store directory.
                    </Typography>
                    <Typography sx={{mt: 2}}>
                        For a manual ceremony, download its custody file and create a signed custody
                        proof to upload to the portal:
                    </Typography>
                    <Box component="pre" sx={codeExample}>
                        <code>{`trustee attest --request ${requestFilename} --trustee-config ${keysFilename} --output ${proofFilename}`}</code>
                    </Box>
                    <Typography>
                        For a tally restore, add --tally-session-id with the tally session's ID. The
                        proof contains a signature and public metadata.
                    </Typography>
                </CardContent>
            </Card>
            <Typography variant="h6" component="h2" sx={{mt: 3}}>
                Boards assigned to {auth.trustee || "your trustee"}
            </Typography>
            <Box
                sx={{
                    display: "grid",
                    gridTemplateColumns: {xs: "1fr", sm: "minmax(0, 1fr) auto"},
                    alignItems: "center",
                    gap: 2,
                    my: 2,
                }}
            >
                <TextField
                    label="Platform URL"
                    value={platform}
                    InputProps={{readOnly: true}}
                    fullWidth
                />
                <Button
                    variant="outlined"
                    onClick={refresh}
                    disabled={loading}
                    sx={{minHeight: 44}}
                >
                    {loading ? "Loading boards…" : "Refresh boards"}
                </Button>
            </Box>
            {error && <Alert severity="error">{error}</Alert>}
            {loaded && boards.length === 0 && (
                <Typography>No boards are currently assigned.</Typography>
            )}
            {boards.map((board) => (
                <Card key={board.name} sx={{my: 1}}>
                    <CardContent>
                        <Typography sx={{overflowWrap: "anywhere"}}>{board.name}</Typography>
                        <Typography>
                            {board.kind}
                            {board.parent ? ` · Parent: ${board.parent}` : ""}
                        </Typography>
                    </CardContent>
                </Card>
            ))}
        </Box>
    )
}
