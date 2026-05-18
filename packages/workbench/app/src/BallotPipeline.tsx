// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback, useEffect, useMemo, useState, type CSSProperties} from "react"
import {useLocation} from "react-router-dom"
import {
    decodeBigIntToDecodedVoteContest,
    decryptBallotContent,
    encodeBallot,
    encryptDecodedVoteContest,
    generateKeypair,
    getFixtures,
    runTally,
} from "./tally"

// BallotPipeline — N-ballot playground that walks each selection
// through every transformation a ballot undergoes on its way to the
// tally:
//
//   plaintext  ──encode──▶  encoded BigUint  ──encrypt──▶  ciphertext
//                                                              │
//                                                          decrypt
//                                                              ▼
//   decoded plaintext  ◀──decode──  decrypted BigUint (=encoded)
//                │
//              tally (collective: all rows feed in)
//                ▼
//              result
//
// Each stage holds **one row per ballot**; every row has its own
// textarea + button so operators can probe a single ballot in
// isolation, and per-row errors are kept independent so a broken
// envelope in row 3 does not mask successful round-trips in rows
// 1-2. A "Run on all" button per stage is the obvious
// quality-of-life affordance for the common case (replay every
// captured cast vote through the same step).
//
// Contest + keypair are pipeline-wide (Setup section) so encrypt and
// decrypt agree across every row.
//
// **Seeding from ContestDetailPage.** When the operator clicks
// "Open in ballot pipeline" from a contest page, the inspector
// navigates here with a {@link PipelineSeed} in react-router's
// location state. The seed carries the contest JSON, the snapshot
// keypair, and one row per cast vote on that contest pre-filled
// with the captured plaintext / encrypted envelope / decoded
// BigUint. The pipeline then becomes a side-by-side round-trip
// oracle: operator re-runs encode/encrypt/decrypt/decode and
// visually compares the computed values against the captured ones
// in the textarea below (every cell is read-write, so any cell can
// be edited and the downstream stage re-run to probe failures).
//
// React-router state was chosen for the seed instead of the
// workbench store because the seed is a one-shot navigation payload:
// it does not need to survive a reload, does not need subscribers,
// and must not be written into the persisted snapshot (a freshly
// reloaded pipeline should fall back to velvet-wasm fixtures).

/** One row of the pipeline. A row corresponds to a single ballot's
 *  trip through encode → encrypt → decrypt → decode. */
interface PipelineRow {
    /** Stable client-side id so React keys survive add/remove. */
    rowId: string
    /** Optional human label (e.g. cast-vote id prefix when seeded
     *  from a contest page). Shown in each stage's row header. */
    label?: string
    /** Stage 1 — structured `DecodedVoteContest` JSON. */
    plaintextJson: string
    /** Stage 2 — decimal `BigUint` produced by `encodeBallot`. */
    encodedBigInt: string
    /** Stage 3 — `{contests: ["<b64>"]}` JSON envelope. Same shape
     *  the portal stores in `castVote.content`. */
    encryptedJson: string
    /** Stage 4 — decimal `BigUint` produced by
     *  `decryptBallotContent`. Round-trip invariant: should equal
     *  `encodedBigInt`. */
    decryptedBigInt: string
    /** Stage 5 — structured `DecodedVoteContest` JSON produced by
     *  `decodeBigIntToDecodedVoteContest`. Round-trip invariant:
     *  should match `plaintextJson` after normalisation. */
    decodedJson: string
    errors: Partial<Record<StageKey, string>>
    busy: Partial<Record<StageKey, true>>
}

type StageKey = "encode" | "encrypt" | "decrypt" | "decode"

/** Where each stage sits in the visible pipeline (1-based) and which
 *  cell it populates downstream. Used to drive auto-expansion of
 *  collapsed row textareas: running stage X reveals stage X+1's cell
 *  on success (the populated output) and re-reveals stage X on
 *  failure (so the error is visible). The terminal `decode` stage
 *  populates stage 5, which is read-only (no button). */
const STAGE_FLOW: Record<StageKey, {source: number; target: number}> = {
    encode: {source: 1, target: 2},
    encrypt: {source: 2, target: 3},
    decrypt: {source: 3, target: 4},
    decode: {source: 4, target: 5},
}

/** Compose the `expanded` set key for one row's textarea in one
 *  stage. Format: `<rowId>#<stageIndex>` (1-based). */
const expansionKey = (rowId: string, stageIndex: number): string =>
    `${rowId}#${stageIndex}`

/** Navigation payload accepted by the `/pipeline` route. Built by
 *  the inspector's "Open in ballot pipeline" button on a contest
 *  page; each row pre-fills the cells it has data for and leaves
 *  the rest blank for the operator to compute. */
export interface PipelineSeed {
    /** Optional display name of the contest the seed was built from.
     *  Used purely to title the pipeline view (`Ballot pipeline —
     *  <name> (N cast votes)`); never read by stage logic. The
     *  inspector populates it from `contest.name`; older seeds
     *  without this field fall back to parsing `contestJson`. */
    contestName?: string
    contestJson: string
    pkB64: string
    skB64: string
    rows: PipelineSeedRow[]
}

