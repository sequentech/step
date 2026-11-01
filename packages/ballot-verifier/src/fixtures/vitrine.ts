// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Vitrine objects for the receipt screen's tests: keys from fixed seeds,
// signed trust policies, a `ballots` log with cosigned checkpoints, its
// receipts and receipt chains, the links the voting portal opens, and a fake
// network. Everything is encoded with @vitrine/verify's own formatters, so
// the screen checks real objects.

import {Buffer} from "buffer"
import {createPrivateKey, createPublicKey, sign as signEd25519} from "crypto"
import {
    bytes,
    encoding,
    entries,
    formatChain,
    formatReceiptReference,
    hashes,
    merkle,
    notes,
    proofs,
    trackingId,
    type ReceiptChain,
} from "@vitrine/verify/pure"
import type {THttpInit} from "../services/vitrine/http"
import {ETransparencyMode} from "../services/vitrine/presentation"

const PKCS8_ED25519_PREFIX = "302e020100300506032b657004220420"
const EVENTS_TAG = "vitrine/election/events/v1"
const MEANING_HASH = "sha256:b77e087d44cf4621cb2ca2cdf982abb5c46f3b964ce79f9ff2b04ef645624ab1"
const EVENTS_MEANING_HASH =
    "sha256:a76f20437d3846a9cb930a379fe545c218e9e80f35c186fff6dd79e870b5ba51"

export const NOW = 1_800_000_000
export const EVENT_ID = "0b6c4f53-8c0b-4cbb-9b7a-6c8a3a5c2f10"
export const ELECTION_ID = "4f1d2e3c-5b6a-4789-8a9b-0c1d2e3f4a5b"
export const OTHER_ELECTION_ID = "9a8b7c6d-5e4f-4a3b-9c2d-1e0f9a8b7c6d"
export const BALLOT_ID = "c0ffee".repeat(10) + "abcd"
export const OTHER_BALLOT_ID = "0dd".repeat(21) + "0"
/** The Tracking IDs the digests above write. */
export const TRACKING_ID = trackingId(BALLOT_ID)
export const OTHER_TRACKING_ID = trackingId(OTHER_BALLOT_ID)
export const ACCEPTED_AT = NOW - 5
export const VERIFIER_URL = "https://verifier.example/receipt/"

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

/** The witnesses that cosign the test checkpoints: the policy's quorum. */
export const WITNESSES = KEYS.witnesses.slice(0, 2)

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

export class TestLog {
    readonly entries: Uint8Array[] = []

