// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The "Updates from hubs" settings (design board AdminHubUpdates): whether the
// voting portal offers voters an email update from each hub the election's
// policy names, on its confirmation page and in the cast ballot receipt. They
// are stored with the election event's other voting portal settings, in
// `presentation.hub_updates`, and both are on until an admin turns them off.

import type {Policy} from "@vitrine/verify"

export enum EHubUpdate {
    ON = "on",
    OFF = "off",
}

export interface IHubUpdates {
    /** The "Get email updates" callout on the confirmation page. */
    confirmation: EHubUpdate
    /** The hubs in the cast ballot receipt. */
    castEmail: EHubUpdate
}

const asObject = (value: unknown): Record<string, unknown> | undefined => {
    let v = value
    if (typeof v === "string") {
        try {
            v = JSON.parse(v)
        } catch {
            return undefined
        }
    }
    return v !== null && typeof v === "object" && !Array.isArray(v)
        ? (v as Record<string, unknown>)
        : undefined
}

const readSwitch = (value: unknown): EHubUpdate =>
    value === EHubUpdate.OFF ? EHubUpdate.OFF : EHubUpdate.ON

/** The switches of an election event's `presentation`: each is on unless it
 * says `off`. */
export const readHubUpdates = (presentation: unknown): IHubUpdates => {
    const updates = asObject(asObject(presentation)?.hub_updates)
    return {
        confirmation: readSwitch(updates?.confirmation),
        castEmail: readSwitch(updates?.cast_email),
    }
}

/** What to append to the event's `presentation` to store the switches. Both
 * are written, so the stored settings say what the admin saw. */
export const hubUpdatesPatch = (updates: IHubUpdates) => ({
    hub_updates: {confirmation: updates.confirmation, cast_email: updates.castEmail},
})

export interface IOfferedHub {
    name: string
    /** Who runs the hub, as the policy says. */
    party: string
    /** The hub's explorer, where voters are sent. */
    site: string
}

/** The hubs voters are offered: those the policy gives a `hub-explorer` site,
 * in the policy's order (voters see them in a random one). */
export const offeredHubs = (policy: Policy | undefined): IOfferedHub[] =>
    policy
        ? Array.from(policy.hubs.values()).flatMap((hub) =>
              hub.explorer ? [{name: hub.name, party: hub.operator, site: hub.explorer}] : []
          )
        : []
