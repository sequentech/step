// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export function useGetDocumentUrl() {
    const getImageUrl = (
        tenantId?: string,
        imageDocumentId?: string | null,
        name?: string | null
    ) => `tenant-${tenantId}/document-${imageDocumentId}/${name}`
    return getImageUrl
}
