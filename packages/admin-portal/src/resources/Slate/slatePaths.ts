// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** Routes under the election resource, relative to `/sequent_backend_election`. */
export const SLATE_ROUTE = ":id/slates/:slateId"
export const CREATE_SLATE_ROUTE = ":id/create-slate"

export const slatePath = (electionId: string, slateId: string): string =>
    `/sequent_backend_election/${electionId}/slates/${encodeURIComponent(slateId)}`

export const createSlatePath = (electionId: string): string =>
    `/sequent_backend_election/${electionId}/create-slate`
