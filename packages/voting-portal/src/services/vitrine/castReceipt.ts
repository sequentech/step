// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The receipt after the cast (step-integration.md, section 2.6; Vitrine's
// specs/receipt.md, sections 1.2 and 2, and specs/evidence.md, section 3.1).
// Harvest's answer is data for @vitrine/verify: the receipt, the promise, or
// the entry the log queued. The portal checks it against the pinned policy,
// with an expectation built from its own IDs, and upgrades it while the
// confirmation screen is open: from the log alone for public events, through
// Harvest for private ones. Under `required on`, a cast without a receipt or
// an accepted promise is shown as a failure and reported to the monitors.
// The vote is already cast: nothing here throws into the page.

import type {ChainResult, Expect, Policy, PolicyHistory} from "@vitrine/verify"
import type {notes, promises} from "@vitrine/verify/pure"
import {pinnedPolicy} from "./boothCheck"
import {FetchLike, httpGet, IHttpResponse, joinUrl} from "./http"
import {escrowPromise, IReportOptions, noReceiptObservation, sendObservation} from "./observation"
import type {ITransparency} from "./presentation"
import {
    castEntry,
    chainDocument,
    chainPromise,
    chainReceipt,
    entryAt,
    hasReceipt,
    hubBlocks,
    improves,
    IOwnEntry,
    IReceiptTarget,
    mergeChain,
    promisedEntry,
    receiptEntry,
    receiptExpect,
    upgradeBlocks,
} from "./receipt"
import {IVitrineSdk, loadVitrineSdk} from "./sdk"
import {
    ETransparencyMode,
    EVitrineCastKind,
    EVitrineForwardRoute,
    EVitrineLog,
    IVitrineCast,
    IVitrineVoterContext,
} from "./types"

/** What the page says besides the receipt itself. */
export enum EReceiptNotice {
    /** The receipt speaks for itself. */
    NONE = "none",
    /** No receipt yet: the portal asks the log for it while the page is open. */
    WAITING = "waiting",
    /** `required on`, and neither a receipt nor an accepted promise. */
    MISSING = "missing",
    /** Nothing could be checked: no verifier, no pinned policy, or no entry. */
    UNCHECKED = "unchecked",
}

/** Why a cast got no receipt the booth accepts, when the portal says so
 * itself (specs/evidence.md, section 3.1, `no-receipt`). */
export enum ENoReceiptReason {
    EMPTY = "empty",
    INVALID = "invalid",
    ENTRY_MISMATCH = "entry-mismatch",
    PROMISE_TIME = "promise-time",
    NO_VITRINE = "no-vitrine",
    UNKNOWN_KIND = "unknown-kind",
    NO_LOG = "no-log",
}

/** The `answer` of a `no-receipt` observation when Harvest gave none the
 * portal could use. */
const NO_ANSWER = "none"

const CAST_KINDS: readonly string[] = [
    EVitrineCastKind.RECEIPT,
    EVitrineCastKind.PROMISE,
    EVitrineCastKind.QUEUED,
    EVitrineCastKind.UNAVAILABLE,
]

export interface IReceiptView {
    notice: EReceiptNotice
    policy?: Policy
    /** The pinned policy's signed note. */
    policyText?: string
    expect?: Expect
    /** The receipt chain, once there is a receipt or an accepted promise. */
    chain?: string
    result?: ChainResult
    /** The voter's entry, when the portal knows its `accepted_at`. */
    own?: IOwnEntry
}

export type TForward = (
    route: EVitrineForwardRoute,
    body: string,
    hub?: string
) => Promise<IHttpResponse>

export interface IReceiptRunOptions {
    target: IReceiptTarget
    transparency: ITransparency
    /** The voter context: the policy and its acknowledgement of private events. */
    context?: () => Promise<IVitrineVoterContext | undefined>
    /** Harvest's `vitrine_voter_forward`, for private events. */
    forward?: TForward
    fetch?: typeof fetch
    signal?: AbortSignal
    /** The device clock, milliseconds. */
    now?: () => number
    sleep?: (ms: number, signal?: AbortSignal) => Promise<void>
    report?: IReportOptions
    loadSdk?: () => Promise<IVitrineSdk>
    onView: (view: IReceiptView) => void
}

export interface ICastReceiptOptions extends IReceiptRunOptions {
    /** Harvest's answer; null when it gave none for an event with
     * transparency on. */
    cast: IVitrineCast | null
}

/** A receipt kept on this device: its chain, or the entry that waits for one. */
export interface IKeptReceiptOptions extends IReceiptRunOptions {
    chain?: string
    acceptedAt?: number
}

