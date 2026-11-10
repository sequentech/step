// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export function useGetDocumentUrl() {
    // The name is one part of the path, whatever characters it has.
    const getImageUrl = (
        tenantId?: string,
        imageDocumentId?: string | null,
        name?: string | null
    ) => `tenant-${tenantId}/document-${imageDocumentId}/${encodeURIComponent(String(name))}`
    return getImageUrl
}