    constructor(
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
     * and cosigned at `time`. */
    checkpoint(size: number, witnesses: readonly ITestKey[], time: number = NOW - 10): string {
        const text = this.checkpointText(size)
        const logSig = {
            name: this.key.name,
            keyId: this.key.keyId,
            sig: this.key.sign(bytes.utf8(text)),
        }
        return notes.formatNote(text, [
            logSig,
            cosignature(this.logTime, time, text),
            ...witnesses.map((w) => cosignature(w, time, text)),
        ])
    }

    /** A receipt: the entry at `index` against the checkpoint at `size`. */
    receipt(
        index: number,
        size: number,
        witnesses: readonly ITestKey[],
        time: number = NOW - 10
    ): string {
        return proofs.formatTlogProof({
            extra: this.entries[index],
            index: BigInt(index),
            proof: inclusionPath(this.leaves(size), index),
            checkpoint: this.checkpoint(size, witnesses, time),
        })
    }
}

export const chainText = (
    policyId: string,
    log: string,
    blocks: Partial<Omit<ReceiptChain, "policyId" | "log">>
): string => bytes.fromUtf8(formatChain({policyId, log, hubs: new Map(), unknown: [], ...blocks}))

// -- Trust policies -----------------------------------------------------------

export type TVisibility = ETransparencyMode.PUBLIC | ETransparencyMode.PRIVATE

export interface IPolicyOptions {
    visibility?: TVisibility
    /** The policy's `name`: the event, as the receipt screen shows it. */
    name?: string
    /** The elections whose ballots the `ballots` log holds. */
    electionIds?: readonly string[]
    /** Leave out the profile parameters that name the event and elections. */
    noParams?: boolean
}

/** The policy's lines without the seal: the draft the board seals. */
const policyLines = (o: IPolicyOptions): string[] => {
    const visibility = o.visibility ?? ETransparencyMode.PUBLIC
    const isPrivate = visibility === ETransparencyMode.PRIVATE
    const electionIds = o.electionIds ?? [ELECTION_ID, OTHER_ELECTION_ID]
    return [
        "vitrine/policy/v1",
        `name ${o.name ?? "acme-2026"}`,
        `visibility ${visibility}`,
        ...(isPrivate ? [`policy-salt ${"5a".repeat(32)}`] : []),
        "operator sequent",
        `authority board ${KEYS.board.vkey}`,
        "sign 1 board",
        ...(isPrivate ? [`auditor auditors ${KEYS.auditor.vkey}`] : []),
        "mmd 60",
        "required on",
        `log ballots ${KEYS.ballots.vkey} ${LOG_URLS.ballots}`,
        `log-time ballots ${KEYS.ballotsTime.vkey}`,
        `mirror ballots ${MIRROR_URL}`,
        `mirror-party ballots ${MIRROR_URL} mirror-co`,
        `profile ballots vitrine/election/ballots/v1 ${MEANING_HASH}`,
        ...(visibility === ETransparencyMode.PRIVATE
            ? [`blinded ballots ${PRIVATE_PSEUDO} ${KEYS.blinded.vkey}`]
            : []),
        ...(visibility === ETransparencyMode.PRIVATE
            ? [`opens ${NOW - 3600}`, `closes ${NOW + 3600}`]
            : []),
        ...(o.noParams
            ? []
            : [
                  `param ballots event-id ${EVENT_ID}`,
                  `param ballots election-ids ${electionIds.join(" ")}`,
              ]),
        `log events ${KEYS.events.vkey} ${LOG_URLS.events}`,
        `log-time events ${KEYS.eventsTime.vkey}`,
        `profile events ${EVENTS_TAG} ${EVENTS_MEANING_HASH}`,
        ...(visibility === ETransparencyMode.PRIVATE
            ? [`blinded events ${KEYS.blindedEvents.vkey.split("+")[0]} ${KEYS.blindedEvents.vkey}`]
            : []),
        ...KEYS.witnesses.map((w, i) => `witness w${i + 1} ${w.vkey}`),
        "witness-party w2 observers",
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
    ]
}

/** The board's draft: the policy lines without the seal, unsigned. */
export const draftPolicyText = (o: IPolicyOptions = {}): string => `${policyLines(o).join("\n")}\n`

/** The sealed policy, signed by the board, which seals it. */
export const policyText = (o: IPolicyOptions = {}): string => {
    const lines = policyLines(o)
    const seal = encoding.b64encode(hashes.sha256(`seal ${lines.join(" ")}`).slice(0, 16))
    return signNote(`${[...lines, `seal board ${seal}`].join("\n")}\n`, [KEYS.board])
}

/** The policy ID of a signed policy note: SHA-256 of its text. */
export const policyIdOf = (note: string): string => {
    const text = notes.parseNote(note).textBytes
    return `sha256:${encoding.hexEncode(hashes.sha256(text))}`
}

// -- Receipts -----------------------------------------------------------------

export const ballotEntry = (
    ballotId: string = BALLOT_ID,
    electionId: string = ELECTION_ID,
    acceptedAt: number = ACCEPTED_AT,
    stage: "received" | "cast" | "audited" = "cast"
): Uint8Array =>
    entries.buildElectionEntry({eventId: EVENT_ID, electionId, stage, ballotId, acceptedAt})

export interface IReceiptScenario {
    /** The signed policy note. */
    policy: string
    policyId: string
    ballots: TestLog
    ballotId: string
    /** The voter's entry, and its index in the log. */
    entry: Uint8Array
    index: number
    /** The receipt the log issued at cast. */
    receipt: string
    /** The receipt chain holding it. */
    chain: string
}

/** A `ballots` log with the voter's entry and another one, and the voter's
 * receipt chain. */
export const receiptScenario = (o: IPolicyOptions = {}): IReceiptScenario => {
    const policy = policyText(o)
    const policyId = policyIdOf(policy)
    const ballots = new TestLog(KEYS.ballots, KEYS.ballotsTime, LOG_URLS.ballots)
    const entry = ballotEntry()
    const index = ballots.add(entry)
    ballots.add(ballotEntry(OTHER_BALLOT_ID))
    const receipt = ballots.receipt(index, 2, WITNESSES)
    const chain = chainText(policyId, "ballots", {issued: bytes.utf8(receipt)})
    return {policy, policyId, ballots, ballotId: BALLOT_ID, entry, index, receipt, chain}
}

/** Where the voting portal's link says a public event's policy is. */
export const policyUrlOf = (s: IReceiptScenario): string =>
    `${LOG_URLS.ballots}policy/${s.policyId}`

export type TLinkPolicy = {url: string} | {text: string}

/** The fragment the voting portal's "Check independently" link opens. */
export const chainLink = (chain: string, entry?: Uint8Array, policy?: TLinkPolicy): string => {
    const params = [`c=${encoding.b64urlEncode(bytes.utf8(chain))}`]
    if (entry) params.push(`e=${encoding.b64urlEncode(entry)}`)
    if (policy && "url" in policy) params.push(`u=${encodeURIComponent(policy.url)}`)
    if (policy && "text" in policy) {
        params.push(`policy=${encoding.b64urlEncode(bytes.utf8(policy.text))}`)
    }
    return `#vitrine:chain?${params.join("&")}`
}

/** The receipt's reference, as its QR code holds it. */
export const referenceOf = (s: IReceiptScenario): string =>
    formatReceiptReference(VERIFIER_URL, s.policyId, "ballots", s.index, s.entry)

/** Where the log serves the receipt a reference names. */
export const referenceReceiptUrl = (s: IReceiptScenario): string =>
    `${LOG_URLS.ballots}receipt/${encoding.hexEncode(hashes.leafHash(s.entry))}?index=${s.index}`

// -- Audited ballots ------------------------------------------------------------

/** When the ballot box took the voter's audit. */
export const AUDITED_AT = ACCEPTED_AT + 2
/** The ballots log's own server, which lists its buckets. */
export const LOG_SERVER = "https://logs.acme.example"

export interface IAuditedScenario {
    policy: string
    policyId: string
    ballots: TestLog
    /** The `audited` entry, and its index in the log. */
    entry: Uint8Array
    index: number
    /** The log's receipt for it. */
    receipt: string
}

/** A `ballots` log where the voter's ballot was received, then audited (or
 * cast, with `stage`), with another ballot cast between them. */
export const auditedScenario = (
    o: IPolicyOptions = {},
    stage: "cast" | "audited" = "audited"
): IAuditedScenario => {
    const policy = policyText(o)
    const ballots = new TestLog(KEYS.ballots, KEYS.ballotsTime, LOG_URLS.ballots)
    ballots.add(ballotEntry(BALLOT_ID, ELECTION_ID, ACCEPTED_AT, "received"))
    ballots.add(ballotEntry(OTHER_BALLOT_ID))
    const entry = ballotEntry(BALLOT_ID, ELECTION_ID, AUDITED_AT, stage)
    const index = ballots.add(entry)
    return {
        policy,
        policyId: policyIdOf(policy),
        ballots,
        entry,
        index,
        receipt: ballots.receipt(index, ballots.entries.length, WITNESSES),
    }
}

/** The bucket a Tracking ID is in: its first 3 characters. */
export const bucketOf = (id: string = TRACKING_ID): string => id.replace(/-/g, "").slice(0, 3)

/** Where `server` lists the bucket of a Tracking ID of the ballots log. */
export const bucketUrlOf = (
    server: string = LOG_SERVER,
    log: string = "e/ballots",
    id: string = TRACKING_ID
): string => `${server}/lookup/${log}/tracking-bucket/${bucketOf(id)}`

/** A bucket answer listing every entry of the log; the lookup keeps those
 * of the bucket. */
export const bucketAnswer = (log: TestLog, extra: {truncated?: boolean} = {}): string =>
    JSON.stringify({
        lookup: "tracking-bucket",
        entries: log.entries.map((entry, index) => ({index, entry: encoding.b64encode(entry)})),
        size: log.entries.length,
        ...extra,
    })

/** Where the log serves the receipt of the entry at `index`. */
export const receiptUrlOf = (entry: Uint8Array, index: number): string =>
    `${LOG_URLS.ballots}receipt/${encoding.hexEncode(hashes.leafHash(entry))}?index=${index}`

// -- The network ----------------------------------------------------------------

export interface IFakeCall {
    url: string
    method: string
    init?: THttpInit
}

export interface IFakeAnswer {
    status: number
    body?: string
}

/** A response as the screen and @vitrine/verify read it. */
export const fakeResponse = (a: IFakeAnswer): Response =>
    ({
        status: a.status,
        headers: {get: () => null},
        body: null,
        arrayBuffer: async () => bytes.utf8(a.body ?? ""),
    }) as unknown as Response

/** The network: what was served answers, POSTs get 202 and the rest 404. */
export class FakeNet {
    readonly calls: IFakeCall[] = []
    private readonly answers = new Map<string, IFakeAnswer>()

    serve(url: string, body: string, status = 200, method = "GET"): void {
        this.answers.set(`${method} ${url}`, {status, body})
    }

    /** Forgets what `url` was served with, which then answers 404. */
    forget(url: string, method = "GET"): void {
        this.answers.delete(`${method} ${url}`)
    }

    readonly fetch = async (input: string, init?: THttpInit): Promise<Response> => {
        const method = init?.method ?? "GET"
        this.calls.push({url: String(input), method, ...(init ? {init} : {})})
        const answer = this.answers.get(`${method} ${input}`)
        return fakeResponse(answer ?? {status: method === "POST" ? 202 : 404})
    }

    /** The requests from the `from`th on, as `METHOD url`. */
    requests(from = 0): string[] {
        return this.calls.slice(from).map((c) => `${c.method} ${c.url}`)
    }
}