/** The first wait between two requests for a receipt, and the longest. */
export const FIRST_POLL_MS = 2_000
export const MAX_POLL_MS = 30_000
/** How long after `accepted_at + mmd` the log may still answer for a queued
 * entry: a queue log looks from `near` to `near + mmd + 60`. */
const RECEIPT_GRACE = 90
/** How long after a promise's deadline the booth still asks for its receipt. */
const PROMISE_ASK_GRACE = 30
/** `|time - now|` beyond which a promise is no promise (receipt.md, section 2). */
const PROMISE_ACCEPT_WINDOW = 60
/** The upgrader's reason when the log answered 425: a quiet period. */
const LOG_QUIET = "log-quiet"

// A reason code Harvest passes on from the sidecar's `queued` or
// `unavailable` answer.
const REASON_CODE = /^[a-z0-9][a-z0-9-]{0,31}$/

// One observation per cast and page load.
const observed = new Set<string>()

const defaultSleep = (ms: number, signal?: AbortSignal): Promise<void> =>
    new Promise((resolve) => {
        const timer = setTimeout(resolve, ms)
        signal?.addEventListener(
            "abort",
            () => {
                clearTimeout(timer)
                resolve()
            },
            {once: true}
        )
    })

class ReceiptRun {
    private view: IReceiptView
    private readonly history: PolicyHistory
    /** The voter's entry while it waits for a receipt. */
    private waiting?: IOwnEntry
    private promiseDeadline?: number

    constructor(
        private readonly sdk: IVitrineSdk,
        private readonly o: IReceiptRunOptions,
        private readonly policy: Policy,
        policyText: string,
        private readonly context: IVitrineVoterContext | undefined
    ) {
        this.history = new sdk.verify.PolicyHistory(policy)
        this.view = {
            notice: EReceiptNotice.NONE,
            policy,
            policyText,
            expect: receiptExpect(o.target),
        }
    }

    private get mode(): ETransparencyMode {
        return this.o.transparency.mode
    }

    private get aborted(): boolean {
        return Boolean(this.o.signal?.aborted)
    }

    private nowSeconds = (): number => Math.floor((this.o.now ?? Date.now)() / 1000)

    private sleep(ms: number): Promise<void> {
        return (this.o.sleep ?? defaultSleep)(ms, this.o.signal)
    }

    private show(patch: Partial<IReceiptView>): void {
        this.view = {...this.view, ...patch}
        if (!this.aborted) this.o.onView(this.view)
    }

    private verify(chain: string, expect: Expect): ChainResult | undefined {
        try {
            return this.sdk.verify.verifyChain(chain, this.policy, {
                expect,
                now: this.nowSeconds(),
                where: "page",
                history: this.history,
            })
        } catch {
            return undefined
        }
    }

    private get ballotsLog() {
        return this.policy.logs.get(EVitrineLog.BALLOTS)
    }

    /** A private event's acknowledgement of the policy, from Harvest, if the
     * log signed it. Public chains get theirs from the log. */
    private get policyAck(): string | undefined {
        const ack = this.context?.policy_ack
        const log = this.ballotsLog
        if (this.mode !== ETransparencyMode.PRIVATE || !ack || !log) return undefined
        try {
            this.sdk.pure.statements.verifyPolicyAck(ack, log, this.policy.id)
            return ack
        } catch {
            return undefined
        }
    }

    /** Shows a chain for the voter's entry, if it verifies. */
    private accept(chain: string, own: IOwnEntry): boolean {
        const expect = receiptExpect(this.o.target, own.acceptedAt)
        const result = this.verify(chain, expect)
        if (!result || !hasReceipt(result)) return false
        this.waiting = undefined
        this.show({notice: EReceiptNotice.NONE, chain, result, expect, own})
        return true
    }

    /** No receipt and no accepted promise: a failure under `required on`,
     * reported once to the monitors with the log's name, the time and the
     * answer, never the entry (specs/evidence.md, section 3.1). */
    private noReceipt(answer: string, own: IOwnEntry | undefined, reason?: string): void {
        this.waiting = own
        const expect = receiptExpect(this.o.target, own?.acceptedAt)
        if (!this.policy.required) {
            this.show({
                notice: own ? EReceiptNotice.WAITING : EReceiptNotice.UNCHECKED,
                expect,
                own,
            })
            return
        }
        const {eventId, electionId, ballotId} = this.o.target
        const seen = `${eventId}/${electionId}/${ballotId}`
        if (!observed.has(seen)) {
            observed.add(seen)
            sendObservation(
                this.policy,
                noReceiptObservation(
                    this.sdk.pure,
                    this.policy.id,
                    EVitrineLog.BALLOTS,
                    this.nowSeconds(),
                    answer,
                    reason
                ),
                this.o.report
            )
        }
        this.show({notice: EReceiptNotice.MISSING, expect, own})
    }