export interface PipelineSeedRow {
    label?: string
    plaintextJson?: string
    encryptedJson?: string
    decryptedBigInt?: string
}

export function BallotPipeline() {
    const location = useLocation()
    const seed = (location.state ?? null) as PipelineSeed | null

    const [contestJson, setContestJson] = useState<string>(
        seed?.contestJson ?? ""
    )
    const [pkB64, setPkB64] = useState<string>(seed?.pkB64 ?? "")
    const [skB64, setSkB64] = useState<string>(seed?.skB64 ?? "")
    const [rows, setRows] = useState<PipelineRow[]>(() =>
        seed ? seed.rows.map(makeRowFromSeed) : []
    )
    const [tallyBallots, setTallyBallots] = useState<string>("")
    const [tallyResult, setTallyResult] = useState<unknown | null>(null)
    const [tallyError, setTallyError] = useState<string | null>(null)
    const [setupError, setSetupError] = useState<string | null>(null)
    const [tallyBusy, setTallyBusy] = useState<boolean>(false)

    // Collapse state for per-row stage textareas. Keys are
    // `expansionKey(rowId, stageIndex)`; a key in the set means the
    // textarea is currently shown. Defaults to **collapsed** to keep
    // the page compact (textareas are bulky ciphertext / proof blobs
    // that the operator rarely needs to read in full). Stage 1 of
    // each row is expanded on row creation when the page is not
    // seeded — that's the only stage the operator can meaningfully
    // edit before pressing anything. When seeded, every row lands
    // collapsed; the operator clicks a row header to inspect.
    const [expanded, setExpanded] = useState<Set<string>>(() => {
        if (!seed) return new Set()
        // Seeded rows: nothing expanded by default. Operator opens
        // what they want to inspect.
        return new Set()
    })

    // Collapse state for the two non-row textareas: Setup's Contest
    // JSON and Tally's ballots array. Same motivation as the per-row
    // stage textareas \u2014 these blobs (a multi-kilobyte contest, a
    // BigUint array) dominate vertical space and the operator rarely
    // needs to read them once the page is configured.
    //
    // Contest JSON: in standalone mode the bootstrap fixture is the
    // most common starting point the operator wants to *change*
    // (pick a different contest shape before encoding), so we land
    // expanded. When seeded from a contest the JSON is fixed by the
    // upstream cast votes and is rarely re-read \u2014 land collapsed.
    const [contestJsonOpen, setContestJsonOpen] = useState<boolean>(
        () => !seed
    )
    const [tallyBallotsOpen, setTallyBallotsOpen] = useState<boolean>(false)

    /** Set membership flip for `expansionKey(rowId, stageIndex)`. */
    const toggleExpanded = useCallback(
        (rowId: string, stageIndex: number) => {
            const k = expansionKey(rowId, stageIndex)
            setExpanded((prev) => {
                const next = new Set(prev)
                if (next.has(k)) next.delete(k)
                else next.add(k)
                return next
            })
        },
        []
    )

    /** Idempotent expand — no state churn if every key is already in
     *  the set. Used by auto-expand hooks (run buttons, add-row,
     *  bootstrap) which fire on every stage completion. */
    const expandKeys = useCallback((keys: string[]) => {
        if (keys.length === 0) return
        setExpanded((prev) => {
            let dirty = false
            const next = new Set(prev)
            for (const k of keys) {
                if (!next.has(k)) {
                    next.add(k)
                    dirty = true
                }
            }
            return dirty ? next : prev
        })
    }, [])

    // First-mount bootstrap for the un-seeded case: pull velvet-wasm
    // fixtures so the page is immediately interactive with one
    // example ballot. When the page is opened from a contest seed we
    // keep the seeded state instead.
    useEffect(() => {
        if (seed) return
        ;(async () => {
            try {
                const fixtures = await getFixtures()
                setContestJson(fixtures.contestJson)
                setTallyBallots(fixtures.ballotsJson)
                const kp = await generateKeypair()
                setPkB64(kp.pkB64)
                setSkB64(kp.skB64)
                const bootstrapRow = makeEmptyRow({
                    plaintextJson: fixtures.decodedVoteContestJson,
                })
                setRows([bootstrapRow])
                // Un-seeded bootstrap: expand the editable input
                // (stage 1, the plaintext cell) so the operator can
                // see and tweak the fixture before pressing Encode.
                expandKeys([expansionKey(bootstrapRow.rowId, 1)])
            } catch (e) {
                setSetupError(formatError(e))
            }
        })()
        // Intentionally empty deps: this is a one-shot bootstrap.
        // `seed` is captured from location.state and only matters on
        // the initial render — a navigation that lands here again
        // with a different seed will remount the component.
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    /** Per-row mutation: replace one row, keeping others by identity. */
    const patchRow = useCallback(
        (rowId: string, patch: (r: PipelineRow) => PipelineRow) => {
            setRows((rs) => rs.map((r) => (r.rowId === rowId ? patch(r) : r)))
        },
        []
    )

    const runStage = useCallback(
        async (
            rowId: string,
            stage: StageKey,
            opts?: {autoExpand?: boolean}
        ) => {
            const autoExpand = opts?.autoExpand ?? true
            const row = rows.find((r) => r.rowId === rowId)
            if (!row) return
            patchRow(rowId, (r) => ({
                ...r,
                busy: {...r.busy, [stage]: true as const},
                errors: stripError(r.errors, stage),
            }))
            try {
                const next = await executeStage(stage, row, {
                    contestJson,
                    pkB64,
                    skB64,
                })
                patchRow(rowId, (r) => ({
                    ...r,
                    ...next,
                    busy: stripBusy(r.busy, stage),
                }))
                // Reveal the cell that was just populated so the
                // operator actually sees the output of the click.
                if (autoExpand) {
                    expandKeys([
                        expansionKey(rowId, STAGE_FLOW[stage].target),
                    ])
                }
            } catch (e) {
                patchRow(rowId, (r) => ({
                    ...r,
                    busy: stripBusy(r.busy, stage),
                    errors: {...r.errors, [stage]: formatError(e)},
                }))
                // Reveal the source cell so the error message (which
                // renders inside the row card) is visible. Errors are
                // always surfaced — even on auto-expand-suppressed
                // background replays — because a silent failure would
                // be misleading.
                expandKeys([expansionKey(rowId, STAGE_FLOW[stage].source)])
            }
        },
        [rows, contestJson, pkB64, skB64, patchRow, expandKeys]
    )

    // Seeded rows arrive with `plaintextJson`, `encryptedJson` and
    // `decryptedBigInt` filled in (see WorkbenchInspector's
    // `handleOpenInPipeline`), but never with `encodedBigInt`:
    // production never persists the encoded BigUint — it's a
    // transient intermediate inside `encrypt_decoded_contest`
    // (sequent-core/src/encrypt.rs, the `contest.encode_plaintext_contest(&decoded)`
    // call), discarded as soon as it's encrypted. To avoid a
    // counterintuitive empty cell between two filled ones we
    // re-compute it workbench-side by replaying the Encode stage
    // on each seeded row. The work happens after mount (mirrors
    // the decrypt-bridge's async fill, §M.3 in LIFTING.md) so
    // navigation into `/pipeline` stays snappy and per-row errors
    // surface in the normal error slot.
    useEffect(() => {
        if (!seed) return
        rows.forEach((r) => {
            if (r.plaintextJson.trim() && !r.encodedBigInt.trim()) {
                // `autoExpand: false` keeps the seeded view compact —
                // this replay is bookkeeping (production never
                // persists `encodedBigInt`; see the comment above)
                // not an operator-initiated click, so opening stage 2
                // for every row on landing would defeat the whole
                // collapsed-by-default point of the seeded view.
                void runStage(r.rowId, "encode", {autoExpand: false})
            }
        })
        // Intentionally empty deps: one-shot, mirrors the seed
        // bootstrap effect above.
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    /** Run a stage on every row that has its input cell populated.
     *  Per-row failures are recorded on the row; the loop never
     *  aborts. */
    const runStageOnAll = useCallback(
        async (stage: StageKey) => {
            const ids = rows
                .filter((r) => hasStageInput(r, stage))
                .map((r) => r.rowId)
            await Promise.all(ids.map((id) => runStage(id, stage)))
        },
        [rows, runStage]
    )

    const handleNewKeypair = useCallback(async () => {
        setSetupError(null)
        try {
            const kp = await generateKeypair()
            setPkB64(kp.pkB64)
            setSkB64(kp.skB64)
        } catch (e) {
            setSetupError(formatError(e))
        }
    }, [])

    const handleAddRow = useCallback(() => {
        const fresh = makeEmptyRow({})
        setRows((rs) => [...rs, fresh])
        // New rows are blank: open stage 1 so the operator has a
        // visible input target without an extra click.
        expandKeys([expansionKey(fresh.rowId, 1)])
    }, [expandKeys])

    const handleRemoveRow = useCallback((rowId: string) => {
        setRows((rs) => rs.filter((r) => r.rowId !== rowId))
    }, [])

    /** Seed the tally textarea from each row's decrypted-or-encoded
     *  BigUint. Rows without either are skipped. */
    const handleSeedTally = useCallback(() => {
        const list = rows
            .map((r) => r.decryptedBigInt.trim() || r.encodedBigInt.trim())
            .filter((s) => s.length > 0)
        if (list.length === 0) {
            setTallyError(
                "no encoded ballots in any row — run encode or decrypt first"
            )
            return
        }
        setTallyError(null)
        setTallyBallots(JSON.stringify(list, null, 2))
        // The "Seed tally from rows" button is the populate trigger
        // for this textarea; mirror the per-row auto-expand contract
        // so the operator sees what was just written.
        setTallyBallotsOpen(true)
    }, [rows])

    const handleRunTally = useCallback(async () => {
        setTallyBusy(true)
        setTallyError(null)
        try {
            const ballots = parseBallots(tallyBallots)
            const result = await runTally(contestJson, ballots)
            setTallyResult(result)
        } catch (e) {
            setTallyError(formatError(e))
        } finally {
            setTallyBusy(false)
        }
    }, [contestJson, tallyBallots])

    // Per-stage "any row busy" flag, used to gate the per-stage
    // "Run on all" buttons (cheap to compute and avoids double-fires).
    const anyBusy = useMemo(
        () => rows.some((r) => Object.keys(r.busy).length > 0),
        [rows]
    )

    // Title suffix: when the page was opened from a contest in the
    // inspector, surface the contest name + cast-vote count so the
    // operator does not have to scroll down to the Setup section to
    // confirm where they are. Falls back to parsing the (possibly
    // older) seed's `contestJson` if `contestName` is absent.
    const titleSuffix = useMemo(() => {
        if (!seed) return ""
        let name = seed.contestName
        if (!name) {
            try {
                const parsed = JSON.parse(seed.contestJson) as {
                    name?: unknown
                }
                if (typeof parsed?.name === "string") name = parsed.name
            } catch {
                /* fall through to placeholder */
            }
        }
        const displayName = name ?? "(unnamed contest)"
        const n = seed.rows.length
        return ` — ${displayName} (${n} cast vote${n === 1 ? "" : "s"})`
    }, [seed])

    return (
        // `div`, not `main`: when mounted under `InspectorLayout` the
        // layout's own `<main>` already wraps the routed outlet, so a
        // second `<main>` here would nest the landmarks. The visual
        // styling (centered narrow column) is preserved via `styles.main`.
        <div style={styles.main}>
            <h1>Ballot pipeline{titleSuffix}</h1>
            <p style={styles.help}>
                Walk one or more ballot selections through the full
                encode → encrypt → decrypt → decode → tally chain.
                Each row is an independent ballot; every cell is
                editable, and per-row "Run" buttons rerun a single
                stage on a single ballot. The "Run on all" button in
                each stage replays that stage across every row.
            </p>

            <Section title="Setup">
                <CollapsibleField
                    label="Contest JSON"
                    value={contestJson}
                    open={contestJsonOpen}
                    onToggle={() => setContestJsonOpen((v) => !v)}
                >
                    <textarea
                        value={contestJson}
                        onChange={(e) => setContestJson(e.target.value)}
                        style={{...styles.textarea, height: "12rem"}}
                        spellCheck={false}
                    />
                </CollapsibleField>
                <Field label="Public key (base64-no-pad)">
                    <input
                        value={pkB64}
                        onChange={(e) => setPkB64(e.target.value)}
                        style={styles.input}
                        spellCheck={false}
                    />
                </Field>
                <Field label="Secret key (base64-no-pad)">
                    <input
                        value={skB64}
                        onChange={(e) => setSkB64(e.target.value)}
                        style={styles.input}
                        spellCheck={false}
                    />
                </Field>
                <button
                    onClick={handleNewKeypair}
                    style={styles.button}
                    disabled={!!seed}
                    title={
                        seed
                            ? "Disabled: seeded ciphertexts were encrypted with the contest's keypair. Regenerating would make every Decrypt stage fail."
                            : undefined
                    }
                >
                    Generate new keypair
                </button>
                {setupError && (
                    <pre style={{...styles.output, color: "crimson"}}>
                        {setupError}
                    </pre>
                )}
                {seed && (
                    <>
                        <p style={styles.help}>
                            Seeded from inspector ({seed.rows.length}{" "}
                            ballot{seed.rows.length === 1 ? "" : "s"}).
                        </p>
                        {/* Each seeded row's `encryptedJson` was produced
                         *  upstream with the contest's keypair (the one
                         *  shown above). Editing pk/sk or generating a
                         *  new pair here would orphan those ciphertexts:
                         *  Decrypt would fail (wrong sk) and a re-Encrypt
                         *  with the new pk would silently invalidate the
                         *  link back to the original cast vote. We keep
                         *  the inputs editable for teaching purposes
                         *  (operator can paste a wrong key to *see* the
                         *  failure) but warn explicitly and disable the
                         *  one-click regen button above. */}
                        <p
                            style={{
                                ...styles.help,
                                color: "#b22222",
                            }}
                        >
                            Keypair is bound to the seeded ciphertexts.
                            Changing pk/sk or regenerating will make
                            Decrypt fail on every row — the rows above
                            were encrypted with the keypair shown.
                        </p>
                    </>
                )}
            </Section>

            <Stage
                index={1}
                title="Plaintext (DecodedVoteContest)"
                help='Structured selection: {"contest_id": "…", "choices": [{"id": "…", "selected": 0, "write_in_text": null}, …], …}. selected = -1 means "not picked".'
                stage="encode"
                buttonLabel="Encode ▼"
                rows={rows}
                cellOf={(r) => r.plaintextJson}
                setCell={(rowId, v) =>
                    patchRow(rowId, (r) => ({...r, plaintextJson: v}))
                }
                onRunRow={runStage}
                onRunAll={runStageOnAll}
                anyBusy={anyBusy}
                onAddRow={handleAddRow}
                onRemoveRow={handleRemoveRow}
                expanded={expanded}
                onToggleExpanded={toggleExpanded}
            />

            <Stage
                index={2}
                title="Encoded plaintext (decimal BigUint)"
                help="The encrypt step actually re-runs encode internally to obtain the [u8;30] plaintext element; the BigUint here is shown so you can compare it byte-for-byte against the decrypted BigUint downstream."
                stage="encrypt"
                buttonLabel="Encrypt ▼"
                rows={rows}
                cellOf={(r) => r.encodedBigInt}
                setCell={(rowId, v) =>
                    patchRow(rowId, (r) => ({...r, encodedBigInt: v}))
                }
                onRunRow={runStage}
                onRunAll={runStageOnAll}
                anyBusy={anyBusy}
                small
                expanded={expanded}
                onToggleExpanded={toggleExpanded}
            />

            <Stage
                index={3}
                title="Encrypted ballot envelope (HashableBallot JSON)"
                help='{"contests": ["<base64 of HashableBallotContest>"]}. Same shape the portal stores in castVote.content, so this can be hand-pasted from a real cast vote too.'
                stage="decrypt"
                buttonLabel="Decrypt ▼"
                rows={rows}
                cellOf={(r) => r.encryptedJson}
                setCell={(rowId, v) =>
                    patchRow(rowId, (r) => ({...r, encryptedJson: v}))
                }
                onRunRow={runStage}
                onRunAll={runStageOnAll}
                anyBusy={anyBusy}
                expanded={expanded}
                onToggleExpanded={toggleExpanded}
            />

            <Stage
                index={4}
                title="Decrypted plaintext (= encoded BigUint)"
                help="Round-trip check: this should be byte-identical to the encoded BigUint in step 2."
                stage="decode"
                buttonLabel="Decode ▼"
                rows={rows}
                cellOf={(r) => r.decryptedBigInt}
                setCell={(rowId, v) =>
                    patchRow(rowId, (r) => ({...r, decryptedBigInt: v}))
                }
                onRunRow={runStage}
                onRunAll={runStageOnAll}
                anyBusy={anyBusy}
                small
                expanded={expanded}
                onToggleExpanded={toggleExpanded}
            />

            <Stage
                index={5}
                title="Decoded plaintext (DecodedVoteContest)"
                help="Round-trip check: each row should match its step-1 plaintext after normalisation."
                stage={null}
                buttonLabel={null}
                rows={rows}
                cellOf={(r) => r.decodedJson}
                setCell={(rowId, v) =>
                    patchRow(rowId, (r) => ({...r, decodedJson: v}))
                }
                onRunRow={runStage}
                onRunAll={runStageOnAll}
                anyBusy={anyBusy}
                expanded={expanded}
                onToggleExpanded={toggleExpanded}
            />

            <Section title="6. Tally ballots (array of decimal BigUint strings)">
                <button onClick={handleSeedTally} style={styles.button}>
                    Seed tally from rows ▼
                </button>
                <CollapsibleField
                    label="Ballots array"
                    value={tallyBallots}
                    open={tallyBallotsOpen}
                    onToggle={() => setTallyBallotsOpen((v) => !v)}
                >
                    <textarea
                        value={tallyBallots}
                        onChange={(e) => setTallyBallots(e.target.value)}
                        style={{...styles.textarea, height: "10rem"}}
                        spellCheck={false}
                    />
                </CollapsibleField>
                <button
                    onClick={handleRunTally}
                    disabled={tallyBusy}
                    style={styles.button}
                >
                    {tallyBusy ? "Tally…" : "Run tally"}
                </button>
                {tallyError && (
                    <pre style={{...styles.output, color: "crimson"}}>
                        {tallyError}
                    </pre>
                )}
                {tallyResult !== null && (
                    <pre style={styles.output}>
                        {JSON.stringify(tallyResult, null, 2)}
                    </pre>
                )}
            </Section>
        </div>
    )
}

// ---------------------------------------------------------------------------
// Stage component (renders one stage across all rows)
// ---------------------------------------------------------------------------

interface StageProps {
    index: number
    title: string
    help?: string
    /** Which transformation this stage's button runs on a row. `null`
     *  for the final stage (decoded plaintext) which has no
     *  downstream button. */
    stage: StageKey | null
    buttonLabel: string | null
    rows: PipelineRow[]
    cellOf: (r: PipelineRow) => string
    setCell: (rowId: string, value: string) => void
    onRunRow: (rowId: string, stage: StageKey) => void
    onRunAll: (stage: StageKey) => void
    anyBusy: boolean
    small?: boolean
    onAddRow?: () => void
    onRemoveRow?: (rowId: string) => void
    /** Set of `expansionKey(rowId, stageIndex)` strings that are
     *  currently expanded. A row whose key is absent renders only
     *  its header (chevron + badge + label + char count) and hides
     *  the textarea + Run button + error to keep the page compact. */
    expanded: Set<string>
    /** Click handler for the row header disclosure button. */
    onToggleExpanded: (rowId: string, stageIndex: number) => void
}

function Stage(props: StageProps): JSX.Element {
    const {
        index,
        title,
        help,
        stage,
        buttonLabel,
        rows,
        cellOf,
        setCell,
        onRunRow,
        onRunAll,
        anyBusy,
        small,
        onAddRow,
        onRemoveRow,
        expanded,
        onToggleExpanded,
    } = props
    return (
        <section style={styles.section}>
            <div style={styles.stageHeader}>
                <h2 style={styles.h2}>
                    {index}. {title}
                </h2>
                {stage && buttonLabel && (
                    <button
                        onClick={() => onRunAll(stage)}
                        disabled={anyBusy || rows.length === 0}
                        style={styles.runAllButton}
                        title={`Run ${stage} on every row`}
                    >
                        Run on all ▼
                    </button>
                )}
            </div>
            {help && <p style={styles.help}>{help}</p>}
            {rows.length === 0 && (
                <p style={styles.empty}>(no ballots — add one below)</p>
            )}
            {rows.map((row, i) => {
                const busy = stage ? !!row.busy[stage] : false
                const err = stage ? row.errors[stage] : undefined
                const cell = cellOf(row)
                const isOpen = expanded.has(expansionKey(row.rowId, index))
                return (
                    <div key={row.rowId} style={styles.rowCard}>
                        <div style={styles.rowHeader}>
                            <button
                                type="button"
                                onClick={() =>
                                    onToggleExpanded(row.rowId, index)
                                }
                                aria-expanded={isOpen}
                                style={styles.disclosure}
                                title={
                                    isOpen
                                        ? "Collapse this cell"
                                        : "Expand this cell"
                                }
                            >
                                <span
                                    aria-hidden="true"
                                    style={styles.chevron}
                                >
                                    {isOpen ? "▾" : "▸"}
                                </span>
                                <span style={styles.rowBadge}>
                                    #{i + 1}
                                </span>
                                {row.label && (
                                    <span style={styles.rowLabel}>
                                        {row.label}
                                    </span>
                                )}
                                <span style={styles.charCount}>
                                    {formatCharCount(cell.length)}
                                </span>
                                {err && !isOpen && (
                                    <span
                                        style={styles.errorBadge}
                                        title={err}
                                    >
                                        error
                                    </span>
                                )}
                            </button>
                            {onRemoveRow && (
                                <button
                                    onClick={() => onRemoveRow(row.rowId)}
                                    style={styles.removeButton}
                                    title="Remove this ballot from every stage"
                                >
                                    ×
                                </button>
                            )}
                        </div>
                        {isOpen && (
                            <>
                                <textarea
                                    value={cell}
                                    onChange={(e) =>
                                        setCell(row.rowId, e.target.value)
                                    }
                                    style={{
                                        ...styles.textarea,
                                        height: small ? "5rem" : "10rem",
                                    }}
                                    spellCheck={false}
                                />
                                {stage && buttonLabel && (
                                    <button
                                        onClick={() =>
                                            onRunRow(row.rowId, stage)
                                        }
                                        disabled={busy}
                                        style={styles.rowButton}
                                    >
                                        {busy
                                            ? `${buttonLabel.replace(/ ▼$/, "")}…`
                                            : buttonLabel}
                                    </button>
                                )}
                                {err && (
                                    <pre
                                        style={{
                                            ...styles.output,
                                            color: "crimson",
                                            marginTop: "0.3rem",
                                        }}
                                    >
                                        {err}
                                    </pre>
                                )}
                            </>
                        )}
                    </div>
                )
            })}
            {onAddRow && (
                <button
                    onClick={onAddRow}
                    style={{...styles.button, marginTop: "0.5rem"}}
                >
                    + Add ballot
                </button>
            )}
        </section>
    )
}

/** Compact char-count label for collapsed row headers. Uses `k` for
 *  thousands so the header width stays predictable even for
 *  multi-kilobyte ciphertexts. Empty cells read as `empty` rather
 *  than `0 chars` to make "no data yet" visually distinct from
 *  "small data". */
function formatCharCount(n: number): string {
    if (n === 0) return "empty"
    if (n < 1000) return `${n} chars`
    if (n < 100_000) return `${(n / 1000).toFixed(1)}k chars`
    return `${Math.round(n / 1000)}k chars`
}

// ---------------------------------------------------------------------------
// Stage execution: pure-ish; reads from a row, returns a partial row
// ---------------------------------------------------------------------------

interface PipelineContext {
    contestJson: string
    pkB64: string
    skB64: string
}

async function executeStage(
    stage: StageKey,
    row: PipelineRow,
    ctx: PipelineContext
): Promise<Partial<PipelineRow>> {
    switch (stage) {
        case "encode": {
            const bigint = await encodeBallot(
                ctx.contestJson,
                row.plaintextJson
            )
            return {encodedBigInt: bigint}
        }
        case "encrypt": {
            if (!ctx.pkB64) throw new Error("public key is empty")
            // Encrypt uses the structured plaintext (re-encodes
            // internally to the [u8;30] element); see tally.ts.
            const envelope = await encryptDecodedVoteContest(
                ctx.contestJson,
                row.plaintextJson,
                ctx.pkB64
            )
            return {encryptedJson: prettyJson(envelope)}
        }
        case "decrypt": {
            if (!ctx.skB64) throw new Error("secret key is empty")
            const contestId = readContestId(ctx.contestJson)
            const bigint = await decryptBallotContent(
                row.encryptedJson,
                ctx.skB64,
                contestId
            )
            return {decryptedBigInt: bigint}
        }
        case "decode": {
            const decoded = await decodeBigIntToDecodedVoteContest(
                ctx.contestJson,
                row.decryptedBigInt.trim()
            )
            return {decodedJson: prettyJson(decoded)}
        }
    }
}

/** Does the row have non-empty input for the given stage? Used to
 *  skip rows that would only fail with "empty input" on "Run on
 *  all" — those failures would clutter the per-row error display. */
function hasStageInput(row: PipelineRow, stage: StageKey): boolean {
    switch (stage) {
        case "encode":
        case "encrypt":
            return row.plaintextJson.trim().length > 0
        case "decrypt":
            return row.encryptedJson.trim().length > 0
        case "decode":
            return row.decryptedBigInt.trim().length > 0
    }
}

// ---------------------------------------------------------------------------
// Row factories
// ---------------------------------------------------------------------------

function makeEmptyRow(init: {
    plaintextJson?: string
    encryptedJson?: string
    decryptedBigInt?: string
    label?: string
}): PipelineRow {
    return {
        rowId: generateRowId(),
        label: init.label,
        plaintextJson: init.plaintextJson ?? "",
        encodedBigInt: "",
        encryptedJson: init.encryptedJson ?? "",
        decryptedBigInt: init.decryptedBigInt ?? "",
        decodedJson: "",
        errors: {},
        busy: {},
    }
}

function makeRowFromSeed(seed: PipelineSeedRow): PipelineRow {
    return makeEmptyRow({
        label: seed.label,
        plaintextJson: seed.plaintextJson,
        encryptedJson: seed.encryptedJson,
        decryptedBigInt: seed.decryptedBigInt,
    })
}

function generateRowId(): string {
    if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
        return crypto.randomUUID()
    }
    return `row-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

// ---------------------------------------------------------------------------
// Small helpers
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

function Field({
    label,
    children,
}: {
    label: string
    children: React.ReactNode
}) {
    return (
        <div style={{marginBottom: "0.5rem"}}>
            <label style={styles.label}>{label}</label>
            {children}
        </div>
    )
}

/** A `Field` whose label doubles as a disclosure button: the body
 *  (typically a bulky textarea) is hidden until the operator clicks
 *  the header. Mirrors the per-row stage collapse pattern for the
 *  two large non-row textareas on the page (Setup contest JSON and
 *  Tally ballots array). `value` is consulted only to render the
 *  char-count breadcrumb \u2014 the actual textarea inside `children`
 *  remains the source of truth. */
function CollapsibleField({
    label,
    value,
    open,
    onToggle,
    children,
}: {
    label: string
    value: string
    open: boolean
    onToggle: () => void
    children: React.ReactNode
}) {
    return (
        <div style={{marginBottom: "0.5rem"}}>
            <button
                type="button"
                onClick={onToggle}
                aria-expanded={open}
                style={{
                    ...styles.disclosure,
                    marginBottom: "0.2rem",
                }}
                title={open ? "Collapse" : "Expand"}
            >
                <span aria-hidden="true" style={styles.chevron}>
                    {open ? "▾" : "▸"}
                </span>
                <span style={{fontSize: "0.8rem", color: "#333"}}>
                    {label}
                </span>
                <span style={styles.charCount}>
                    {formatCharCount(value.length)}
                </span>
            </button>
            {open && children}
        </div>
    )
}

function parseBallots(json: string): string[] {
    const parsed: unknown = JSON.parse(json)
    if (
        !Array.isArray(parsed) ||
        !parsed.every((v) => typeof v === "string")
    ) {
        throw new Error(
            "ballots JSON must be an array of decimal BigUint strings"
        )
    }
    return parsed
}

function readContestId(contestJson: string): string {
    const parsed: unknown = JSON.parse(contestJson)
    if (
        parsed &&
        typeof parsed === "object" &&
        "id" in parsed &&
        typeof (parsed as {id: unknown}).id === "string"
    ) {
        return (parsed as {id: string}).id
    }
    throw new Error("contest JSON has no string `id` field")
}

function prettyJson(s: string): string {
    try {
        return JSON.stringify(JSON.parse(s), null, 2)
    } catch {
        return s
    }
}

function formatError(e: unknown): string {
    if (e instanceof Error) return e.message
    return String(e)
}

function stripError(
    errors: PipelineRow["errors"],
    stage: StageKey
): PipelineRow["errors"] {
    if (!(stage in errors)) return errors
    const next = {...errors}
    delete next[stage]
    return next
}

function stripBusy(
    busy: PipelineRow["busy"],
    stage: StageKey
): PipelineRow["busy"] {
    if (!(stage in busy)) return busy
    const next = {...busy}
    delete next[stage]
    return next
}

const styles: Record<string, CSSProperties> = {
    main: {
        fontFamily: "system-ui, sans-serif",
        padding: "1rem 2rem",
        maxWidth: "70rem",
        margin: "0 auto",
    },
    section: {
        marginBottom: "1.25rem",
        paddingBottom: "0.75rem",
        borderBottom: "1px solid #ddd",
    },
    stageHeader: {
        display: "flex",
        alignItems: "baseline",
        gap: "0.75rem",
        marginBottom: "0.3rem",
    },
    h2: {
        fontSize: "1.05rem",
        margin: "0 0 0.4rem 0",
    },
    rowCard: {
        border: "1px solid #e5e5e5",
        borderRadius: "0.25rem",
        padding: "0.5rem",
        marginBottom: "0.5rem",
        background: "#fafafa",
    },
    rowHeader: {
        display: "flex",
        alignItems: "center",
        gap: "0.5rem",
        marginBottom: "0.3rem",
    },
    rowBadge: {
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
        fontSize: "0.75rem",
        background: "#333",
        color: "white",
        padding: "0.1rem 0.4rem",
        borderRadius: "0.2rem",
    },
    rowLabel: {
        fontSize: "0.8rem",
        color: "#444",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
    },
    removeButton: {
        marginLeft: "auto",
        border: "1px solid #ccc",
        background: "white",
        cursor: "pointer",
        padding: "0 0.4rem",
        fontSize: "0.9rem",
        lineHeight: 1.4,
    },
    rowButton: {
        marginTop: "0.35rem",
        padding: "0.3rem 0.7rem",
        fontSize: "0.85rem",
        cursor: "pointer",
    },
    runAllButton: {
        padding: "0.25rem 0.6rem",
        fontSize: "0.8rem",
        cursor: "pointer",
    },
    textarea: {
        width: "100%",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
        fontSize: "0.8rem",
        padding: "0.5rem",
        boxSizing: "border-box",
    },
    input: {
        width: "100%",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
        fontSize: "0.8rem",
        padding: "0.35rem 0.5rem",
        boxSizing: "border-box",
    },
    button: {
        marginTop: "0.5rem",
        padding: "0.4rem 0.9rem",
        fontSize: "0.9rem",
        cursor: "pointer",
    },
    output: {
        background: "#f4f4f4",
        padding: "0.75rem",
        marginTop: "0.5rem",
        overflow: "auto",
        fontSize: "0.8rem",
        wordBreak: "break-all",
        whiteSpace: "pre-wrap",
    },
    help: {
        fontSize: "0.8rem",
        color: "#555",
        margin: "0 0 0.4rem 0",
    },
    empty: {
        fontSize: "0.8rem",
        color: "#888",
        fontStyle: "italic",
    },
    label: {
        display: "block",
        fontSize: "0.8rem",
        color: "#333",
        marginBottom: "0.2rem",
    },
    // Row-header disclosure button. Looks like the previous inline
    // header (badge + label) but is now a single clickable target
    // that toggles the row's textarea. `flex: 1` so it stretches and
    // remains the click target across the full header width; the
    // `×` remove button sits to its right.
    disclosure: {
        flex: 1,
        display: "flex",
        alignItems: "center",
        gap: "0.5rem",
        background: "transparent",
        border: 0,
        padding: 0,
        margin: 0,
        cursor: "pointer",
        textAlign: "left",
        font: "inherit",
        color: "inherit",
    },
    chevron: {
        display: "inline-block",
        width: "0.9rem",
        fontSize: "0.8rem",
        color: "#666",
    },
    charCount: {
        fontSize: "0.75rem",
        color: "#666",
        fontFamily: "ui-monospace, Menlo, Consolas, monospace",
        marginLeft: "auto",
    },
    errorBadge: {
        fontSize: "0.7rem",
        color: "white",
        background: "crimson",
        padding: "0.05rem 0.35rem",
        borderRadius: "0.2rem",
        marginLeft: "0.4rem",
    },
}
