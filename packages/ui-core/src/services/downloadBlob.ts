// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const downloadBlob = async (blob: Blob, name: string): Promise<void> => {
    const exportUrl = URL.createObjectURL(blob)
    try {
        await downloadUrl(exportUrl, name)
    } finally {
        // Failed downloads must release the in-memory export as well.
        URL.revokeObjectURL(exportUrl)
    }
}

export const downloadUrl = async (url: string, name: string) => {
    const link = document.createElement("a")
    link.href = url
    link.target = "_blank"
    link.download = name
    document.body.appendChild(link)
    try {
        link.click()
    } finally {
        link.remove()
    }
}
