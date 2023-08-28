// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {sleep} from "./sleep"

export const downloadBlob = async (blob: Blob, name: string) => {
    let exportUrl = URL.createObjectURL(blob)

    await downloadUrl(exportUrl, name)
}

export const downloadUrl = async (url: string, name: string) => {
    const link = document.createElement("a")
    link.href = url
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
