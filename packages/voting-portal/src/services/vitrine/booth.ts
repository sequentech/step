// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApolloClient} from "@apollo/client"
import {checkBooth, EBoothCheckReason, EBoothRefusalState, pinnedPolicy} from "./boothCheck"
import {electionKeyBytes} from "./digests"
import {configObservation, IReportOptions, sendObservation} from "./observation"
import {eventTransparency} from "./presentation"
import {IVitrineSdk, loadVitrineSdk} from "./sdk"
import {ETransparencyMode} from "./types"
import {fetchVoterContext} from "./voterContext"

/** What the booth encrypts to, from the ballot style. */
export interface IBoothStyle {
    electionEventId: string
    electionId: string
    publicKey?: string
    /** The ballot style's text, exactly as the portal received it. */
    styleText?: string
    /** The ballot style's copy of the event presentation. */
    presentation?: unknown
}

export interface IRunBoothCheck {
    client: ApolloClient
    style: IBoothStyle
    /** The event's own presentation. */
    eventPresentation?: unknown
    /** This portal's version. */
    release: string
    /** True when there is no logged-in voter to ask Harvest for (previews,
     * DISABLE_AUTH). */
    skip: boolean
    now?: () => number
    report?: IReportOptions
    loadSdk?: () => Promise<IVitrineSdk>
}

export type TBoothOutcome =
    | {ok: true}
    | {ok: false; reason: EBoothCheckReason; state: EBoothRefusalState}

// One observation per refusal and election for each page load: a voter who
// tries again doesn't send the same report twice.
const observed = new Set<string>()

/** The check before encrypting, for events with transparency on. Events
 * without it pass at once. */
export const runBoothCheck = async (o: IRunBoothCheck): Promise<TBoothOutcome> => {
    const transparency = eventTransparency(o.eventPresentation, o.style.presentation)
    if (transparency.mode === ETransparencyMode.DISABLED || o.skip) return {ok: true}
    let sdk: IVitrineSdk
    try {
        sdk = await (o.loadSdk ?? loadVitrineSdk)()
    } catch {
        return {
            ok: false,
            reason: EBoothCheckReason.VERIFIER_UNAVAILABLE,
            state: EBoothRefusalState.PENDING,
        }
    }
    const {electionEventId: eventId, electionId} = o.style
    const context = await fetchVoterContext(o.client, eventId, electionId)
    const now = Math.floor((o.now ?? Date.now)() / 1000)
    const result = checkBooth(sdk, {
        mode: transparency.mode,
        presentation: transparency.vitrine,
        context,
        eventId,
        electionId,
        publicKey: o.style.publicKey,
        styleText: o.style.styleText,
        release: o.release,
        now,
    })
    if (result.ok) return {ok: true}
    // Public policy admission does not depend on the context being available.
    // Never derive report destinations from an unpinned policy supplied by a
    // failed private-context request.
    const fallback =
        transparency.mode === ETransparencyMode.PUBLIC
            ? pinnedPolicy(sdk, transparency.mode, transparency.vitrine, undefined)
            : undefined
    const reportPolicy = result.policy ?? (fallback?.ok ? fallback.policy : undefined)
    const seen = `${eventId}/${electionId} ${result.reason}`
    if (reportPolicy && !observed.has(seen)) {
        observed.add(seen)
        try {
            const body = configObservation(sdk.pure, {
                policyId: reportPolicy.id,
                time: now,
                eventId,
                electionId,
                reason: result.reason,
                key: electionKeyBytes(sdk.pure, o.style.publicKey),
                styles: context?.style_hashes ?? undefined,
                styleText: o.style.styleText,
                release: o.release,
                config: context?.config,
                opened: context?.opened,
            })
            sendObservation(reportPolicy, body, o.report)
        } catch {
            /* Reporting cannot replace or bypass the refusal. */
        }
    }
    return {ok: false, reason: result.reason, state: result.state}
}
