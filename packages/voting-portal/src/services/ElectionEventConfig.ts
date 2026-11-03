// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {IElectionEventPresentation} from "@sequentech/ui-core"

export interface ElectionEventConfigDocument {
    id: string
    tenant_id: string
    election_event_id: string
    election_event_presentation: IElectionEventPresentation
}

// Create one loader per App mount. Share startup requests (including StrictMode
// effect replay), without retaining stale configuration across page reloads.
export const createElectionEventConfigLoader = () => {
    let current: {url: string; promise: Promise<ElectionEventConfigDocument>} | undefined
    return (url: string): Promise<ElectionEventConfigDocument> => {
        if (current?.url === url) {
            return current.promise
        }
        const promise = (async () => {
            const response = await fetch(url)
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`)
            }
            return (await response.json()) as ElectionEventConfigDocument
        })()
        current = {url, promise}
        void promise.catch(() => {
            // An old failed request must not evict a newer event's request.
            if (current?.promise === promise) {
                current = undefined
            }
        })
        return promise
    }
}
