// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {expect} from "@playwright/test"
import {test} from "../fixtures"
import {serveTrusteeWorkerHelper} from "./trustee-startup"

test("starts the trustee cryptographic worker pool from browser-served assets", async ({
    page,
    portal,
    context,
}) => {
    // Keep the real worker pool bounded on hosts with many logical CPUs.
    await page.addInitScript(() => {
        Object.defineProperty(navigator, "hardwareConcurrency", {value: 2})
    })
    await serveTrusteeWorkerHelper(context, portal)
    const errors: string[] = []
    page.on("console", (message) => {
        if (message.type() === "error") errors.push(message.text())
    })
    await page.goto(`${portal.origin}/trustee?lang=en`)
    await expect(page.getByRole("heading", {name: "Braid Trustee Node"})).toBeVisible()
    await expect(page.getByText(/braid-wasm loaded and thread pool initialized/)).toBeVisible()
    await expect(page.getByText(/WASM init failed/)).toHaveCount(0)
    expect(errors).toEqual([])
})
