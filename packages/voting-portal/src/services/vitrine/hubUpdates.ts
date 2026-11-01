// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Get email updates" (design boards GetUpdates and HubEmails; DECISIONS D7):
// the hubs an election's policy names, offered to the voter as equal choices.
// Each hub's own site takes the policy ID and the ballot's receipt in the
// link's fragment, which browsers never send to a server, and asks there for
// the address and what to watch. The portal never sees an address.

import {explorersOf, type Policy} from "@vitrine/verify"
import {asObject} from "./presentation"

/** Whether the portal offers updates from the policy's hubs. */
export enum EHubUpdate {
    ON = "on",
    OFF = "off",
}

export interface IHubUpdates {
    /** The "Get email updates" callout on the confirmation page. */
    confirmation: EHubUpdate
    /** The hubs in the cast ballot's email, which Windmill sends. */
    castEmail: EHubUpdate
}

const readSwitch = (value: unknown): EHubUpdate =>
    value === EHubUpdate.OFF ? EHubUpdate.OFF : EHubUpdate.ON

/** The "Updates from hubs" switches of an election event's `presentation`
 * (`hub_updates`, written by the admin portal). Each is on unless it says
 * `off`, as for events created before the switches. */
export const readHubUpdates = (presentation: unknown): IHubUpdates => {
    const updates = asObject(asObject(presentation)?.hub_updates)
    return {
        confirmation: readSwitch(updates?.confirmation),
        castEmail: readSwitch(updates?.cast_email),
    }
}

/** A hub the voter can get updates from. */
export interface IOfferedHub {
    name: string
    /** Who runs the hub, as the policy says. */
    party: string
    /** The hub's explorer: its own site, where voters are sent. */
    site: string
}

/** The hubs that the policy gives a site (`hub-explorer`), in no order that
 * means anything: the screen shuffles them. */
export const offeredHubs = (policy: Policy | undefined): IOfferedHub[] =>
    policy
        ? explorersOf(policy).flatMap((explorer) =>
              explorer.kind === "hub"
                  ? [{name: explorer.name, party: explorer.party, site: explorer.url}]
                  : []
          )
        : []

/** The items in a random order (Fisher-Yates), so that no hub is first more
 * often than another, nor the default. The list given is left alone. */
export const shuffled = <T>(items: readonly T[], random: () => number = Math.random): T[] => {
    const result = [...items]
    for (let i = result.length - 1; i > 0; i--) {
        const j = Math.floor(random() * (i + 1))
        const kept = result[i]
        result[i] = result[j]
        result[j] = kept
    }
    return result
}

/** A text as base64url of its UTF-8 bytes, without padding. */
export const watchPayload = (text: string): string => {
    const bytes = new TextEncoder().encode(text)
    let binary = ""
    for (const byte of Array.from(bytes)) binary += String.fromCharCode(byte)
    return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "")
}

/** The link into a hub's site that starts a watch (D7):
 * `<hub-explorer url>#policy=<sha256:hex>&watch=<base64url receipt>`. The
 * site's own fragment, if any, is dropped; without a receipt yet the link
 * names only the policy, and the site offers to watch the election.
 *
 * Adapter: the vendored Vitrine package has no builder for this link yet.
 * When it gets one, this is the place to swap. */
export const watchLink = (site: string, policyId: string, receipt?: string): string => {
    const params = new URLSearchParams({policy: policyId})
    if (receipt) params.set("watch", watchPayload(receipt))
    const hash = site.indexOf("#")
    return `${hash < 0 ? site : site.slice(0, hash)}#${params.toString()}`
}
