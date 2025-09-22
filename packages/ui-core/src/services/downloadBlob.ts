// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const downloadBlob = async (blob: Blob, name: string) => {
    let exportUrl = URL.createObjectURL(blob)

    await downloadUrl(exportUrl, name)

    // Optionally, revoke the blob URL after opening if no longer needed:
    URL.revokeObjectURL(exportUrl)
}

export const downloadUrl = async (url: string, name: string) => {
    const link = document.createElement("a")
    link.href = url
    link.target = "_blank"
    link.download = name
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
}
