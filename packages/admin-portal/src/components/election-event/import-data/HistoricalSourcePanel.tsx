// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useRef, useState} from "react"
import {Box, Button, Typography} from "@mui/material"
import {ProblemList, type Problem} from "@sequentech/ui-essentials"
import type {BuildOutput} from "sequent-election-config"
import {HistoricalImportRuntimeError} from "@/services/HistoricalImportWorkerClient"
import type {HistoricalImportConverterApi} from "@/services/HistoricalImportConverter"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import defaults from "@/services/historical-profile-defaults.json"
import {useTranslation} from "react-i18next"

interface Summary {
    profile: string
    sqlSha256: string
    workbookSha256: string
    sourceRows: Record<string, number>
    generatedRows: Record<string, number>
    selectedCandidates: number
    outsideScopeCandidates: number
    excludedVoters: number
    opaqueBinaryValues: number
    excludedPollingCenters: number
    excludedPrecinctRows: number
    electionContexts: Array<{workbookRow: number; centerRows: number[]}>
    seats: Array<{sourceRow: number; candidateRows: number; eligibleAmount: string}>
    precincts: Array<{sourceRow: number; centerRow: number; workbookAreaRows: number[]}>
    areaWorkbookRows: number[]
    decisionsSha256: string
    unresolvedAffiliations: number
}
interface HistoricalOutput {
    summary: Summary
    output: BuildOutput
}
interface Props {
    api: HistoricalImportConverterApi
    disabled: boolean
    droppedFiles?: File[]
    onRetry?: () => void
    onPrepare: () => void
    onAccepted: (file: File, hash: string) => Promise<void>
}

