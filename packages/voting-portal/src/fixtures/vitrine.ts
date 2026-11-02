// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Vitrine objects for tests: keys from fixed seeds, a signed policy, the
// `events` and `ballots` logs with cosigned checkpoints, receipts, promises,
// chains and the voter context Harvest serves. Everything is encoded with
// @vitrine/verify's own formatters, so the portal checks real objects.

import {Buffer} from "buffer"
import type {BallotStage} from "@vitrine/verify/pure"
import {createPrivateKey, createPublicKey, sign as signEd25519} from "crypto"
import {
    bytes,
    encoding,
    entries,
    formatChain,
    hashes,
    merkle,
    notes,
    promises,
    proofs,
    type ReceiptChain,
} from "@vitrine/verify/pure"
import {
    ETransparencyMode,
    IVitrineVoterContext,
    VOTING_PORTAL_PACKAGE,
} from "../services/vitrine/types"

const PKCS8_ED25519_PREFIX = "302e020100300506032b657004220420"
const EVENTS_TAG = "vitrine/election/events/v1"
const MEANING_HASH = "sha256:b77e087d44cf4621cb2ca2cdf982abb5c46f3b964ce79f9ff2b04ef645624ab1"
const EVENTS_MEANING_HASH =
    "sha256:a76f20437d3846a9cb930a379fe545c218e9e80f35c186fff6dd79e870b5ba51"

export const NOW = 1_800_000_000
export const EVENT_ID = "0b6c4f53-8c0b-4cbb-9b7a-6c8a3a5c2f10"
export const ELECTION_ID = "4f1d2e3c-5b6a-4789-8a9b-0c1d2e3f4a5b"
export const RELEASE = "10.4.0"
export const BALLOT_ID = "c0ffee".repeat(10) + "abcd"
export const ACCEPTED_AT = NOW - 5
/** When voting opened: the milestone states its entry's time. */
export const OPENED_AT = NOW - 150

export interface ITestKey {
    name: string
    type: number
    key: Uint8Array
    keyId: number
    vkey: string
    sign(message: Uint8Array): Uint8Array
}

export const testKey = (name: string, seed: number, type: number = notes.SIG_ED25519): ITestKey => {
    const der = Buffer.concat([Buffer.from(PKCS8_ED25519_PREFIX, "hex"), Buffer.alloc(32, seed)])
    const privateKey = createPrivateKey({key: der, format: "der", type: "pkcs8"})
    const spki = createPublicKey(privateKey).export({format: "der", type: "spki"})
    const key = new Uint8Array(spki.subarray(spki.length - 32))
    return {
        name,
        type,
        key,
        keyId: notes.keyId(name, type, key),
        vkey: notes.formatVkey(name, type, key),
        sign: (message) => new Uint8Array(signEd25519(null, Buffer.from(message), privateKey)),
    }
}

/** `text` as a note signed by each of `keys`. */
export const signNote = (text: string, keys: readonly ITestKey[]): string =>
    notes.formatNote(
        text,
        keys.map((k) => ({name: k.name, keyId: k.keyId, sig: k.sign(bytes.utf8(text))}))
    )

const cosignature = (witness: ITestKey, time: number, checkpointText: string) => ({
    name: witness.name,
    keyId: witness.keyId,
    sig: bytes.concat(
        bytes.u64be(time),
        witness.sign(notes.cosignatureMessage(BigInt(time), checkpointText))
    ),
})

const PRIVATE_PSEUDO = encoding.hexEncode(
    hashes.sha256("vitrine/origin/v1", "logs.acme.example/e/ballots", new Uint8Array(32).fill(0x5a))
)