    private fromReceipt(text: string): void {
        const {RECEIPT} = EVitrineCastKind
        try {
            this.sdk.pure.proofs.parseTlogProof(text)
        } catch {
            return this.noReceipt(RECEIPT, undefined, ENoReceiptReason.INVALID)
        }
        const own = receiptEntry(this.sdk, this.o.target, text)
        if (!own) return this.noReceipt(RECEIPT, undefined, ENoReceiptReason.ENTRY_MISMATCH)
        const chain = chainDocument(this.sdk, this.policy.id, {
            issued: text,
            policyAck: this.policyAck,
        })
        if (!this.accept(chain, own)) this.noReceipt(RECEIPT, own, ENoReceiptReason.INVALID)
    }

    /** The time a promise is judged against: the quorum-vouched time of the
     * log's latest checkpoint, else the device clock (receipt.md, section 2). */
    private async signedNow(): Promise<number> {
        const device = this.nowSeconds()
        const url = this.ballotsLog?.url
        const quiet = this.sdk.pure.policies.inQuietPeriod(this.policy, device)
        if (this.mode !== ETransparencyMode.PUBLIC || quiet || !url) return device
        const got = await httpGet(joinUrl(url, "checkpoint"), this.http)
        if (got.status !== 200 || !got.body) return device
        return (
            this.sdk.pure.history.vouchedNow(
                this.history,
                [got.body],
                this.sdk.pure.history.logKeys(EVitrineLog.BALLOTS),
                device
            ) ?? device
        )
    }

    /** A promise (receipt.md, section 2): escrowed in any case, and accepted
     * only for the voter's entry and when its batch's time is near now. */
    private async fromPromise(text: string, logKey: notes.Vkey): Promise<void> {
        const {PROMISE} = EVitrineCastKind
        escrowPromise(this.policy, text, this.o.report)
        let promise: promises.VerifiedPromise
        try {
            promise = this.sdk.pure.promises.verifyPromise(text, logKey, (id) => {
                const p = this.history.byId(id)
                return p && {mmd: p.mmd, skew: p.skew}
            })
        } catch {
            return this.noReceipt(PROMISE, undefined, ENoReceiptReason.INVALID)
        }
        const own = promisedEntry(this.sdk, this.o.target, promise.leaf, promise.batch.time)
        if (!own) return this.noReceipt(PROMISE, undefined, ENoReceiptReason.ENTRY_MISMATCH)
        const now = await this.signedNow()
        if (Math.abs(promise.batch.time - now) > PROMISE_ACCEPT_WINDOW) {
            return this.noReceipt(PROMISE, own, ENoReceiptReason.PROMISE_TIME)
        }
        const chain = chainDocument(this.sdk, this.policy.id, {
            promise: text,
            policyAck: this.policyAck,
        })
        if (!this.accept(chain, own)) return this.noReceipt(PROMISE, own, ENoReceiptReason.INVALID)
        this.promiseDeadline = promise.deadline
    }

    private get http() {
        return {
            ...(this.o.fetch ? {fetch: this.o.fetch as FetchLike} : {}),
            ...(this.o.signal ? {signal: this.o.signal} : {}),
        }
    }

    private get upgradeOptions() {
        return {
            ...(this.o.fetch ? {fetch: this.o.fetch} : {}),
            ...(this.o.signal ? {signal: this.o.signal} : {}),
            ...(this.o.sleep ? {sleep: this.o.sleep} : {}),
            now: this.nowSeconds,
            history: this.history,
        }
    }

    /** One request to the log for the waiting entry's receipt
     * (`receipt/<leaf hash>?near=<accepted_at>`, receipt.md, section 2.2).
     * False when the log is in a quiet period and won't answer until it ends. */
    private async askLog(own: IOwnEntry): Promise<boolean> {
        const url = this.ballotsLog?.url
        if (!url) return false
        const leaf = this.sdk.pure.encoding.hexEncode(this.sdk.pure.hashes.leafHash(own.entry))
        const got = await httpGet(joinUrl(url, `receipt/${leaf}?near=${own.acceptedAt}`), this.http)
        if (got.status === 425) return false
        if (got.status === 200 && got.body && receiptEntry(this.sdk, this.o.target, got.body)) {
            this.accept(chainDocument(this.sdk, this.policy.id, {issued: got.body}), own)
        }
        return true
    }

