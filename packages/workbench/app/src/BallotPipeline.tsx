// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useState, type CSSProperties} from "react"
import {
    decodeBigIntToDecodedVoteContest,
    decryptBallotContent,
    encodeBallot,
    encryptDecodedVoteContest,
    generateKeypair,
    getFixtures,
    runTally,
} from "./tally"

// BallotPipeline — single-contest playground that walks a selection
// through every transformation a ballot undergoes on its way to the
// tally:
//
//   plaintext  ──encode──▶  encoded BigUint  ──encrypt──▶  ciphertext
//                                                              │
//                                                          decrypt
//                                                              ▼
//   decoded plaintext  ◀──decode──  decrypted BigUint (=encoded)
//                │
//              tally
//                ▼
//              result
//
// Each stage is a textarea + button. Buttons read the textarea above
// (and the Setup inputs) and write into the textarea below; operators
// can edit any intermediate stage and rerun downstream steps to probe
// failures. This is intentionally per-contest: the encrypted envelope
// produced here only contains one contest, which is enough to round-trip
// through `decrypt_ballot_content`.
//
// The contest is fixed across the pipeline (Setup); the keypair is also
// pipeline-wide so encrypt and decrypt agree.
export function BallotPipeline() {
    const [contestJson, setContestJson] = useState<string>("")
    const [pkB64, setPkB64] = useState<string>("")
    const [skB64, setSkB64] = useState<string>("")

    const [plaintextJson, setPlaintextJson] = useState<string>("")
    const [encodedBigInt, setEncodedBigInt] = useState<string>("")
    const [encryptedJson, setEncryptedJson] = useState<string>("")
    const [decryptedBigInt, setDecryptedBigInt] = useState<string>("")
    const [decodedJson, setDecodedJson] = useState<string>("")
    const [tallyBallots, setTallyBallots] = useState<string>("")
    const [tallyResult, setTallyResult] = useState<unknown | null>(null)

    const [busy, setBusy] = useState<string | null>(null)
    const [error, setError] = useState<string | null>(null)

    useEffect(() => {
        ;(async () => {
            try {
                const fixtures = await getFixtures()
                setContestJson(fixtures.contestJson)
                setPlaintextJson(fixtures.decodedVoteContestJson)
                setTallyBallots(fixtures.ballotsJson)
                const kp = await generateKeypair()
                setPkB64(kp.pkB64)
                setSkB64(kp.skB64)
            } catch (e) {
                setError(formatError(e))
            }
        })()
    }, [])

    const wrap =
        (label: string, fn: () => Promise<void>) =>
        async () => {
            setBusy(label)
            setError(null)
            try {
                await fn()
            } catch (e) {
                setError(formatError(e))
            } finally {
                setBusy(null)
            }
        }

    const handleNewKeypair = wrap("keypair", async () => {
        const kp = await generateKeypair()
        setPkB64(kp.pkB64)
        setSkB64(kp.skB64)
    })

    const handleEncode = wrap("encode", async () => {
        const bigint = await encodeBallot(contestJson, plaintextJson)
        setEncodedBigInt(bigint)
    })

    const handleEncrypt = wrap("encrypt", async () => {
        if (!pkB64) throw new Error("public key is empty")
        // Encrypt path needs the *structured* plaintext (it does its
        // own encode internally so it can produce the proper [u8;30]
        // plaintext element for ElGamal). We feed the same plaintext
        // textarea the encode step consumed.
        const envelope = await encryptDecodedVoteContest(
            contestJson,
            plaintextJson,
            pkB64
        )
        setEncryptedJson(prettyJson(envelope))
    })

    const handleDecrypt = wrap("decrypt", async () => {
        if (!skB64) throw new Error("secret key is empty")
        const contestId = readContestId(contestJson)
        const bigint = await decryptBallotContent(
            encryptedJson,
            skB64,
            contestId
        )
        setDecryptedBigInt(bigint)
    })

    const handleDecode = wrap("decode", async () => {
        const decoded = await decodeBigIntToDecodedVoteContest(
            contestJson,
            decryptedBigInt.trim()
        )
        setDecodedJson(prettyJson(decoded))
    })

    const handleSeedTallyFromDecoded = wrap("seed-tally", async () => {
        // The tally function consumes encoded BigUint strings, not
        // decoded selections — so seed it with the decrypted BigUint
        // (which equals the encoded BigUint after round-trip).
        const list = decryptedBigInt.trim()
            ? [decryptedBigInt.trim()]
            : encodedBigInt.trim()
            ? [encodedBigInt.trim()]
            : []
        if (list.length === 0) {
            throw new Error(
                "no encoded ballot to seed tally with — run encode or decrypt first"
            )
        }
        setTallyBallots(JSON.stringify(list, null, 2))
    })

    const handleTally = wrap("tally", async () => {
        const ballots = parseBallots(tallyBallots)
        const result = await runTally(contestJson, ballots)
        setTallyResult(result)
    })

    const isBusy = busy !== null
    const busyLabel = (key: string, idle: string) =>
        busy === key ? `${idle}…` : idle

    return (
        <main style={styles.main}>
            <h1>Sequentech workbench — ballot pipeline</h1>
            <p style={styles.help}>
                Walk a single ballot selection through the full
                encode → encrypt → decrypt → decode → tally chain.
                Every intermediate value is editable; rerun any
                downstream button after tweaking.
            </p>

            <Section title="Setup">
                <Field label="Contest JSON">
                    <textarea
                        value={contestJson}
                        onChange={(e) => setContestJson(e.target.value)}
                        style={{...styles.textarea, height: "12rem"}}
                        spellCheck={false}
                    />
                </Field>
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
                    disabled={isBusy}
                    style={styles.button}
                >
                    {busyLabel("keypair", "Generate new keypair")}
                </button>
            </Section>

            <Stage
                index={1}
                title="Plaintext (DecodedVoteContest)"
                help='Structured selection: {"contest_id": "…", "choices": [{"id": "…", "selected": 0, "write_in_text": null}, …], …}. selected = -1 means "not picked".'
                value={plaintextJson}
                onChange={setPlaintextJson}
                buttonLabel={busyLabel("encode", "Encode ▼")}
                onClick={handleEncode}
                disabled={isBusy}
            />

            <Stage
                index={2}
                title="Encoded plaintext (decimal BigUint)"
                value={encodedBigInt}
                onChange={setEncodedBigInt}
                buttonLabel={busyLabel("encrypt", "Encrypt ▼")}
                onClick={handleEncrypt}
                disabled={isBusy}
                help="The encrypt step actually re-runs encode internally to obtain the [u8;30] plaintext element; the BigUint above is shown so you can compare it byte-for-byte against the decrypted BigUint downstream."
                small
            />

            <Stage
                index={3}
                title="Encrypted ballot envelope (HashableBallot JSON)"
                help='{"contests": ["<base64 of HashableBallotContest>"]}. Same shape the portal stores in castVote.content, so this can be hand-pasted from a real cast vote too.'
                value={encryptedJson}
                onChange={setEncryptedJson}
                buttonLabel={busyLabel("decrypt", "Decrypt ▼")}
                onClick={handleDecrypt}
                disabled={isBusy}
            />

            <Stage
                index={4}
                title="Decrypted plaintext (= encoded BigUint)"
                help="Round-trip check: this should be byte-identical to the encoded BigUint in step 2."
                value={decryptedBigInt}
                onChange={setDecryptedBigInt}
                buttonLabel={busyLabel("decode", "Decode ▼")}
                onClick={handleDecode}
                disabled={isBusy}
                small
            />

            <Stage
                index={5}
                title="Decoded plaintext (DecodedVoteContest)"
                help="Round-trip check: this should match the plaintext in step 1 once normalisation runs."
                value={decodedJson}
                onChange={setDecodedJson}
                buttonLabel={busyLabel(
                    "seed-tally",
                    "Seed tally with this ballot ▼"
                )}
                onClick={handleSeedTallyFromDecoded}
                disabled={isBusy}
            />

            <Section title="6. Tally ballots (array of decimal BigUint strings)">
                <textarea
                    value={tallyBallots}
                    onChange={(e) => setTallyBallots(e.target.value)}
                    style={{...styles.textarea, height: "10rem"}}
                    spellCheck={false}
                />
                <button
                    onClick={handleTally}
                    disabled={isBusy}
                    style={styles.button}
                >
                    {busyLabel("tally", "Run tally")}
                </button>
                {tallyResult !== null && (
                    <pre style={styles.output}>
                        {JSON.stringify(tallyResult, null, 2)}
                    </pre>
                )}
            </Section>

            {error && (
                <Section title="Error">
                    <pre style={{...styles.output, color: "crimson"}}>
                        {error}
                    </pre>
                </Section>
            )}
        </main>
    )
}

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

function Stage(props: {
    index: number
    title: string
    help?: string
    value: string
    onChange: (v: string) => void
    buttonLabel: string
    onClick: () => void
    disabled: boolean
    small?: boolean
}) {
    return (
        <section style={styles.section}>
            <h2 style={styles.h2}>
                {props.index}. {props.title}
            </h2>
            {props.help && <p style={styles.help}>{props.help}</p>}
            <textarea
                value={props.value}
                onChange={(e) => props.onChange(e.target.value)}
                style={{
                    ...styles.textarea,
                    height: props.small ? "5rem" : "12rem",
                }}
                spellCheck={false}
            />
            <button
                onClick={props.onClick}
                disabled={props.disabled}
                style={styles.button}
            >
                {props.buttonLabel}
            </button>
        </section>
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
    label: {
        display: "block",
        fontSize: "0.8rem",
        color: "#333",
        marginBottom: "0.2rem",
    },
}
