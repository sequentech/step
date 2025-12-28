// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export function useGetEventDocumentUrl() {
    const getImageUrl = (
        tenantId?: string,
        imageDocumentId?: string | null,
        name?: string | null,
        electionEventId?: string
    ) => `tenant-${tenantId}/event-${electionEventId}/document-${imageDocumentId}/${name}`
    return getImageUrl
}