    /** A check from the log alone: successors, the policy acknowledgement,
     * the promise's receipt, the first upgrade, the hub links and the latest
     * checkpoints, without mirrors, hubs, gossip or their delay. False when
     * the log is in a quiet period. */
    private async checkWithLog(): Promise<boolean> {
        const {chain, expect, result} = this.view
        if (!chain || !expect) return false
        try {
            const out = await this.sdk.verify.checkChain(chain, this.policy, {
                ...this.upgradeOptions,
                expect,
                where: "page",
                checkMirror: false,
                hubCheckpoints: false,
                gossip: false,
            })
            if (!result || improves(result, out.result)) {
                this.show({chain: out.chain, result: out.result})
            }
            return !out.report?.reasons.some((r) => r.code === LOG_QUIET)
        } catch {
            return false
        }
    }

    /** While the confirmation screen is open, the upgrade at cast: a record
     * against a checkpoint that meets the quorum, from the log only. */
    private async upgradeAtCast(): Promise<void> {
        const {chain, expect, result} = this.view
        if (!chain || !expect || !chainReceipt(this.sdk, chain)) return
        try {
            const out = await this.sdk.verify.upgradeAtCast(chain, this.policy, {
                ...this.upgradeOptions,
                ...(this.o.now ? {clock: this.o.now} : {}),
            })
            const next = out.upgraded ? this.verify(out.chain, expect) : undefined
            if (next && (!result || improves(result, next))) {
                this.show({chain: out.chain, result: next})
            }
        } catch {
            // A log that contradicts itself: the check that follows keeps
            // the contradiction in its result.
        }
    }

    /** Asks the log until `due` (device clock), less and less often. */
    private async poll(due: number, ask: () => Promise<boolean>, done: () => boolean) {
        let wait = FIRST_POLL_MS
        while (!this.aborted && !done() && this.nowSeconds() <= due) {
            await this.sleep(wait)
            if (this.aborted || !(await ask())) return
            wait = Math.min(wait * 2, MAX_POLL_MS)
        }
    }

    private async upgradePublic(atCast: boolean): Promise<void> {
        const own = this.waiting
        if (own && !this.view.chain) {
            if (!atCast) await this.askLog(own)
            else
                await this.poll(
                    own.acceptedAt + this.policy.mmd + RECEIPT_GRACE,
                    () => this.askLog(own),
                    () => !this.waiting
                )
        }
        const promised = () => Boolean(this.view.chain && chainPromise(this.sdk, this.view.chain))
        if (promised()) {
            if (!atCast) await this.checkWithLog()
            else
                await this.poll(
                    (this.promiseDeadline ?? this.nowSeconds()) + PROMISE_ASK_GRACE,
                    () => this.checkWithLog(),
                    () => !promised()
                )
        }
        if (this.aborted || !this.view.chain || !chainReceipt(this.sdk, this.view.chain)) return
        if (atCast) await this.upgradeAtCast()
        if (!this.aborted) await this.checkWithLog()
    }

    /** Merges a voter route's answer into the chain if it verifies. */
    private mergeIn(answer: string, take: Parameters<typeof mergeChain>[3]): void {
        const {chain, expect, result} = this.view
        if (!chain || !expect || !result) return
        const merged = mergeChain(this.sdk, chain, answer, take, (id) =>
            Boolean(this.history.byId(id))
        )
        const next = merged && this.verify(merged, expect)
        if (merged && next && improves(result, next)) this.show({chain: merged, result: next})
    }

    /** Private events: selective policy proofs supplement inclusion but cannot
     * establish complete discovery or a successor's earliest quorum time.
     * Keep those claims pending; never send a full private events prefix to
     * voters. Upgrades/openings still go through Harvest. With `strict on`,
     * the log answers 425 until voting closes and the chain keeps its promise. */
    private async upgradePrivate(): Promise<void> {
        const forward = this.o.forward
        const chain = this.view.chain
        if (!forward || !chain) return
        const policies = await forward(EVitrineForwardRoute.POLICIES, "")
        if (policies.status === 200 && policies.body) {
            try {
                this.sdk.pure.history.applyPolicyProofs(
                    this.history,
                    policies.body,
                    this.nowSeconds()
                )
                if (this.view.expect) {
                    const result = this.verify(chain, this.view.expect)
                    if (result) this.show({result})
                }
            } catch {
                /* Discovery remains pending until authentic proofs arrive. */
            }
        }
        const body = chainReceipt(this.sdk, chain) ?? chainPromise(this.sdk, chain)
        if (body) {
            const got = await forward(EVitrineForwardRoute.UPGRADE, body)
            if (got.status === 200 && got.body) this.mergeIn(got.body, upgradeBlocks)
        }
        for (const hub of this.sdk.pure.policies.appHubs(this.policy)) {
            const receipt = this.view.chain && chainReceipt(this.sdk, this.view.chain)
            if (this.aborted || !receipt) return
            const got = await forward(EVitrineForwardRoute.OPENING, receipt, hub.name)
            if (got.status === 200 && got.body) this.mergeIn(got.body, hubBlocks(hub.name))
        }
    }