export const KEYS = {
    board: testKey("board.acme.example", 1),
    stranger: testKey("stranger.example", 2),
    ballots: testKey("logs.acme.example/e/ballots", 3),
    events: testKey("logs.acme.example/e/events", 4),
    witnesses: [
        testKey("witness-1.example", 5, notes.SIG_COSIGNATURE_V1),
        testKey("witness-2.example", 6, notes.SIG_COSIGNATURE_V1),
        testKey("witness-3.example", 7, notes.SIG_COSIGNATURE_V1),
    ],
    blinded: testKey(PRIVATE_PSEUDO, 17),
    blindedEvents: testKey(
        encoding.hexEncode(
            hashes.sha256(
                "vitrine/origin/v1",
                "logs.acme.example/e/events",
                new Uint8Array(32).fill(0x5a)
            )
        ),
        18
    ),
    monitor: testKey("monitor.example", 8),
    monitor2: testKey("monitor2.example", 12),
    hub1: testKey("hub1.example", 13),
    hub2: testKey("hub2.example", 14),
    hubWitness1: testKey("hub-witness1.example", 15, notes.SIG_COSIGNATURE_V1),
    hubWitness2: testKey("hub-witness2.example", 16, notes.SIG_COSIGNATURE_V1),
    auditor: testKey("auditor.example", 9),
    // The logs' log-time keys: cosigner keys named by each log's origin.
    ballotsTime: testKey("logs.acme.example/e/ballots", 10, notes.SIG_COSIGNATURE_V1),
    eventsTime: testKey("logs.acme.example/e/events", 11, notes.SIG_COSIGNATURE_V1),
}

export const LOG_URLS = {
    ballots: "https://logs.acme.example/e/ballots/",
    events: "https://logs.acme.example/e/events/",
}
export const MONITOR_URL = "https://monitor.example/"
export const MIRROR_URL = "https://mirror.example/ballots/"

// -- Merkle trees (RFC 9162, section 2.1) ------------------------------------

const split = (n: number): number => {
    let k = 1
    while (k * 2 < n) k *= 2
    return k
}

const inclusionPath = (leaves: Uint8Array[], index: number): Uint8Array[] => {
    if (leaves.length <= 1) return []
    const k = split(leaves.length)
    return index < k
        ? [...inclusionPath(leaves.slice(0, k), index), merkle.treeRoot(leaves.slice(k))]
        : [...inclusionPath(leaves.slice(k), index - k), merkle.treeRoot(leaves.slice(0, k))]
}

const subproof = (leaves: Uint8Array[], m: number, whole: boolean): Uint8Array[] => {
    const n = leaves.length
    if (m === n) return whole ? [] : [merkle.treeRoot(leaves)]
    const k = split(n)
    return m <= k
        ? [...subproof(leaves.slice(0, k), m, whole), merkle.treeRoot(leaves.slice(k))]
        : [...subproof(leaves.slice(k), m - k, false), merkle.treeRoot(leaves.slice(0, k))]
}

/** A witness and the time it cosigns a checkpoint. */
export interface ITestCosigner {
    key: ITestKey
    time: number
}

export class TestLog {
    readonly entries: Uint8Array[] = []

    constructor(
        readonly name: string,
        readonly key: ITestKey,
        readonly logTime: ITestKey,
        readonly url: string
    ) {}

    get origin(): string {
        return this.key.name
    }

    add(entry: Uint8Array): number {
        this.entries.push(entry)
        return this.entries.length - 1
    }

    leaves(size: number): Uint8Array[] {
        return this.entries.slice(0, size).map((e) => hashes.leafHash(e))
    }

    checkpointText(size: number): string {
        return `${this.origin}\n${size}\n${encoding.b64encode(merkle.treeRoot(this.leaves(size)))}\n`
    }

    /** The checkpoint at `size`, signed by the log, dated by its log-time key
     * and cosigned at `time`, or by each witness at its own time. */
    checkpoint(
        size: number,
        witnesses: readonly (ITestKey | ITestCosigner)[],
        time: number = NOW - 10
    ): string {
        const text = this.checkpointText(size)
        const logSig = {
            name: this.key.name,
            keyId: this.key.keyId,
            sig: this.key.sign(bytes.utf8(text)),
        }
        return notes.formatNote(text, [
            logSig,
            cosignature(this.logTime, time, text),
            ...witnesses.map((w) =>
                "time" in w ? cosignature(w.key, w.time, text) : cosignature(w, time, text)
            ),
        ])
    }

    inclusion(index: number, size: number): Uint8Array[] {
        return inclusionPath(this.leaves(size), index)
    }

    consistency(from: number, to: number): Uint8Array[] {
        return subproof(this.leaves(to), from, true)
    }

