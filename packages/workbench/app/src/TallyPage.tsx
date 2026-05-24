// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * `/tally` — standalone tally sandbox.
 *
 * Sister tool of `/pipeline` (BallotPipeline.tsx). Where the pipeline
 * exercises the encode/encrypt/decrypt/decode chain end-to-end on a
 * single ballot, this page exercises the **tally** step in isolation
 * and renders its result through the lifted
 * `@sequentech/ui-essentials/TallyResultsView` so the visualization is
 * 1:1 with what admin-portal produces.
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

import {
    TallyResultsView,
    type TallyResultsViewModel,
} from "@sequentech/ui-essentials"

import {runTally} from "./tally"
import {adaptVelvetContestResult} from "./lib/velvetTallyAdapter"

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
    const [model, setModel] = useState<TallyResultsViewModel | null>(() => {
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

    const handleRunTally = useCallback(async () => {
        setBusy(true)
        setError(null)
        try {
            const ballots = parseDecodedBallots(ballotsJson)
            const result = await runTally(contestJson, ballots)
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
    }, [contestJson, ballotsJson, contestName])

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
                    <TallyResultsView model={model} />
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

function formatError(e: unknown): string {
    if (e instanceof Error) return e.message
    return String(e)
}

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
        borderBottom: "1px solid #ddd",
    },
    h1: {
        fontSize: "1.4rem",
        margin: "0 0 0.2rem 0",
    },
    subtitle: {
        margin: "0.1rem 0",
        color: "#444",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
        fontSize: "0.9rem",
    },
    section: {
        marginBottom: "1.25rem",
        paddingBottom: "0.75rem",
        borderBottom: "1px solid #ddd",
    },
    h2: {
        fontSize: "1.05rem",
        margin: "0 0 0.4rem 0",
    },
    textarea: {
        width: "100%",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
        fontSize: "0.8rem",
        padding: "0.5rem",
        boxSizing: "border-box",
    },
    actions: {
        display: "flex",
        gap: "0.5rem",
        marginTop: "0.5rem",
    },
    button: {
        padding: "0.4rem 0.9rem",
        fontSize: "0.9rem",
        cursor: "pointer",
    },
    primaryButton: {
        padding: "0.4rem 0.9rem",
        fontSize: "0.9rem",
        cursor: "pointer",
        background: "#1a73e8",
        color: "white",
        border: "1px solid #1a73e8",
        borderRadius: "0.2rem",
    },
    error: {
        marginBottom: "1rem",
        padding: "0.5rem 0.75rem",
        background: "#fdecea",
        border: "1px solid #f5c2c0",
        borderRadius: "0.25rem",
        color: "#a00",
        fontSize: "0.85rem",
        whiteSpace: "pre-wrap",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
    },
    help: {
        fontSize: "0.85rem",
        color: "#555",
        margin: "0.3rem 0 0 0",
    },
    placeholder: {
        color: "#777",
        fontStyle: "italic",
        fontSize: "0.9rem",
    },
}
