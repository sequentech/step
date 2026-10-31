// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useQueryClient, type QueryClient} from "@tanstack/react-query"

let client: QueryClient | undefined

/**
 * Exposes the story's react-query client. A widget that renders nothing both
 * while its read is pending and after it failed is told apart by the read's
 * status.
 */
export function QueryStateProbe() {
    client = useQueryClient()
    return null
}

/** Status of each react-admin read of the story, e.g. `["error"]`. */
export const readStatuses = () =>
    client
        ?.getQueryCache()
        .getAll()
        .map((query) => query.state.status) ?? []
