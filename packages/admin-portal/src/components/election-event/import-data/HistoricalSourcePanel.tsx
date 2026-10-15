// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState, useEffect, useRef} from "react"
import {
    Box,
    Button,
    MenuItem,
    TextField,
    Typography,
    Checkbox,
    FormControlLabel,
} from "@mui/material"
import {ProblemList, type Problem} from "@sequentech/ui-essentials"
import type {BuildOutput} from "sequent-election-config"
import {HistoricalImportRuntimeError} from "@/services/HistoricalImportWorkerClient"
import type {HistoricalImportConverterApi} from "@/services/HistoricalImportConverter"
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
    onPrepare: () => void
    onAccepted: (file: File, hash: string) => Promise<void>
}

export const HistoricalSourcePanel: React.FC<Props> = ({
    api,
    disabled,
    onPrepare,
    onAccepted,
    droppedFiles,
}) => {
    const {t} = useTranslation()
    const label = (key: string, fallback: string) =>
        String(t(`electionEventScreen.import.historical.${key}`, {defaultValue: fallback}))
    const [sql, setSql] = useState<File>()
    const [workbook, setWorkbook] = useState<File>()
    const [census, setCensus] = useState("")
    const [ballot, setBallot] = useState("")
    const [scope, setScope] = useState("")
    const [offset, setOffset] = useState("")
    const [working, setWorking] = useState(false)
    const [result, setResult] = useState<HistoricalOutput>()
    const [error, setError] = useState("")
    const [inspection, setInspection] = useState<Summary>()
    const [affiliation, setAffiliation] = useState("require-known-party")
    const [precinctCoverage, setPrecinctCoverage] = useState("require-complete-coverage")
    const [postCoverage, setPostCoverage] = useState("require-complete-coverage")
    const [seatMap, setSeatMap] = useState<Record<string, number[]>>({})
    const [areaMap, setAreaMap] = useState<Record<string, number>>({})
    const changeSource = () => {
        reset()
        setInspection(undefined)
        setSeatMap({})
        setAreaMap({})
        setAffiliation("require-known-party")
        setPostCoverage("require-complete-coverage")
        setPrecinctCoverage("require-complete-coverage")
    }
    const reset = () => {
        setResult(undefined)
        setError("")
        onPrepare()
    }
    const lastDrop = useRef<File[] | undefined>(undefined)
    useEffect(() => {
        if (!droppedFiles?.length || lastDrop.current === droppedFiles) return
        lastDrop.current = droppedFiles
        setResult(undefined)
        setError("")
        setInspection(undefined)
        setSeatMap({})
        setAreaMap({})
        setAffiliation("require-known-party")
        setPostCoverage("require-complete-coverage")
        setPrecinctCoverage("require-complete-coverage")
        onPrepare()
        const dump = droppedFiles.find((file) => /\.sql$/i.test(file.name))
        const supplement = droppedFiles.find((file) => /\.xlsx$/i.test(file.name))
        if (dump) setSql(dump)
        if (supplement) setWorkbook(supplement)
    }, [droppedFiles, onPrepare])
    const convert = async () => {
        if (!sql || !workbook || !census || !ballot || !scope) return
        setWorking(true)
        reset()
        try {
            const minutes = offset.trim() === "" ? null : Number(offset)
            if (minutes !== null && (!Number.isInteger(minutes) || Math.abs(minutes) > 840)) {
                setError(
                    label(
                        "offsetError",
                        "UTC offset must be an integer between -840 and 840 minutes."
                    )
                )
                return
            }
            if (sql.size > 128 * 1024 * 1024 || workbook.size > 32 * 1024 * 1024) {
                setError(
                    label(
                        "sizeError",
                        "SQL must be at most 128 MiB and the workbook at most 32 MiB."
                    )
                )
                return
            }
            let sqlBytes: ArrayBuffer, workbookBytes: ArrayBuffer
            try {
                ;[sqlBytes, workbookBytes] = await Promise.all([
                    sql.arrayBuffer(),
                    workbook.arrayBuffer(),
                ])
            } catch {
                setError(
                    label(
                        "readError",
                        "Could not read one of the local source files. Check file access and select the file again. No file was uploaded."
                    )
                )
                return
            }
            const electionMappings = Object.entries(seatMap)
                .filter(([, rows]) => rows.length > 0)
                .map(([workbookRow, seatRows]) => ({workbookRow: Number(workbookRow), seatRows}))
            const areaGroups: Record<string, number[]> = {}
            for (const [precinctRow, workbookRow] of Object.entries(areaMap)) {
                if (workbookRow) (areaGroups[workbookRow] ??= []).push(Number(precinctRow))
            }
            const areaMappings = Object.entries(areaGroups).map(([workbookRow, precinctRows]) => ({
                workbookRow: Number(workbookRow),
                precinctRows,
            }))
            const prepared = (await api.buildFromHistorical(
                new Uint8Array(sqlBytes),
                new Uint8Array(workbookBytes),
                {
                    census,
                    ballot,
                    scope,
                    scheduleOffsetMinutes: minutes,
                    postCoverage,
                    precinctCoverage,
                    affiliation,
                    electionMappings,
                    areaMappings,
                    sourceDigests: inspection
                        ? {
                              sqlSha256: inspection.sqlSha256,
                              workbookSha256: inspection.workbookSha256,
                          }
                        : null,
                },
                {}
            )) as HistoricalOutput
            setResult(prepared)
            setInspection(prepared.summary)
        } catch (error) {
            setError(
                error instanceof HistoricalImportRuntimeError
                    ? error.message
                    : label(
                          "conversionError",
                          "Converter runtime failed. Check the configured plugin version and source format, then retry. The selected files are retained; no source file was uploaded."
                      )
            )
        } finally {
            setWorking(false)
        }
    }
    const accept = async () => {
        if (
            !result?.output.archive ||
            result.output.report.problems.some((p) => p.severity === "error")
        )
            return
        setWorking(true)
        try {
            const bytes = new Uint8Array(result.output.archive.bytes)
            const digest = await crypto.subtle.digest("SHA-256", bytes)
            const hash = Array.from(new Uint8Array(digest), (b) =>
                b.toString(16).padStart(2, "0")
            ).join("")
            await onAccepted(
                new File([bytes], result.output.archive.name, {type: "application/zip"}),
                hash
            )
        } catch {
            setError(
                label(
                    "packageError",
                    "Could not prepare the validated package for upload. Keep the selected files and retry; check upload diagnostics in this drawer."
                )
            )
        } finally {
            setWorking(false)
        }
    }
    const locked = disabled || working
    const problems = result?.output.report.problems as Problem[] | undefined
    return (
        <Box
            component="fieldset"
            disabled={locked}
            sx={{mt: 2, minWidth: 0, display: "grid", gap: 1.5}}
        >
            <legend>{label("title", "Historical SQL + workbook (legacy-seat-v1)")}</legend>
            <Typography variant="body2">
                {label(
                    "description",
                    "Conversion runs on this machine. Keep the original files. Review the decisions and diagnostics before uploading the generated native package."
                )}
            </Typography>
            <Typography variant="body2">
                {label(
                    "localSelectionStage",
                    "1. Select both sources locally. No source file is uploaded."
                )}
            </Typography>
            <label>
                {label("sql", "Original SQL dump")}
                <input
                    type="file"
                    accept=".sql"
                    aria-label="Historical SQL"
                    onChange={(e) => {
                        changeSource()
                        setSql(e.target.files?.[0])
                    }}
                />
            </label>
            <label>
                {label("workbook", "Supplemental workbook")}
                <input
                    type="file"
                    accept=".xlsx"
                    aria-label="Historical workbook"
                    onChange={(e) => {
                        changeSource()
                        setWorkbook(e.target.files?.[0])
                    }}
                />
            </label>
            <TextField
                select
                label={label("census", "Census decision")}
                value={census}
                onChange={(e) => {
                    reset()
                    setCensus(e.target.value)
                }}
            >
                <MenuItem value="configuration-only">
                    {label(
                        "configurationOnly",
                        "Configuration only: exclude voters; eligibility requires separate review"
                    )}
                </MenuItem>
            </TextField>
            <TextField
                select
                label={label("ballot", "Reviewed ballot policy")}
                value={ballot}
                onChange={(e) => {
                    reset()
                    setBallot(e.target.value)
                }}
            >
                <MenuItem value="seat-plurality-at-large">
                    {label(
                        "plurality",
                        "Plurality at large: seat count sets maximum choices and winners; minimum 0"
                    )}
                </MenuItem>
            </TextField>
            <TextField
                select
                label={label("scope", "Candidate scope decision")}
                value={scope}
                onChange={(e) => {
                    reset()
                    setScope(e.target.value)
                }}
            >
                <MenuItem value="require-complete-coverage">
                    {label("complete", "Require every candidate to match a configured seat")}
                </MenuItem>
                <MenuItem value="configured-seats-only">
                    {label(
                        "configured",
                        "Use configured seats only; report candidates outside that scope"
                    )}
                </MenuItem>
            </TextField>
            <TextField
                label={label("offset", "UTC offset in minutes for dates without a timezone")}
                value={offset}
                type="number"
                onChange={(e) => {
                    reset()
                    setOffset(e.target.value)
                }}
                helperText={label(
                    "offsetHelp",
                    "Leave blank only when all source dates already include their timezone."
                )}
            />
            {inspection && (
                <>
                    <TextField
                        select
                        label={label("postCoverage", "Post coverage decision")}
                        value={postCoverage}
                        onChange={(e) => {
                            reset()
                            setPostCoverage(e.target.value)
                        }}
                    >
                        <MenuItem value="require-complete-coverage">
                            {label(
                                "allPosts",
                                "Require a workbook context for every SQL polling center"
                            )}
                        </MenuItem>
                        <MenuItem value="workbook-posts-only">
                            {label(
                                "workbookPosts",
                                "Use workbook posts only; report excluded SQL centers and precincts"
                            )}
                        </MenuItem>
                    </TextField>
                    <TextField
                        select
                        label={label("affiliation", "Affiliation reference decision")}
                        value={affiliation}
                        onChange={(e) => {
                            reset()
                            setAffiliation(e.target.value)
                        }}
                    >
                        <MenuItem value="require-known-party">
                            {label(
                                "knownParty",
                                "Require every affiliation code to match a source party"
                            )}
                        </MenuItem>
                        <MenuItem value="preserve-unresolved-code">
                            {label(
                                "preserveParty",
                                "Preserve unresolved codes; do not infer party names"
                            )}
                        </MenuItem>
                    </TextField>
                    <TextField
                        select
                        label={label("precinctCoverage", "Precinct coverage decision")}
                        value={precinctCoverage}
                        onChange={(e) => {
                            reset()
                            setPrecinctCoverage(e.target.value)
                        }}
                    >
                        <MenuItem value="require-complete-coverage">
                            {label(
                                "allPrecincts",
                                "Require an area association for every selected SQL precinct"
                            )}
                        </MenuItem>
                        <MenuItem value="workbook-posts-only">
                            {label(
                                "workbookAreas",
                                "Use workbook areas only; report precinct rows without an exact match"
                            )}
                        </MenuItem>
                    </TextField>
                    {(inspection.electionContexts ?? []).map((context) => (
                        <Box key={context.workbookRow} component="fieldset">
                            <legend>
                                {label("electionRow", "Workbook Elections row")}{" "}
                                {context.workbookRow}
                            </legend>
                            <Typography variant="body2">
                                {label(
                                    "seatReview",
                                    "Select the SQL seats this election includes. Associations are bound to the displayed source hashes."
                                )}
                            </Typography>
                            {(inspection.seats ?? []).map((seat) => (
                                <FormControlLabel
                                    key={seat.sourceRow}
                                    label={`${label("seatRow", "SQL seat row")} ${seat.sourceRow}: ${seat.candidateRows} ${label("candidates", "candidates")}, ${label("seatCount", "seat count")} ${seat.eligibleAmount}`}
                                    control={
                                        <Checkbox
                                            checked={(seatMap[context.workbookRow] ?? []).includes(
                                                seat.sourceRow
                                            )}
                                            onChange={(e) => {
                                                reset()
                                                const current = seatMap[context.workbookRow] ?? []
                                                setSeatMap({
                                                    ...seatMap,
                                                    [context.workbookRow]: e.target.checked
                                                        ? [...current, seat.sourceRow]
                                                        : current.filter(
                                                              (row) => row !== seat.sourceRow
                                                          ),
                                                })
                                            }}
                                        />
                                    }
                                />
                            ))}
                        </Box>
                    ))}
                    {(inspection.precincts ?? [])
                        .filter((precinct) =>
                            (inspection.electionContexts ?? []).some((context) =>
                                context.centerRows.includes(precinct.centerRow)
                            )
                        )
                        .map((precinct) => (
                            <TextField
                                key={precinct.sourceRow}
                                select
                                label={`${label("precinctRow", "SQL precinct row")} ${precinct.sourceRow}: ${label("areaAssociation", "workbook area association")}`}
                                value={areaMap[precinct.sourceRow] ?? 0}
                                onChange={(e) => {
                                    reset()
                                    setAreaMap({
                                        ...areaMap,
                                        [precinct.sourceRow]: Number(e.target.value),
                                    })
                                }}
                            >
                                <MenuItem value={0}>
                                    {label("exactArea", "Keep exact source-name matching")}
                                </MenuItem>
                                {(inspection.areaWorkbookRows ?? []).map((row) => (
                                    <MenuItem key={row} value={row}>
                                        {label("areaRow", "Workbook Areas row")} {row}
                                    </MenuItem>
                                ))}
                            </TextField>
                        ))}
                </>
            )}
            <Typography variant="body2">
                {label(
                    "localConversionStage",
                    "2. Convert and validate locally in this browser. Review all diagnostics before accepting a package."
                )}
            </Typography>
            <Button
                disabled={locked || !sql || !workbook || !census || !ballot || !scope}
                onClick={convert}
            >
                {label("prepare", "Convert and validate locally")}
            </Button>
            {error && (
                <Typography role="alert" color="error">
                    {error}
                </Typography>
            )}
            {result && (
                <>
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
                    <Button
                        disabled={
                            locked ||
                            !result.output.archive ||
                            problems?.some((p) => p.severity === "error")
                        }
                        onClick={accept}
                    >
                        {label("accept", "Accept reviewed package and upload")}
                    </Button>
                </>
            )}
        </Box>
    )
}
