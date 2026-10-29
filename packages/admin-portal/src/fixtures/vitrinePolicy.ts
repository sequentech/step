// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Vitrine objects for the Transparency tab's tests: keys from fixed seeds, the
// draft trust policy Harvest makes for an event, the policy the Electoral
// Board seals and signs from it, and what the sidecar reports about the
// event's logs. Policies are encoded with @vitrine/verify's own formatters,
// so the tab checks real ones.

import {Buffer} from "buffer"
import {createPrivateKey, createPublicKey, sign as signEd25519} from "crypto"
import {bytes, encoding, hashes, notes} from "@vitrine/verify/pure"

const PKCS8_ED25519_PREFIX = "302e020100300506032b657004220420"
const EVENTS_TAG = "vitrine/election/events/v1"
const MEANING_HASH = "sha256:03a5a99d740d19741de6491ee2fb77bd1680d8d934f6dc5ca90bb25c841bdba7"
const EVENTS_MEANING_HASH =
    "sha256:819b9a9da4755aaf78b18ea35390f7908d1a21a824e7c7c6874702bf7152bc4f"

export const EVENT_ID = "0b6c4f53-8c0b-4cbb-9b7a-6c8a3a5c2f10"
export const OTHER_EVENT_ID = "7e0c1d2b-3a4f-4b5c-8d6e-9f0a1b2c3d4e"
export const ELECTION_IDS = [
    "4f1d2e3c-5b6a-4789-8a9b-0c1d2e3f4a5b",
    "9a8b7c6d-5e4f-4a3b-9c2d-1e0f9a8b7c6d",
]
export const LOG_URLS = {
    ballots: "https://logs.acme.example/e/ballots/",
    events: "https://logs.acme.example/e/events/",
}
const MIRROR_URL = "https://mirror.example/ballots/"
export const NOW = 1_800_000_000

interface ITestKey {
    name: string
    vkey: string
    keyId: number
    sign(message: Uint8Array): Uint8Array
}

const testKey = (name: string, seed: number, type: number = notes.SIG_ED25519): ITestKey => {
    const der = Buffer.concat([Buffer.from(PKCS8_ED25519_PREFIX, "hex"), Buffer.alloc(32, seed)])
    const privateKey = createPrivateKey({key: der, format: "der", type: "pkcs8"})
    const spki = createPublicKey(privateKey).export({format: "der", type: "spki"})
    const key = new Uint8Array(spki.subarray(spki.length - 32))
    return {
        name,
        vkey: notes.formatVkey(name, type, key),
        keyId: notes.keyId(name, type, key),
        sign: (message) => new Uint8Array(signEd25519(null, Buffer.from(message), privateKey)),
    }
}

const PRIVATE_PSEUDO = encoding.hexEncode(
    hashes.sha256("vitrine/origin/v1", "logs.acme.example/e/ballots", new Uint8Array(32).fill(0x5a))
)

const KEYS = {
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
    hub3: testKey("hub3.example", 19),
    hubWitness1: testKey("hub-witness1.example", 15, notes.SIG_COSIGNATURE_V1),
    hubWitness2: testKey("hub-witness2.example", 16, notes.SIG_COSIGNATURE_V1),
    hubWitness3: testKey("hub-witness3.example", 20, notes.SIG_COSIGNATURE_V1),
    auditor: testKey("auditor.example", 9),
    ballotsTime: testKey("logs.acme.example/e/ballots", 10, notes.SIG_COSIGNATURE_V1),
    eventsTime: testKey("logs.acme.example/e/events", 11, notes.SIG_COSIGNATURE_V1),
}

export type TVisibility = "public" | "private"

export interface IPolicyOptions {
    visibility?: TVisibility
    /** The event its `ballots` log names. */
    eventId?: string
    /** The policy's `name`. */
    name?: string
    /** Adds the bootstrap hub h3, in which the app hubs anchor. */
    bootstrap?: boolean
}

