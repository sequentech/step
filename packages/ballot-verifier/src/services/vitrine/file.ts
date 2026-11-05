// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** Saves `text` as a file on this device, without any request. */
export const saveTextFile = (name: string, text: string, type: string): void => {
    const url = URL.createObjectURL(new Blob([text], {type}))
    const link = document.createElement("a")
    link.href = url
    link.download = name
    link.rel = "noopener"
    document.body.appendChild(link)
    link.click()
    link.remove()
    URL.revokeObjectURL(url)
}
