// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * `/tally` — standalone tally sandbox.
 *
 * Sister tool of `/pipeline` (BallotPipeline.tsx). Where the pipeline
 * exercises the encode/encrypt/decrypt/decode chain end-to-end on a
 * single ballot, this page exercises the **tally** step in isolation
 * and renders its result through ui-essentials'
 * `ResultsAndParticipation` so the visualization is 1:1 with what
 * production (results-portal / admin-portal) produces.
 *
 * Three input panes:
 *   1. Setup           — contest descriptor JSON (the `Contest` that
 *                        velvet-wasm deserialises). Drives the counting
 *                        algorithm + candidate axis labels.
 *   2. Input ballots   — JSON array of `DecodedVoteContest` objects,
 *                        one per cast vote. This is the **only** tally
 *                        input shape; ballots in BigUint form decode
 *                        via /pipeline first.
 *   3. Output          — JSON `ContestResult`. Either populated by
 *                        "Run tally" (compute path) or pasted in
 *                        manually (render-only path, for exercising the
 *                        visualization with pre-existing data).
 *
 * Hand-off entry points: the contest page (`WorkbenchInspector`)
 * carries a `TallySeed` in react-router location state ("Open in
 * tally"); the ballot pipeline does the same with its tally textarea
 * ("Send to tally"). Mirrors `PipelineSeed` semantics — one-shot
 * navigation payload, no persistence.
 */

import type {CSSProperties} from "react"
import {useCallback, useMemo, useState} from "react"
import {useLocation} from "react-router-dom"

import {createTheme, ThemeProvider} from "@mui/material/styles"
import CssBaseline from "@mui/material/CssBaseline"

import {
    PreferentialCandidateResults,
    ResultsAndParticipation,
} from "@sequentech/ui-essentials"

import {
    decodeBigIntToDecodedVoteContest,
    encodeBallot,
    runTally,
} from "./tally"
import {
    adaptVelvetContestResult,
    type VelvetTallyView,
} from "./lib/velvetTallyAdapter"
import {
    applyPolicyOverlayToContest,
    usePolicyOverrides,
} from "./policyOverridesStore"

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/** Navigation payload accepted by the `/tally` route. Built by call
 *  sites that have a contest descriptor + decoded ballots already in
 *  hand (contest page; ballot pipeline's tally section). */
export interface TallySeed {
    /** Display name shown in the page header (purely cosmetic). */
    contestName?: string
    /** JSON-serialised `sequent_core::ballot::Contest`. */
    contestJson: string
    /** Array of `DecodedVoteContest` objects (not JSON-stringified).
     *  Will be JSON.stringify-ed for display in the input textarea. */
    decodedBallots: unknown[]
    /** Optional pre-computed `ContestResult` JSON object. When present
     *  the page renders it immediately without running a tally — used
     *  by call sites that already ran the tally upstream and want the
     *  paste-in-output path. */
    result?: unknown
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function TallyPage(): React.ReactElement {
    const location = useLocation()
    const seed = (location.state ?? null) as TallySeed | null

    const [contestJson, setContestJson] = useState<string>(
        seed?.contestJson ?? ""
    )
    const [ballotsJson, setBallotsJson] = useState<string>(
        seed?.decodedBallots
            ? JSON.stringify(seed.decodedBallots, null, 2)
            : ""
    )
    const [outputJson, setOutputJson] = useState<string>(
        seed?.result ? JSON.stringify(seed.result, null, 2) : ""
    )

    // The currently-rendered model (the visualization's source of
    // truth). Separate from `outputJson` so a malformed paste does not
    // wipe out the last good render.
    const [model, setModel] = useState<VelvetTallyView | null>(() => {
        if (seed?.result) {
            return adaptVelvetContestResult(seed.result, seed.contestName)
        }
        return null
    })

    const [busy, setBusy] = useState<boolean>(false)
    const [error, setError] = useState<string | null>(null)

    const contestName = useMemo(
        () => seed?.contestName ?? readContestName(contestJson),
        [seed?.contestName, contestJson]
    )

    // Effective overlay for the contest currently in the textarea.
    // Read live so the badge below the textarea updates as the
    // operator flips switches on the contest detail page.
    const contestId = useMemo(
        () => readContestId(contestJson),
        [contestJson]
    )
    const contestOverlay = usePolicyOverrides((m) =>
        contestId ? m[contestId] : undefined
    )
    const overlayFieldCount = contestOverlay
        ? Object.keys(contestOverlay).length
        : 0

    const handleRunTally = useCallback(async () => {
        setBusy(true)
        setError(null)
        try {
            const ballots = parseDecodedBallots(ballotsJson)
            // Ephemeral policy overlay (see
            // `policyOverridesStore.ts`): tally run is one of the
            // two boundary points where the operator's per-contest
            // policy overrides are applied. The overlay wins for the
            // six validation policies; everything else in the
            // textarea passes through untouched. Reading from the
            // store at click time means the very latest panel state
            // wins, even if it changed after the textarea was last
            // edited.
            const baseContest = JSON.parse(contestJson) as {
                id?: unknown
            } & Record<string, unknown>
            const id =
                typeof baseContest.id === "string"
                    ? baseContest.id
                    : undefined
            const overlay = id ? contestOverlay : undefined
            const effective = applyPolicyOverlayToContest(
                baseContest,
                overlay
            )
            const effectiveJson =
                effective === baseContest
                    ? contestJson
                    : JSON.stringify(effective)
            // Re-validate: encode each decoded ballot back to a
            // BigInt then re-decode with the effective contest so
            // that all validation checkers (blank, over-vote,
            // under-vote, etc.) re-run under the current policy
            // overlay. This mirrors production, where decode happens
            // at tally time with the authoritative contest config.
            const revalidated = await Promise.all(
                ballots.map(async (ballotJson) => {
                    const bigint = await encodeBallot(
                        effectiveJson,
                        ballotJson
                    )
                    return decodeBigIntToDecodedVoteContest(
                        effectiveJson,
                        bigint
                    )
                })
            )
            const result = await runTally(effectiveJson, revalidated)
            const pretty = JSON.stringify(result, null, 2)
            setOutputJson(pretty)
            const next = adaptVelvetContestResult(result, contestName)
            if (next === null) {
                setError(
                    "tally ran but adapter could not produce a view model — see output JSON below"
                )
            }
            setModel(next)
        } catch (e) {
            setError(formatError(e))
        } finally {
            setBusy(false)
        }
    }, [contestJson, ballotsJson, contestName, contestOverlay])

    const handleRenderOutput = useCallback(() => {
        setError(null)
        try {
            const parsed: unknown = JSON.parse(outputJson)
            const next = adaptVelvetContestResult(parsed, contestName)
            if (next === null) {
                setError(
                    "output JSON does not look like a ContestResult — adapter returned null"
                )
                return
            }
            setModel(next)
        } catch (e) {
            setError(formatError(e))
        }
    }, [outputJson, contestName])

    return (
        <main style={styles.main}>
            <header style={styles.header}>
                <h1 style={styles.h1}>Tally sandbox</h1>
                {contestName ? (
                    <p style={styles.subtitle}>{contestName}</p>
                ) : null}
                <p style={styles.help}>
                    Tally a batch of decoded ballots in isolation. Edit
                    any pane, then <strong>Run tally</strong> to compute
                    from setup + input, or paste a{" "}
                    <code>ContestResult</code> JSON into the output pane
                    and click <strong>Render output</strong> to exercise
                    the visualization without recomputing.
                </p>
            </header>

            <Section title="1. Setup — contest descriptor (JSON Contest)">
                <textarea
                    value={contestJson}
                    onChange={(e) => setContestJson(e.target.value)}
                    style={{...styles.textarea, height: "12rem"}}
                    spellCheck={false}
                    aria-label="Contest JSON"
                />
            </Section>

            <Section title="2. Input ballots — array of DecodedVoteContest objects">
                <textarea
                    value={ballotsJson}
                    onChange={(e) => setBallotsJson(e.target.value)}
                    style={{...styles.textarea, height: "14rem"}}
                    spellCheck={false}
                    aria-label="Decoded ballots JSON array"
                />
                <div style={styles.actions}>
                    <button
                        type="button"
                        onClick={handleRunTally}
                        disabled={busy}
                        style={styles.primaryButton}
                    >
                        {busy ? "Running…" : "Run tally"}
                    </button>
                    {overlayFieldCount > 0 ? (
                        <span
                            style={styles.overlayBadge}
                            title={formatOverlayTitle(contestOverlay)}
                        >
                            {overlayFieldCount} policy override
                            {overlayFieldCount === 1 ? "" : "s"} active
                        </span>
                    ) : null}
                </div>
            </Section>

            <Section title="3. Output — ContestResult JSON (compute target or paste-in source)">
                <textarea
                    value={outputJson}
                    onChange={(e) => setOutputJson(e.target.value)}
                    style={{...styles.textarea, height: "14rem"}}
                    spellCheck={false}
                    aria-label="ContestResult JSON"
                    placeholder='Paste a ContestResult JSON here and click "Render output", or click "Run tally" above to populate this pane from the input ballots.'
                />
                <div style={styles.actions}>
                    <button
                        type="button"
                        onClick={handleRenderOutput}
                        disabled={busy || outputJson.trim().length === 0}
                        style={styles.button}
                        title="Render the visualization from the output
 JSON without recomputing"
                    >
                        Render output
                    </button>
                </div>
            </Section>

            {error ? (
                <div role="alert" style={styles.error}>
                    {error}
                </div>
            ) : null}

            <Section title="4. Visualization">
                {model ? (
                    <ThemeProvider theme={muiDarkTheme}>
                        <CssBaseline enableColorScheme />
                        <p style={styles.algorithmLine}>
                            Counting algorithm:{" "}
                            {model.countingAlgorithm ?? "unknown"} · winners:{" "}
                            {model.winnersCount}
                        </p>
                        {/* `preferential` is left false so the plurality
                            table always renders; the round-by-round view
                            is appended below when velvet emitted one, so
                            the sandbox shows both rather than either. */}
                        <ResultsAndParticipation
                            chartName={model.chartName}
                            summary={model.summary}
                            candidates={model.candidates}
                            preferential={false}
                        />
                        {model.processResults ? (
                            <ThemeProvider theme={muiLightTheme}>
                                <PreferentialCandidateResults
                                    processResults={model.processResults}
                                    candidates={model.candidates}
                                />
                            </ThemeProvider>
                        ) : null}
                    </ThemeProvider>
                ) : (
                    <p style={styles.placeholder}>
                        Run a tally or render an output JSON to see the
                        visualization.
                    </p>
                )}
            </Section>
        </main>
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function Section({
    title,
    children,
}: {
    title: string
    children: React.ReactNode
}) {
    return (
        <section style={styles.section}>
            <h2 style={styles.h2}>{title}</h2>
            {children}
        </section>
    )
}

/** Parse the decoded-ballots textarea. Accepts a JSON array of
 *  `DecodedVoteContest` objects and returns the array of JSON-
 *  stringified entries (the wire shape velvet-wasm's
 *  `tally_decoded_ballots` consumes). Mirrors
 *  `BallotPipeline.tsx::parseBallots`. */
function parseDecodedBallots(json: string): string[] {
    const parsed: unknown = JSON.parse(json)
    if (!Array.isArray(parsed)) {
        throw new Error(
            "input ballots must be a JSON array of DecodedVoteContest objects"
        )
    }
    return parsed.map((entry, i) => {
        if (entry === null || typeof entry !== "object") {
            throw new Error(
                `ballots[${i}] must be a DecodedVoteContest object`
            )
        }
        return JSON.stringify(entry)
    })
}

/** Best-effort read of `contest.name` for the page header. Treat
 *  parse failures as "no name" rather than surfacing an error — the
 *  contest pane is the operator's working area, malformed JSON is
 *  expected mid-edit. */
function readContestName(contestJson: string): string | undefined {
    try {
        const parsed = JSON.parse(contestJson) as unknown
        if (
            parsed &&
            typeof parsed === "object" &&
            "name" in parsed &&
            typeof (parsed as {name: unknown}).name === "string"
        ) {
            return (parsed as {name: string}).name
        }
    } catch {
        // ignore
    }
    return undefined
}

/** Best-effort read of `contest.id` — used to look up the contest's
 *  ephemeral policy overlay. Same parse-failure semantics as
 *  {@link readContestName}. */
function readContestId(contestJson: string): string | undefined {
    try {
        const parsed = JSON.parse(contestJson) as unknown
        if (
            parsed &&
            typeof parsed === "object" &&
            "id" in parsed &&
            typeof (parsed as {id: unknown}).id === "string"
        ) {
            return (parsed as {id: string}).id
        }
    } catch {
        // ignore
    }
    return undefined
}

function formatError(e: unknown): string {
    if (e instanceof Error) return e.message
    return String(e)
}

/** Hover-text for the policy-override badge: one "field = value" line
 *  per active override. */
function formatOverlayTitle(
    overlay: {[key: string]: unknown} | undefined
): string {
    if (!overlay) return ""
    const lines: string[] = []
    for (const [k, v] of Object.entries(overlay)) {
        if (v !== undefined) lines.push(`${k} = ${String(v)}`)
    }
    if (lines.length === 0) return ""
    return `Effective at "Run tally":\n${lines.join("\n")}`
}

// ---------------------------------------------------------------------------
// MUI dark theme for the tally visualization (DataGrid, charts)
// ---------------------------------------------------------------------------

const muiDarkTheme = createTheme({
    palette: {
        mode: "dark",
        background: {default: "#1e1e1e", paper: "#2a2a2a"},
    },
})

// ui-essentials' PreferentialCandidateResults hardcodes light cell
// backgrounds (#FBFBFB / #fff / #F9F9FF) because results-portal renders
// it on a light page. Under muiDarkTheme MUI paints its text white, so
// the candidate column becomes white-on-white. We do not fork the
// upstream component (see LIFTING-TALLY.md), so the round table is given
// a light theme of its own — a light island in the dark chrome, but
// legible and 1:1 with production.
const muiLightTheme = createTheme({palette: {mode: "light"}})

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles: Record<string, CSSProperties> = {
    main: {
        fontFamily: "system-ui, sans-serif",
        padding: "1rem 2rem",
        maxWidth: "70rem",
        margin: "0 auto",
    },
    header: {
        marginBottom: "1rem",
        paddingBottom: "0.5rem",
        borderBottom: "1px solid #3a3a3a",
    },
    h1: {
        fontSize: "1.4rem",
        margin: "0 0 0.2rem 0",
        color: "#e0e0e0",
    },
    subtitle: {
        margin: "0.1rem 0",
        color: "#999",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
        fontSize: "0.9rem",
    },
    section: {
        marginBottom: "1.25rem",
        paddingBottom: "0.75rem",
        borderBottom: "1px solid #3a3a3a",
    },
    h2: {
        fontSize: "1.05rem",
        margin: "0 0 0.4rem 0",
        color: "#e0e0e0",
    },
    textarea: {
        width: "100%",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
        fontSize: "0.8rem",
        padding: "0.5rem",
        boxSizing: "border-box",
        background: "#252525",
        color: "#e0e0e0",
        border: "1px solid #4a4a4a",
        borderRadius: 3,
    },
    actions: {
        display: "flex",
        gap: "0.5rem",
        marginTop: "0.5rem",
        alignItems: "center",
    },
    overlayBadge: {
        padding: "0.2rem 0.55rem",
        fontSize: "0.78rem",
        background: "#3d3000",
        color: "#f0c200",
        border: "1px solid #f0c200",
        borderRadius: "0.25rem",
        cursor: "help",
    },
    button: {
        padding: "0.4rem 0.9rem",
        fontSize: "0.9rem",
        cursor: "pointer",
        background: "#383838",
        color: "#e0e0e0",
        border: "1px solid #555",
        borderRadius: "0.2rem",
    },
    primaryButton: {
        padding: "0.4rem 0.9rem",
        fontSize: "0.9rem",
        cursor: "pointer",
        background: "#2563eb",
        color: "white",
        border: "1px solid #2563eb",
        borderRadius: "0.2rem",
    },
    error: {
        marginBottom: "1rem",
        padding: "0.5rem 0.75rem",
        background: "#3a1c1c",
        border: "1px solid #ef4444",
        borderRadius: "0.25rem",
        color: "#ef4444",
        fontSize: "0.85rem",
        whiteSpace: "pre-wrap",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
    },
    help: {
        fontSize: "0.85rem",
        color: "#999",
        margin: "0.3rem 0 0 0",
    },
    placeholder: {
        color: "#888",
        fontStyle: "italic",
        fontSize: "0.9rem",
    },
    // Counting algorithm / winners line. ui-essentials'
    // ResultsAndParticipation has no slot for these, so the workbench
    // renders them itself above the visualization.
    algorithmLine: {
        color: "#999",
        fontSize: "0.85rem",
        margin: "0 0 12px",
    },
}
