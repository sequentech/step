// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const downloadTextFile = (content: string, fileName: string): void => {
    const blob = new Blob([content], {type: "text/plain"})
    const blobUrl = window.URL.createObjectURL(blob)
    const tempLink = document.createElement("a")
    tempLink.href = blobUrl
    tempLink.setAttribute("download", fileName)
    document.body.appendChild(tempLink)
    try {
        tempLink.click()
    } finally {
        tempLink.remove()
        window.URL.revokeObjectURL(blobUrl)
    }
}