    private async upgrade(atCast: boolean): Promise<void> {
        if (this.aborted) return
        if (this.mode === ETransparencyMode.PUBLIC) await this.upgradePublic(atCast)
        else await this.upgradePrivate()
    }

    async fromCast(cast: IVitrineCast | null): Promise<void> {
        const text = cast?.text ?? ""
        const logKey = this.ballotsLog?.vkey
        if (!cast) this.noReceipt(NO_ANSWER, undefined, ENoReceiptReason.NO_VITRINE)
        else if (!CAST_KINDS.includes(cast.kind)) {
            this.noReceipt(NO_ANSWER, undefined, ENoReceiptReason.UNKNOWN_KIND)
        } else if (!logKey) this.noReceipt(cast.kind, undefined, ENoReceiptReason.NO_LOG)
        else if (cast.kind === EVitrineCastKind.RECEIPT) {
            if (text) this.fromReceipt(text)
            else this.noReceipt(cast.kind, undefined, ENoReceiptReason.EMPTY)
        } else if (cast.kind === EVitrineCastKind.PROMISE) {
            if (text) await this.fromPromise(text, logKey)
            else this.noReceipt(cast.kind, undefined, ENoReceiptReason.EMPTY)
        } else {
            const own = text ? castEntry(this.sdk, this.o.target, text) : undefined
            const reason = cast.reason && REASON_CODE.test(cast.reason) ? cast.reason : undefined
            this.noReceipt(cast.kind, own, reason)
        }
        await this.upgrade(true)
    }

    /** A kept chain is shown as it verifies now; a kept entry waits for its
     * receipt. */
    async fromKept(chain: string | undefined, acceptedAt: number | undefined): Promise<void> {
        const receipt = chain ? chainReceipt(this.sdk, chain) : undefined
        const own =
            (receipt ? receiptEntry(this.sdk, this.o.target, receipt) : undefined) ??
            (acceptedAt === undefined ? undefined : entryAt(this.sdk, this.o.target, acceptedAt))
        const expect = receiptExpect(this.o.target, own?.acceptedAt)
        if (chain) {
            this.show({chain, expect, own, result: this.verify(chain, expect)})
        } else {
            this.waiting = own
            this.show({
                notice: own ? EReceiptNotice.WAITING : EReceiptNotice.UNCHECKED,
                expect,
                own,
            })
        }
        await this.upgrade(false)
    }
}

const start = async (o: IReceiptRunOptions): Promise<ReceiptRun | undefined> => {
    const {mode} = o.transparency
    if (mode === ETransparencyMode.DISABLED) return undefined
    const unchecked = () => {
        if (!o.signal?.aborted) o.onView({notice: EReceiptNotice.UNCHECKED})
    }
    let sdk: IVitrineSdk
    try {
        sdk = await (o.loadSdk ?? loadVitrineSdk)()
    } catch {
        unchecked()
        return undefined
    }
    let context: IVitrineVoterContext | undefined
    if (mode === ETransparencyMode.PRIVATE) {
        try {
            context = await o.context?.()
        } catch {
            context = undefined
        }
    }
    const text =
        mode === ETransparencyMode.PUBLIC ? o.transparency.vitrine?.policy : context?.policy
    const pinned = pinnedPolicy(sdk, mode, o.transparency.vitrine, context?.policy)
    if (!pinned.ok || !text) {
        unchecked()
        return undefined
    }
    return new ReceiptRun(sdk, o, pinned.policy, text, context)
}

/** Checks Harvest's answer at cast and upgrades the receipt while the page is
 * open, reporting each new view. */
export const runCastReceipt = async (o: ICastReceiptOptions): Promise<void> => {
    try {
        const run = await start(o)
        await run?.fromCast(o.cast)
    } catch {
        // Aborted, or an unexpected failure: the page keeps what it showed.
    }
}

/** Checks a receipt kept on this device, after one attempt to upgrade it
 * (or to get it, for an entry that waited for one). */
export const runKeptReceipt = async (o: IKeptReceiptOptions): Promise<void> => {
    try {
        const run = await start(o)
        await run?.fromKept(o.chain, o.acceptedAt)
    } catch {
        // As above.
    }
}
