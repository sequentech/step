// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export function useGetEventPublicUrl() {
    const getImageUrl = (
        tenantId?: string,
        imageDocumentId?: string | null,
        name?: string | null,
        electionEventId?: string
    ) => `tenant-${tenantId}/event-${electionEventId}/document-${imageDocumentId}/${name}`
    return getImageUrl
}