    /** A receipt: the entry at `index` against the checkpoint at `size`. */
    receipt(
        index: number,
        size: number,
        witnesses: readonly (ITestKey | ITestCosigner)[],
        time: number = NOW - 10
    ): string {
        return proofs.formatTlogProof({
            extra: this.entries[index],
            index: BigInt(index),
            proof: this.inclusion(index, size),
            checkpoint: this.checkpoint(size, witnesses, time),
        })
    }

    /** A promise for the entry at `index`, in a batch of the entries at
     * `batch` (in ascending order of their leaf hashes), under `policyId`. */
    promise(
        index: number,
        batch: readonly number[],
        policyId: string,
        time: number = NOW - 2,
        mmd = 60
    ): string {
        const leaf = hashes.leafHash(this.entries[index])
        const batchLeaves = batch.map((i) => hashes.leafHash(hashes.leafHash(this.entries[i])))
        const text = promises.formatPromiseBatchText({
            origin: this.origin,
            policy: policyId,
            time,
            mmd,
            size: batch.length,
            root: merkle.treeRoot(batchLeaves),
        })
        const at = batch.indexOf(index)
        return promises.formatPromise(
            leaf,
            at,
            inclusionPath(batchLeaves, at),
            signNote(text, [this.key])
        )
    }
}

export const hashLines = (proof: readonly Uint8Array[]): string =>
    proof.map((h) => `${encoding.b64encode(h)}\n`).join("")

export const chainText = (
    policyId: string,
    log: string,
    blocks: Partial<Omit<ReceiptChain, "policyId" | "log">>
): string => bytes.fromUtf8(formatChain({policyId, log, hubs: new Map(), unknown: [], ...blocks}))

// -- The policy ------------------------------------------------------------

export type TVisibility = ETransparencyMode.PUBLIC | ETransparencyMode.PRIVATE

/** The policy's lines without the seal: the draft the board seals. */
const policyLines = (visibility: TVisibility, extra: readonly string[]): string[] => [
    "vitrine/policy/v1",
    "name acme-2026",
    `visibility ${visibility}`,
    ...(visibility === ETransparencyMode.PRIVATE ? [`policy-salt ${"5a".repeat(32)}`] : []),
    "operator sequent",
    `authority board ${KEYS.board.vkey}`,
    "sign 1 board",
    ...(visibility === ETransparencyMode.PRIVATE ? [`auditor auditors ${KEYS.auditor.vkey}`] : []),
    "mmd 60",
    ...(!extra.some((line) => line.startsWith("required ")) ? ["required on"] : []),
    `log ballots ${KEYS.ballots.vkey} ${LOG_URLS.ballots}`,
    `log-time ballots ${KEYS.ballotsTime.vkey}`,
    `mirror ballots ${MIRROR_URL}`,
    `mirror-party ballots ${MIRROR_URL} mirror-co`,
    `profile ballots vitrine/election/ballots/v1 ${MEANING_HASH}`,
    ...(visibility === ETransparencyMode.PRIVATE
        ? [`blinded ballots ${PRIVATE_PSEUDO} ${KEYS.blinded.vkey}`]
        : []),
    ...(visibility === ETransparencyMode.PRIVATE && !extra.some((line) => line.startsWith("opens "))
        ? [`opens ${NOW - 3600}`]
        : []),
    ...(visibility === ETransparencyMode.PRIVATE &&
    !extra.some((line) => line.startsWith("closes "))
        ? [`closes ${NOW + 3600}`]
        : []),
    `log events ${KEYS.events.vkey} ${LOG_URLS.events}`,
    `log-time events ${KEYS.eventsTime.vkey}`,
    `profile events ${EVENTS_TAG} ${EVENTS_MEANING_HASH}`,
    ...(visibility === ETransparencyMode.PRIVATE
        ? [`blinded events ${KEYS.blindedEvents.vkey.split("+")[0]} ${KEYS.blindedEvents.vkey}`]
        : []),
    ...KEYS.witnesses.map((w, i) => `witness w${i + 1} ${w.vkey}`),
    "group witnesses 2 w1 w2 w3",
    "quorum witnesses",
    `monitor observers ${KEYS.monitor.vkey} ${MONITOR_URL}`,
    `hub h1 ${KEYS.hub1.vkey} https://hub1.example/`,
    "hub-operator h1 hubco1",
    `hub-witness h1 w ${KEYS.hubWitness1.vkey}`,
    "hub-witness-party h1 w hub-witness-party1",
    "hub-quorum h1 w",
    `hub h2 ${KEYS.hub2.vkey} https://hub2.example/`,
    "hub-operator h2 hubco2",
    `hub-witness h2 w ${KEYS.hubWitness2.vkey}`,
    "hub-witness-party h2 w hub-witness-party2",
    "hub-quorum h2 w",
    `monitor observers2 ${KEYS.monitor2.vkey} https://monitor2.example/`,
    "monitor-party observers monitorco1",
    "monitor-party observers2 monitorco2",
    "monitor-quorum 2",
    ...extra,
]

