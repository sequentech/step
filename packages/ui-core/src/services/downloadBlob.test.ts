/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {afterEach, expect, it, jest} from "@jest/globals"
import {downloadBlob, downloadUrl} from "./downloadBlob"

const originalUrlMethods = new Map(
    ["createObjectURL", "revokeObjectURL"].map((name) => [
        name,
        Object.getOwnPropertyDescriptor(URL, name),
    ])
)

afterEach(() => {
    jest.restoreAllMocks()
    document.body.replaceChildren()
    for (const [name, descriptor] of originalUrlMethods) {
        if (descriptor) Object.defineProperty(URL, name, descriptor)
        else Reflect.deleteProperty(URL, name)
    }
})

it("clicks a temporary download link and removes it afterwards", async () => {
    const click = jest.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
        this: HTMLAnchorElement
    ) {
        expect(this.isConnected).toBe(true)
        expect(this.href).toBe("https://example.org/results.csv")
        expect(this.download).toBe("results.csv")
        expect(this.target).toBe("_blank")
    })
    await downloadUrl("https://example.org/results.csv", "results.csv")
    expect(click).toHaveBeenCalledTimes(1)
    expect(document.querySelector("a")).toBeNull()
})

it.each([false, true])(
    "releases the object URL and DOM node when click fails: %s",
    async (failClick) => {
        const create = jest
            .fn<typeof URL.createObjectURL>()
            .mockReturnValue("blob:synthetic-export")
        const revoke = jest.fn<typeof URL.revokeObjectURL>()
        // jsdom has no object-URL store. Stub only that browser resource boundary;
        // the anchor still lives in the real test document.
        Object.defineProperty(URL, "createObjectURL", {configurable: true, value: create})
        Object.defineProperty(URL, "revokeObjectURL", {configurable: true, value: revoke})
        const failure = new Error("download interrupted")
        jest.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {
            if (failClick) throw failure
        })
        const blob = new Blob(["synthetic result"])
        const operation = downloadBlob(blob, "results.csv")
        if (failClick) await expect(operation).rejects.toBe(failure)
        else await expect(operation).resolves.toBeUndefined()
        expect(create).toHaveBeenCalledWith(blob)
        expect(revoke).toHaveBeenCalledWith("blob:synthetic-export")
        expect(document.querySelector("a")).toBeNull()
    }
)
