/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {downloadTextFile} from "./downloadTextFile"

describe("downloadTextFile", () => {
    const blobUrl = "blob:http://localhost/key"
    let createObjectURL: jest.Mock
    let revokeObjectURL: jest.Mock
    let click: jest.SpyInstance

    beforeEach(() => {
        createObjectURL = jest.fn(() => blobUrl)
        revokeObjectURL = jest.fn()
        Object.assign(window.URL, {createObjectURL, revokeObjectURL})
        click = jest.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {})
    })

    afterEach(() => {
        click.mockRestore()
    })

    it("revokes the object URL it created once the download is triggered", () => {
        downloadTextFile("secret-key", "key.txt")

        expect(createObjectURL).toHaveBeenCalledTimes(1)
        expect(click).toHaveBeenCalledTimes(1)
        expect(revokeObjectURL).toHaveBeenCalledWith(blobUrl)
        expect(revokeObjectURL.mock.invocationCallOrder[0]).toBeGreaterThan(
            click.mock.invocationCallOrder[0]
        )
    })

    it("names the download and leaves no link behind in the document", () => {
        let downloadName: string | null = null
        click.mockImplementation(function (this: HTMLAnchorElement) {
            downloadName = this.getAttribute("download")
        })

        downloadTextFile("secret-key", "key.txt")

        expect(downloadName).toBe("key.txt")
        expect(document.querySelector(`a[href="${blobUrl}"]`)).toBeNull()
    })

    it("revokes the object URL even when the click fails", () => {
        click.mockImplementation(() => {
            throw new Error("blocked")
        })

        expect(() => downloadTextFile("secret-key", "key.txt")).toThrow("blocked")
        expect(revokeObjectURL).toHaveBeenCalledWith(blobUrl)
    })
})