/** The policy's lines without the seal: the draft the Board seals. */
const policyLines = (o: IPolicyOptions): string[] => {
    const isPrivate = o.visibility === "private"
    return [
        "vitrine/policy/v1",
        `name ${o.name ?? "acme-2026"}`,
        `visibility ${o.visibility ?? "public"}`,
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
        ...(isPrivate ? [`blinded ballots ${PRIVATE_PSEUDO} ${KEYS.blinded.vkey}`] : []),
        ...(isPrivate ? [`opens ${NOW - 3600}`, `closes ${NOW + 3600}`] : []),
        `param ballots event-id ${o.eventId ?? EVENT_ID}`,
        `param ballots election-ids ${ELECTION_IDS.join(" ")}`,
        `log events ${KEYS.events.vkey} ${LOG_URLS.events}`,
        `log-time events ${KEYS.eventsTime.vkey}`,
        `profile events ${EVENTS_TAG} ${EVENTS_MEANING_HASH}`,
        ...(isPrivate
            ? [`blinded events ${KEYS.blindedEvents.vkey.split("+")[0]} ${KEYS.blindedEvents.vkey}`]
            : []),
        ...KEYS.witnesses.map((w, i) => `witness w${i + 1} ${w.vkey}`),
        "witness-party w2 observers",
        "group witnesses 2 w1 w2 w3",
        "quorum witnesses",
        `monitor observers ${KEYS.monitor.vkey} https://monitor.example/`,
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
        ...(o.bootstrap
            ? [
                  `hub h3 ${KEYS.hub3.vkey} https://hub3.example/`,
                  "hub-operator h3 hubco3",
                  `hub-witness h3 w ${KEYS.hubWitness3.vkey}`,
                  "hub-witness-party h3 w hub-witness-party3",
                  "hub-quorum h3 w",
                  "bootstrap h3",
              ]
            : []),
        `monitor observers2 ${KEYS.monitor2.vkey} https://monitor2.example/`,
        "monitor-party observers monitorco1",
        "monitor-party observers2 monitorco2",
        "monitor-quorum 2",
    ]
}

/** The draft Harvest makes: the policy lines without the seal, unsigned. */
export const draftPolicyText = (o: IPolicyOptions = {}): string => `${policyLines(o).join("\n")}\n`

/** The policy sealed from that draft, without the Board's signature. */
export const sealedPolicyBody = (o: IPolicyOptions = {}): string => {
    const lines = policyLines(o)
    const seal = encoding.b64encode(hashes.sha256(`seal ${lines.join(" ")}`).slice(0, 16))
    return `${[...lines, `seal board ${seal}`].join("\n")}\n`
}

/** The sealed policy, signed by the Board, as it uploads it. */
export const policyText = (o: IPolicyOptions = {}): string => {
    const body = sealedPolicyBody(o)
    return notes.formatNote(body, [
        {name: KEYS.board.name, keyId: KEYS.board.keyId, sig: KEYS.board.sign(bytes.utf8(body))},
    ])
}

/** A signed policy's ID as the sidecar reports it: the hex of SHA-256 of its
 * text. */
export const policyHexOf = (note: string): string =>
    encoding.hexEncode(hashes.sha256(notes.parseNote(note).textBytes))

/** The sidecar's `GET /admin/logs` entries for the event's two logs. */
export const sidecarLogs = (policyHex: string): unknown[] => [
    {
        name: `${EVENT_ID}-ballots`,
        origin: KEYS.ballots.name,
        visibility: "public",
        frozen: false,
        size: 12,
        policy: policyHex,
        alert: null,
        queue: {rows: 2, oldest_seconds: 4.5},
    },
    {
        name: `${EVENT_ID}-events`,
        origin: KEYS.events.name,
        visibility: "public",
        frozen: false,
        size: 3,
        policy: policyHex,
        alert: "witness-quorum-late",
        queue: {rows: 0},
    },
]

/** The sidecar's `GET <log>/status` of each log, by the event's name for it. */
export const sidecarStatus = (policyHex: string): Record<string, unknown> => ({
    ballots: {
        name: `${EVENT_ID}-ballots`,
        origin: KEYS.ballots.name,
        visibility: "public",
        size: 12,
        checkpoint: {size: 10, time: NOW - 30},
        policy: policyHex,
        frozen: false,
        witnesses: [
            {name: "w1", current: true, size: 10, cosigned_age_seconds: 30.2, alert: false},
            {
                name: "w2",
                current: false,
                size: 7,
                cosigned_age_seconds: 4000,
                last_error: "connection refused",
                alert: true,
            },
            {name: "w3", current: false},
        ],
        hubs: {
            h1: {origin: "hub1.example", size: 8, entry_index: 41, hub_size: 50, time: NOW - 90},
            h2: {origin: "hub2.example", size: null, entry_index: null, hub_size: null, time: null},
        },
    },
    events: {
        name: `${EVENT_ID}-events`,
        origin: KEYS.events.name,
        visibility: "public",
        size: 3,
        checkpoint: null,
        policy: policyHex,
        frozen: false,
        witnesses: [],
        hubs: {},
    },
})