/** The board's draft: the policy lines without the seal, unsigned. */
export const draftPolicyText = (visibility: TVisibility, extra: readonly string[] = []): string =>
    `${policyLines(visibility, extra).join("\n")}\n`

/** The sealed policy, signed by the board, which seals it. */
export const policyText = (visibility: TVisibility, extra: readonly string[] = []): string => {
    const lines = policyLines(visibility, extra)
    const seal = encoding.b64encode(hashes.sha256(`seal ${lines.join(" ")}`).slice(0, 16))
    return signNote(`${[...lines, `seal board ${seal}`].join("\n")}\n`, [KEYS.board])
}

/** The policy ID of a signed policy note: SHA-256 of its text. */
export const policyIdOf = (note: string): string => {
    const text = notes.parseNote(note).textBytes
    return `sha256:${encoding.hexEncode(hashes.sha256(text))}`
}

// -- Entries ------------------------------------------------------------------

const pub = (value: Uint8Array): entries.Field => ({kind: entries.FIELD_PUBLIC, value})

export const eventsEntry = (
    kind: string,
    statement: string,
    time: number = NOW - 100
): Uint8Array =>
    entries.encodeEntry(EVENTS_TAG, [
        pub(bytes.utf8(kind)),
        pub(bytes.u64be(time)),
        pub(Uint8Array.of(0)),
        pub(bytes.utf8(statement)),
        {kind: entries.FIELD_COMMITMENT, value: hashes.sha256(`details ${kind} ${time}`)},
    ])

export const ballotEntry = (
    ballotId: string = BALLOT_ID,
    acceptedAt: number = ACCEPTED_AT,
    stage: BallotStage = "cast"
) =>
    entries.buildElectionEntry({
        eventId: EVENT_ID,
        electionId: ELECTION_ID,
        stage,
        ballotId,
        acceptedAt,
    })

// -- The booth's inputs ---------------------------------------------------------

/** The election public key as step's ballot styles carry it: base64, no padding. */
export const PUBLIC_KEY = encoding.b64encode(new Uint8Array(32).fill(42)).replace(/=+$/, "")
export const STYLE_TEXT = JSON.stringify({id: "style-north", public_key: {public_key: PUBLIC_KEY}})
const OTHER_STYLE_TEXT = JSON.stringify({id: "style-south", public_key: {public_key: PUBLIC_KEY}})

const sha256Hex = (text: string): string => encoding.hexEncode(hashes.sha256(text))

export const styleHashes = (): string[] =>
    [sha256Hex(STYLE_TEXT), sha256Hex(OTHER_STYLE_TEXT)].sort()

const stylesLine = (hexDigests: readonly string[]): string =>
    encoding.b64encode(
        hashes.sha256(
            ...hexDigests
                .map((h) => encoding.hexDecode(h, 32))
                .sort((a, b) => bytes.compareBytes(a, b))
        )
    )

export interface IConfigFields {
    event: string
    election: string
    key: string
    styles: string
    opens: number
    closes: number
    policy: string
    releases: string[]
    transparency: string
    demo: string
    time: number
}

export const configStatementText = (f: IConfigFields): string =>
    [
        "vitrine/config/v1",
        `event ${f.event}`,
        `election ${f.election}`,
        `key ${f.key}`,
        `styles ${f.styles}`,
        `opens ${f.opens}`,
        `closes ${f.closes}`,
        `policy ${f.policy}`,
        ...f.releases.map((r) => `release ${r}`),
        `transparency ${f.transparency}`,
        `demo ${f.demo}`,
        `time ${f.time}`,
    ].join("\n") + "\n"

