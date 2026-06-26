// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useRef, useState} from "react"
import {useRecordContext, useGetList} from "react-admin"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useEmulatorStore} from "@/providers/EmulatorContextProvider"
import {EEmulatorSessionStatus} from "@/types/emulator"
import {useTranslation} from "react-i18next"
import {
    Box,
    Button,
    Chip,
    FormControl,
    InputLabel,
    ListSubheader,
    MenuItem,
    Select,
    TextField,
    Typography,
    Alert,
    Paper,
    SelectChangeEvent,
} from "@mui/material"

interface BallotStyleRecord {
    id: string
    status?: string | null
    election_id?: string
    area_id?: string | null
    labels?: unknown
}

const PUBLISHED_STATUS = "PUBLISHED"

export const EditElectionEventEmulator: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const {status, outputLines, errorMessage, startSession, sendInput, resetSession} =
        useEmulatorStore()

    const [selectedBallotStyleId, setSelectedBallotStyleId] = useState<string>("")
    const [inputValue, setInputValue] = useState("")
    const outputEndRef = useRef<HTMLDivElement | null>(null)

    const {data: ballotStyles} = useGetList<BallotStyleRecord>(
        "sequent_backend_ballot_style",
        {
            filter: {election_event_id: record?.id},
        }
    )

    const {published, unpublished} = useMemo(() => {
        const pub: BallotStyleRecord[] = []
        const unpub: BallotStyleRecord[] = []
        ballotStyles?.forEach((bs) => {
            if (bs.status === PUBLISHED_STATUS) {
                pub.push(bs)
            } else {
                unpub.push(bs)
            }
        })
        return {published: pub, unpublished: unpub}
    }, [ballotStyles])

    // Auto-scroll output to bottom
    useEffect(() => {
        outputEndRef.current?.scrollIntoView({behavior: "smooth"})
    }, [outputLines])

    const handleStart = () => {
        if (!record || !selectedBallotStyleId) {
            return
        }
        const ballotStyle = ballotStyles?.find((bs) => bs.id === selectedBallotStyleId)
        if (!ballotStyle) {
            return
        }
        startSession(record, ballotStyle)
    }

    const handleSendInput = () => {
        if (!inputValue.trim()) {
            return
        }
        sendInput(inputValue.trim())
        setInputValue("")
    }

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "Enter") {
            e.preventDefault()
            handleSendInput()
        }
    }

    const handleBallotStyleChange = (e: SelectChangeEvent) => {
        setSelectedBallotStyleId(e.target.value)
    }

    const renderBallotStyleItem = (bs: BallotStyleRecord) => (
        <MenuItem key={bs.id} value={bs.id}>
            <Box sx={{display: "flex", alignItems: "center", gap: 1, width: "100%"}}>
                <Typography variant="body2" sx={{fontFamily: "monospace", flexShrink: 0}}>
                    {bs.id.slice(0, 8)}
                </Typography>
                <Chip
                    label={bs.status ?? "—"}
                    size="small"
                    color={bs.status === PUBLISHED_STATUS ? "success" : "default"}
                    variant="outlined"
                    sx={{fontSize: "0.7rem"}}
                />
            </Box>
        </MenuItem>
    )

    const isSessionActive =
        status === EEmulatorSessionStatus.AWAITING_INPUT ||
        status === EEmulatorSessionStatus.PROCESSING

    const isIdle = status === EEmulatorSessionStatus.IDLE
    const isDisconnected = status === EEmulatorSessionStatus.DISCONNECTED
    const isError = status === EEmulatorSessionStatus.ERROR
    const isInitializing = status === EEmulatorSessionStatus.INITIALIZING
    const canSendInput = status === EEmulatorSessionStatus.AWAITING_INPUT

    return (
        <Box sx={{p: 2, display: "flex", flexDirection: "column", gap: 2}}>
            <Typography variant="h6">
                {t("electionEventScreen.emulator.title")}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {t("electionEventScreen.emulator.description")}
            </Typography>

            {/* Session setup */}
            {(isIdle || isDisconnected || isError) && (
                <Box sx={{display: "flex", gap: 2, alignItems: "flex-end", flexWrap: "wrap"}}>
                    <FormControl sx={{minWidth: 360}}>
                        <InputLabel id="ballot-style-select-label">
                            {t("electionEventScreen.emulator.ballotStyleLabel")}
                        </InputLabel>
                        <Select
                            labelId="ballot-style-select-label"
                            value={selectedBallotStyleId}
                            onChange={handleBallotStyleChange}
                            label={t("electionEventScreen.emulator.ballotStyleLabel")}
                            size="small"
                        >
                            {published.length > 0 && (
                                <ListSubheader>
                                    {t("electionEventScreen.emulator.publishedGroup")}
                                </ListSubheader>
                            )}
                            {published.map(renderBallotStyleItem)}
                            {unpublished.length > 0 && (
                                <ListSubheader>
                                    {t("electionEventScreen.emulator.unpublishedGroup")}
                                </ListSubheader>
                            )}
                            {unpublished.map(renderBallotStyleItem)}
                        </Select>
                    </FormControl>
                    <Button
                        variant="contained"
                        onClick={handleStart}
                        disabled={!selectedBallotStyleId || isInitializing}
                    >
                        {isDisconnected || isError
                            ? t("electionEventScreen.emulator.restart")
                            : t("electionEventScreen.emulator.start")}
                    </Button>
                    {(isDisconnected || isError) && (
                        <Button variant="outlined" onClick={resetSession}>
                            {t("electionEventScreen.emulator.reset")}
                        </Button>
                    )}
                </Box>
            )}

            {/* Error banner */}
            {isError && errorMessage && (
                <Alert severity="error">{errorMessage}</Alert>
            )}

            {/* Disconnected banner */}
            {isDisconnected && (
                <Alert severity="info">
                    {t("electionEventScreen.emulator.sessionEnded")}
                </Alert>
            )}

            {/* Terminal output */}
            {outputLines.length > 0 && (
                <Paper
                    variant="outlined"
                    sx={{
                        bgcolor: "#1e1e1e",
                        color: "#d4d4d4",
                        fontFamily: "monospace",
                        fontSize: "0.875rem",
                        p: 2,
                        maxHeight: 400,
                        overflowY: "auto",
                        whiteSpace: "pre-wrap",
                        wordBreak: "break-word",
                    }}
                >
                    {outputLines.map((line) => (
                        <Box
                            key={line.timestamp}
                            component="div"
                            sx={{
                                color: line.text.startsWith("> ") ? "#569cd6" : "#d4d4d4",
                                lineHeight: 1.6,
                            }}
                        >
                            {line.text}
                        </Box>
                    ))}
                    <div ref={outputEndRef} />
                </Paper>
            )}

            {/* Input field */}
            {isSessionActive && (
                <Box sx={{display: "flex", gap: 1}}>
                    <TextField
                        fullWidth
                        size="small"
                        value={inputValue}
                        onChange={(e) => setInputValue(e.target.value)}
                        onKeyDown={handleKeyDown}
                        disabled={!canSendInput}
                        placeholder={t("electionEventScreen.emulator.inputPlaceholder")}
                        autoFocus
                        sx={{fontFamily: "monospace"}}
                    />
                    <Button
                        variant="contained"
                        onClick={handleSendInput}
                        disabled={!canSendInput || !inputValue.trim()}
                    >
                        {t("electionEventScreen.emulator.send")}
                    </Button>
                </Box>
            )}
        </Box>
    )
}
