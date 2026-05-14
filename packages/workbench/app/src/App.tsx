// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useState, type CSSProperties} from "react"
import {encodeBallot, getFixtures, runTally} from "./tally"

// Step B workbench page.
//
// Two textareas (Contest JSON, Ballots JSON), an "Encode ballot" helper
// for turning a `DecodedVoteContest` JSON into a `BigUint` ballot
// string, and a "Run tally" button that pipes both textareas into
// `tally_plaintext_ballots` and renders the resulting `ContestResult`.
//
// All inputs are raw JSON for now; structured form fields belong in
// Steps C/D once the booth flow exists.
export function App() {
    const [contestJson, setContestJson] = useState<string>("")
    const [ballotsJson, setBallotsJson] = useState<string>("")
    const [decodedJson, setDecodedJson] = useState<string>("")
    const [encodedBallot, setEncodedBallot] = useState<string | null>(null)
    const [result, setResult] = useState<unknown | null>(null)
    const [error, setError] = useState<string | null>(null)
    const [busy, setBusy] = useState<boolean>(false)

    useEffect(() => {
        ;(async () => {
            try {
                const fixtures = await getFixtures()
                setContestJson(fixtures.contestJson)
                setBallotsJson(fixtures.ballotsJson)
                setDecodedJson(fixtures.decodedVoteContestJson)
            } catch (e) {
                setError(formatError(e))
            }
        })()
    }, [])

    async function handleRun() {
        setBusy(true)
        setError(null)
        setResult(null)
        try {
            const ballots = parseBallots(ballotsJson)
            const tallyResult = await runTally(contestJson, ballots)
            setResult(tallyResult)
        } catch (e) {
            setError(formatError(e))
        } finally {
            setBusy(false)
        }
    }

    async function handleEncode() {
        setBusy(true)
        setError(null)
        setEncodedBallot(null)
        try {
            if (decodedJson.trim().length === 0) {
                throw new Error(
                    "DecodedVoteContest JSON is empty \u2014 paste a JSON object first"
                )
            }
            const encoded = await encodeBallot(contestJson, decodedJson)
            setEncodedBallot(encoded)
        } catch (e) {
            setError(formatError(e))
        } finally {
            setBusy(false)
        }
    }

    return (
        <main style={styles.main}>
            <h1>Sequentech workbench — raw-JSON tally</h1>
            <p>
                Edit the contest definition and ballot list as JSON, then
                run the tally entirely client-side via{" "}
                <code>velvet-wasm</code>.
            </p>

            <section style={styles.section}>
                <h2>Contest JSON</h2>
                <textarea
                    value={contestJson}
                    onChange={(e) => setContestJson(e.target.value)}
                    style={styles.textarea}
                    spellCheck={false}
                />
            </section>

            <section style={styles.section}>
                <h2>Ballots JSON (array of decimal BigUint strings)</h2>
                <textarea
                    value={ballotsJson}
                    onChange={(e) => setBallotsJson(e.target.value)}
                    style={{...styles.textarea, height: "10rem"}}
                    spellCheck={false}
                />
                <button onClick={handleRun} disabled={busy} style={styles.button}>
                    {busy ? "Running…" : "Run tally"}
                </button>
            </section>

            <section style={styles.section}>
                <h2>Encode a single ballot</h2>
                <p style={styles.help}>
                    Paste a <code>DecodedVoteContest</code> JSON object
                    below (contest_id + choices array with{" "}
                    <code>selected</code> indices, <code>-1</code> means
                    "not picked"). The encoded BigUint can be appended
                    to the Ballots JSON above.
                </p>
                <textarea
                    value={decodedJson}
                    onChange={(e) => setDecodedJson(e.target.value)}
                    placeholder='{"contest_id": "…", "choices": [{"id": "…", "selected": 0, "write_in_text": null}, …], "is_explicit_invalid": false, "invalid_errors": [], "invalid_alerts": []}'
                    style={{...styles.textarea, height: "10rem"}}
                    spellCheck={false}
                />
                <button onClick={handleEncode} disabled={busy} style={styles.button}>
                    {busy ? "Encoding…" : "Encode ballot"}
                </button>
                {encodedBallot && (
                    <pre style={styles.output}>{encodedBallot}</pre>
                )}
            </section>

            {error && (
                <section style={styles.section}>
                    <h2 style={{color: "crimson"}}>Error</h2>
                    <pre style={{...styles.output, color: "crimson"}}>
                        {error}
                    </pre>
                </section>
            )}

            {result !== null && (
                <section style={styles.section}>
                    <h2>Tally result</h2>
                    <pre style={styles.output}>
                        {JSON.stringify(result, null, 2)}
                    </pre>
                </section>
            )}
        </main>
    )
}

function parseBallots(json: string): string[] {
    const parsed: unknown = JSON.parse(json)
    if (!Array.isArray(parsed) || !parsed.every((v) => typeof v === "string")) {
        throw new Error(
            "ballots JSON must be an array of decimal BigUint strings"
        )
    }
    return parsed
}

function formatError(e: unknown): string {
    if (e instanceof Error) return e.message
    return String(e)
}

const styles: Record<string, CSSProperties> = {
    main: {
        fontFamily: "system-ui, sans-serif",
        padding: "1rem 2rem",
        maxWidth: "70rem",
        margin: "0 auto",
    },
    section: {
        marginBottom: "1.5rem",
    },
    textarea: {
        width: "100%",
        height: "18rem",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
        fontSize: "0.8rem",
        padding: "0.5rem",
        boxSizing: "border-box",
    },
    button: {
        marginTop: "0.5rem",
        padding: "0.5rem 1rem",
        fontSize: "0.9rem",
        cursor: "pointer",
    },
    output: {
        background: "#f4f4f4",
        padding: "0.75rem",
        overflow: "auto",
        fontSize: "0.8rem",
        wordBreak: "break-all",
        whiteSpace: "pre-wrap",
    },
    help: {
        fontSize: "0.85rem",
        color: "#555",
    },
}
