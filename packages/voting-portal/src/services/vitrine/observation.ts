// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the booth tells the policy's monitors (Vitrine's specs/evidence.md,
// sections 3.1 and 5): observations and promise escrow, without cookies and
// after a random delay of up to a minute. Observations are block documents:
//
//   vitrine/observation/v1
//   policy <policy ID>
//   kind <kind>
//   time <POSIX seconds>
//   <kind-specific lines>
//   <blocks: the objects the monitor checks itself>

import type {Policy} from "@vitrine/verify"
import type {blocks} from "@vitrine/verify/pure"
import {httpPost, IHttpOptions, joinUrl} from "./http"
import type {TVitrinePure} from "./sdk"
import {IVitrineLoggedStatement, VOTING_PORTAL_PACKAGE} from "./types"

export const OBSERVATION_HEADER = "vitrine/observation/v1"
export const OBSERVATION_TYPE = "text/plain; charset=utf-8"
const PROMISE_TYPE = "text/plain"

/** Observations and escrowed promises leave after up to this long. */
export const MAX_REPORT_DELAY_MS = 60_000

export enum EObservationKind {
    CONFIG_MISMATCH = "config-mismatch",
    NO_RECEIPT = "no-receipt",
}

const observation = (
    pure: TVitrinePure,
    policyId: string,
    kind: EObservationKind,
    time: number,
    lines: readonly string[],
    objects: readonly blocks.Block[]
): string =>
    pure.bytes.fromUtf8(
        pure.blocks.formatBlockDoc(
            OBSERVATION_HEADER,
            [`policy ${policyId}`, `kind ${kind}`, `time ${time}`, ...lines],
            objects
        )
    )

const statementBlocks = (
    pure: TVitrinePure,
    name: string,
    logged: IVitrineLoggedStatement | null | undefined
): blocks.Block[] =>
    logged
        ? [
              {type: name, args: [], body: pure.bytes.utf8(logged.statement)},
              {type: `${name}-entry`, args: [], body: pure.bytes.utf8(logged.entry)},
              {type: `${name}-chain`, args: [], body: pure.bytes.utf8(logged.chain)},
          ]
        : []

export interface IConfigObservation {
    policyId: string
    time: number
    eventId: string
    electionId: string
    /** Why the booth refused, as the booth check's reason code. */
    reason: string
    /** The `key` the booth computed from the key it was given. */
    key?: string
    release: string
    config?: IVitrineLoggedStatement | null
    opened?: IVitrineLoggedStatement | null
}

/** A booth was shown a configuration it couldn't bind to the events log
 * (specs/entry.md, section 5.2); the monitor checks the objects and reports
 * `config-mismatch`. The ballot style itself stays out: it names the voter's
 * area. */
export const configObservation = (pure: TVitrinePure, o: IConfigObservation): string =>
    observation(
        pure,
        o.policyId,
        EObservationKind.CONFIG_MISMATCH,
        o.time,
        [
            `event ${o.eventId}`,
            `election ${o.electionId}`,
            `reason ${o.reason}`,
            ...(o.key ? [`key ${o.key}`] : []),
            `release ${VOTING_PORTAL_PACKAGE} ${o.release}`,
        ],
        [...statementBlocks(pure, "config", o.config), ...statementBlocks(pure, "opened", o.opened)]
    )

/** Under `required on`, a cast that got neither a receipt nor a promise the
 * booth accepted: the log's name, the time and the answer, never the entry or
 * its leaf hash (specs/evidence.md, section 3.1). */
export const noReceiptObservation = (
    pure: TVitrinePure,
    policyId: string,
    log: string,
    time: number,
    answer: string,
    reason?: string
): string =>
    observation(
        pure,
        policyId,
        EObservationKind.NO_RECEIPT,
        time,
        [`log ${log}`, `answer ${answer}${reason ? ` ${reason}` : ""}`],
        []
    )

export interface IReportOptions extends IHttpOptions {
    random?: () => number
    /** Runs `send` after `delayMs`; tests run it at once. */
    schedule?: (send: () => Promise<void>, delayMs: number) => void
}

const delayed = (opts: IReportOptions, url: string, body: string, type: string): void => {
    const random = opts.random ?? Math.random
    const schedule =
        opts.schedule ??
        ((send: () => Promise<void>, delayMs: number) => {
            setTimeout(send, delayMs)
        })
    schedule(
        async () => {
            await httpPost(url, body, type, opts)
        },
        Math.floor(random() * MAX_REPORT_DELAY_MS)
    )
}

/** Sends `body` to every monitor of the policy, each after its own delay. */
export const sendObservation = (policy: Policy, body: string, opts: IReportOptions = {}): void => {
    Array.from(policy.monitors.values()).forEach((m) => {
        delayed(opts, joinUrl(m.url, "observe"), body, OBSERVATION_TYPE)
    })
}

/** Promise escrow (receipt.md, section 2.1): one monitor, chosen at random. */
export const escrowPromise = (policy: Policy, promise: string, opts: IReportOptions = {}): void => {
    const monitors = Array.from(policy.monitors.values())
    if (monitors.length === 0) return
    const random = opts.random ?? Math.random
    const monitor = monitors[Math.min(monitors.length - 1, Math.floor(random() * monitors.length))]
    delayed(opts, joinUrl(monitor.url, "promise"), promise, PROMISE_TYPE)
}
