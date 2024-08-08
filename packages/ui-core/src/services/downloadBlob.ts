// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {sleep} from "./sleep"

export const downloadBlob = async (blob: Blob, name: string) => {
    let exportUrl = URL.createObjectURL(blob)

    await downloadUrl(exportUrl, name)
}

export const downloadUrl = async (url: string, name: string) => {
    // Fetch the file data
    const response = await fetch(url)
    const blob = await response.blob()

    // Create a blob URL
    const blobUrl = URL.createObjectURL(blob)

    const link = document.createElement("a")
    link.href = blobUrl
    link.target = "_blank"
    link.download = name

    // this is necessary as link.click() does not work on the latest firefox
    link.dispatchEvent(
        new MouseEvent("click", {
            bubbles: true,
            cancelable: true,
            view: window,
        })
    )
    await sleep(100)
    URL.revokeObjectURL(url)
    link.remove()
}
