// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The ballot style row, as the components that draw a ballot need it.
 *
 * Moved out of `voting-portal/src/store/ballotStyles/ballotStylesSlice.ts`, where
 * this interface used to live beside a `createSlice` call and a `RootState`
 * import. That placement meant the *type* of a ballot component's main prop came
 * from a redux slice file — so importing the type pulled `@reduxjs/toolkit` and
 * the portal's whole store definition into anything that mentioned it, including
 * this package and the Election Architect.
 *
 * Nothing about the shape changed. `ballot_eml` is `ui-core`'s `IBallotStyle` —
 * the document the platform actually publishes — and this is the database row
 * wrapped around it. The portal re-exports this from its old location so its own
 * imports keep working and its selectors keep their types.
 */

import type {IBallotStyle as IPublishedBallotStyle} from "@sequentech/ui-core"

export interface IBallotStyle {
    id: string
    election_id: string
    election_event_id: string
    tenant_id: string
    /** The published document: contests, candidates, presentation, dates, key. */
    ballot_eml: IPublishedBallotStyle
    ballot_signature?: string | null
    created_at: string
    area_id?: string | null
    annotations?: string | null
    labels?: string | null
    last_updated_at: string
}