export interface IScenarioOptions {
    mode?: TVisibility
    config?: Partial<IConfigFields>
    /** Who signs the config statement. */
    configSigners?: readonly ITestKey[]
    /** How many witnesses cosign the events checkpoint. */
    cosigners?: number
    /** The milestone's `config` line for the election, base64. */
    openedConfigLeaf?: string
    /** The milestone's kind. */
    openedKind?: string
    /** More policy lines. */
    policyExtra?: readonly string[]
}

export interface IScenario {
    mode: TVisibility
    policy: string
    policyId: string
    presentation: Record<string, unknown>
    context: IVitrineVoterContext
    events: TestLog
    ballots: TestLog
    configEntry: Uint8Array
    openedEntry: Uint8Array
    policyAck: string
}

/** An event whose booth check passes, unless the options break it. */
export const scenario = (options: IScenarioOptions = {}): IScenario => {
    const mode = options.mode ?? ETransparencyMode.PUBLIC
    const policy = policyText(mode, options.policyExtra)
    const policyId = policyIdOf(policy)
    const witnesses = KEYS.witnesses.slice(0, options.cosigners ?? 2)
    const hashesList = styleHashes()
    const fields: IConfigFields = {
        event: EVENT_ID,
        election: ELECTION_ID,
        key: encoding.b64encode(hashes.sha256(encoding.b64decode(`${PUBLIC_KEY}=`))),
        styles: stylesLine(hashesList),
        opens: NOW - 3600,
        closes: NOW + 3600,
        policy: policyId,
        releases: [`${VOTING_PORTAL_PACKAGE} ${RELEASE}`, "sequent/ballot-verifier 10.4.0"],
        transparency: mode,
        demo: "no",
        time: NOW - 200,
        ...options.config,
    }
    const config = signNote(configStatementText(fields), options.configSigners ?? [KEYS.board])
    const events = new TestLog("events", KEYS.events, KEYS.eventsTime, LOG_URLS.events)
    events.add(eventsEntry("notice", signNote("vitrine/notice/v1\nhello\n", [KEYS.board])))
    const configIndex = events.add(eventsEntry("config", config))
    const configEntry = events.entries[configIndex]
    const leaf = encoding.b64encode(hashes.leafHash(configEntry))
    const opened = signNote(
        [
            "vitrine/milestone/v1",
            `event ${EVENT_ID}`,
            `kind ${options.openedKind ?? "voting-opened"}`,
            `time ${OPENED_AT}`,
            `config ${ELECTION_ID} ${options.openedConfigLeaf ?? leaf}`,
        ].join("\n") + "\n",
        [KEYS.board]
    )
    const openedIndex = events.add(eventsEntry("voting-opened", opened, OPENED_AT))
    const openedEntry = events.entries[openedIndex]
    const size = events.entries.length
    const chainFor = (index: number) =>
        chainText(policyId, "events", {record: bytes.utf8(events.receipt(index, size, witnesses))})
    const policyAck = signNote(
        `vitrine/policy-ack/v1\norigin ${KEYS.ballots.name}\npolicy ${policyId}\ntime ${NOW - 500}\n`,
        [KEYS.ballots]
    )
    const ballots = new TestLog("ballots", KEYS.ballots, KEYS.ballotsTime, LOG_URLS.ballots)
    return {
        mode,
        policy,
        policyId,
        presentation: {
            transparency: mode,
            vitrine: {
                policy_id: policyId.slice("sha256:".length),
                ...(mode === ETransparencyMode.PUBLIC ? {policy} : {}),
                logs: {
                    ballots: {url: LOG_URLS.ballots, origin: KEYS.ballots.name},
                    events: {url: LOG_URLS.events, origin: KEYS.events.name},
                },
            },
        },
        context: {
            mode,
            policy: mode === ETransparencyMode.PRIVATE ? policy : null,
            policy_ack: mode === ETransparencyMode.PRIVATE ? policyAck : null,
            config: {
                statement: config,
                entry: encoding.b64encode(configEntry),
                chain: chainFor(configIndex),
            },
            opened: {
                statement: opened,
                entry: encoding.b64encode(openedEntry),
                chain: chainFor(openedIndex),
            },
            style_hashes: hashesList,
            release: `${VOTING_PORTAL_PACKAGE} ${RELEASE}`,
        },
        events,
        ballots,
        configEntry,
        openedEntry,
        policyAck,
    }
}