export const HistoricalSourcePanel: React.FC<Props> = (props) => {
    const {api, disabled, droppedFiles} = props
    const callbacks = useRef(props)
    callbacks.current = props
    const {loaded, globalSettings} = useContext(SettingsContext)
    const appVersion = globalSettings.APP_VERSION
    const {t} = useTranslation()
    const label = (key: string, fallback: string) =>
        String(t(`electionEventScreen.import.historical.${key}`, {defaultValue: fallback}))
    const [attempt, setAttempt] = useState(0)
    const [working, setWorking] = useState(false)
    const [result, setResult] = useState<HistoricalOutput>()
    const [error, setError] = useState("")
    const sql = droppedFiles?.find((file) => /\.sql$/i.test(file.name))
    const workbook = droppedFiles?.find((file) => /\.xlsx$/i.test(file.name))

    useEffect(() => {
        let active = true
        setResult(undefined)
        setError("")
        setWorking(false)
        callbacks.current.onPrepare()
        if (!sql || !workbook || !loaded) return
        // The converter's standalone default may target a different platform major.
        // Runtime JSON can be incomplete even though GlobalSettings is typed.
        if (
            typeof appVersion !== "string" ||
            (appVersion !== "dev" &&
                !/^[vV]?\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(appVersion))
        ) {
            setError(
                "The application version is unavailable or invalid. Reload the page to reload application settings, or ask your administrator to check APP_VERSION. The selected files have not been uploaded."
            )
            return
        }
        setWorking(true)
        const convert = async () => {
            try {
                if (sql.size > 128 * 1024 * 1024 || workbook.size > 32 * 1024 * 1024) {
                    setError("SQL must be at most 128 MiB and the workbook at most 32 MiB.")
                    return
                }
                let sqlBytes: ArrayBuffer, workbookBytes: ArrayBuffer
                try {
                    ;[sqlBytes, workbookBytes] = await Promise.all([
                        sql.arrayBuffer(),
                        workbook.arrayBuffer(),
                    ])
                } catch {
                    if (active)
                        setError(
                            "Could not read one of the local source files. Check file access and drop it again. No file was uploaded."
                        )
                    return
                }
                if (!active) return
                const prepared = (await api.buildFromHistorical(
                    new Uint8Array(sqlBytes),
                    new Uint8Array(workbookBytes),
                    defaults,
                    {baseExport: {version: appVersion}}
                )) as HistoricalOutput
                if (!active) return
                setResult(prepared)
                if (
                    !prepared.output.archive ||
                    prepared.output.report.problems.some((p) => p.severity === "error")
                )
                    return
                const bytes = new Uint8Array(prepared.output.archive.bytes)
                const digest = await crypto.subtle.digest("SHA-256", bytes)
                if (!active) return
                const hash = Array.from(new Uint8Array(digest), (b) =>
                    b.toString(16).padStart(2, "0")
                ).join("")
                await callbacks.current.onAccepted(
                    new File([bytes], prepared.output.archive.name, {type: "application/zip"}),
                    hash
                )
            } catch (error) {
                if (active)
                    setError(
                        error instanceof HistoricalImportRuntimeError
                            ? error.message
                            : "Converter runtime failed. Check the configured plugin and source format, then retry. The selected files are retained; no source file was uploaded."
                    )
            } finally {
                if (active) setWorking(false)
            }
        }
        void convert()
        return () => {
            active = false
        }
    }, [api, droppedFiles, attempt, loaded, appVersion])

    const problems = result?.output.report.problems.map((problem) => ({
        ...problem,
        // The parser returns stable codes with safe source coordinates. Keep
        // those coordinates visible and add guidance instead of showing a code alone.
        id: undefined,
        message: `${problem.path ? problem.path + ": " : ""}${
            problem.message === "sql.unsupported-statement"
                ? "Unsupported SQL statement. Supply a data-only MySQL dump for the supported historical profile; executable statements are not accepted."
                : /^sql\./.test(problem.message)
                  ? "The SQL dump could not be read safely. Check the syntax at this location and export a supported data-only MySQL dump."
                  : problem.message
        }`,
    })) as Problem[] | undefined
    return (
        <Box sx={{mt: 2, minWidth: 0, display: "grid", gap: 1.5}}>
            {!sql || !workbook ? (
                <Typography role="status">
                    {!sql
                        ? "Drop the SQL dump into the same zone to complete the source pair."
                        : "Drop the supplemental XLSX workbook into the same zone to complete the source pair."}
                </Typography>
            ) : null}
            {sql && workbook && !loaded && (
                <Typography role="status">
                    Waiting for application settings before validating the source files.
                </Typography>
            )}
            {working && (
                <Typography role="status">
                    Validating SQL and workbook locally… No files have been uploaded.
                </Typography>
            )}
            {error && (
                <>
                    <Typography role="alert" color="error">
                        {error}
                    </Typography>
                    <Button
                        disabled={disabled || working}
                        onClick={() =>
                            props.onRetry ? props.onRetry() : setAttempt((value) => value + 1)
                        }
                    >
                        Retry local validation
                    </Button>
                </>
            )}
            {result && (
                <>
                    <Typography variant="body2">
                        Historical profile: configuration only, seat plurality, complete coverage.
                        No voter eligibility or timezone is inferred.
                    </Typography>
                    <Box component="details">
                        <summary>Source details and exclusions</summary>
                        <Typography variant="body2" sx={{overflowWrap: "anywhere"}}>
                            SQL SHA-256: {result.summary.sqlSha256}
                            <br />
                            XLSX SHA-256: {result.summary.workbookSha256}
                            <br />
                            {label("decisionsDigest", "Decisions SHA-256")}:{" "}
                            {result.summary.decisionsSha256}
                        </Typography>
                        <Typography variant="body2">
                            {label("exclusions", "Explicit exclusions")}:{" "}
                            {result.summary.excludedVoters} {label("voters", "voters")};{" "}
                            {result.summary.outsideScopeCandidates}{" "}
                            {label("outside", "candidates outside configured seats")}.{" "}
                            {result.summary.excludedPollingCenters ?? 0}{" "}
                            {label("excludedCenters", "excluded polling centers")};{" "}
                            {result.summary.excludedPrecinctRows ?? 0}{" "}
                            {label("excludedPrecincts", "excluded precinct rows")}.{" "}
                            {result.summary.unresolvedAffiliations}{" "}
                            {label("unresolvedAffiliations", "unresolved affiliation references")}.{" "}
                            {label("binary", "Opaque binary values retained in the original")}:{" "}
                            {result.summary.opaqueBinaryValues}.
                        </Typography>
                        <Box component="details">
                            <summary>{label("counts", "Source and generated row counts")}</summary>
                            <Box component="pre" sx={{whiteSpace: "pre-wrap", fontSize: "0.75rem"}}>
                                {JSON.stringify(
                                    {
                                        source: result.summary.sourceRows,
                                        generated: result.summary.generatedRows,
                                    },
                                    null,
                                    2
                                )}
                            </Box>
                        </Box>
                    </Box>
                    {problems && (
                        <>
                            <Typography variant="body2">
                                {label(
                                    "localDiagnostics",
                                    "Local conversion/validation diagnostics"
                                )}
                            </Typography>
                            <ProblemList report={{problems}} />
                        </>
                    )}
                </>
            )}
        </Box>
    )
}
