// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {readFile, writeFile} from "node:fs/promises"
import {resolve} from "node:path"
import type {Page} from "@playwright/test"
import {test, expect, eventPath} from "./fixtures"

test.skip(!process.env.BALLOT_VERIFIER_JOURNEY_URL, "requires an explicitly selected dev server")

// This opt-in journey owns and restores each edited file, just like the UI bench.
async function editAndRestore(
    page: Page,
    path: string,
    anchor: string,
    replacement: string,
    marker: string,
    preservesState: boolean
) {
    const original = await readFile(path, "utf8")
    expect(original.split(anchor), `one source anchor in ${path}`).toHaveLength(2)
    try {
        await writeFile(path, original.replace(anchor, replacement))
        await expect(page.getByText(marker, {exact: true})).toBeVisible()
        if (preservesState)
            await expect(page.getByRole("textbox", {name: "Ballot ID", exact: true})).toHaveValue(
                "kept-through-refresh"
            )
    } finally {
        await writeFile(path, original)
        await expect(page.getByText(marker, {exact: true})).toHaveCount(0)
        await expect(page.getByRole("heading", {name: "Step 1: Import your ballot"})).toBeVisible()
    }
}

test("leaf, shared component and core translation edits render and restore without page errors", async ({
    page,
    portal,
}) => {
    await page.goto(`${portal.origin}${eventPath}`)
    const ballotId = page.getByRole("textbox", {name: "Ballot ID", exact: true})
    await ballotId.fill("kept-through-refresh")
    const leaf = '<span>{t("homeScreen.step1")}</span>'
    await editAndRestore(
        page,
        resolve(__dirname, "../../src/screens/HomeScreen.tsx"),
        leaf,
        `<span>verifier-leaf-refresh</span>${leaf}`,
        "verifier-leaf-refresh",
        true
    )
    await expect(ballotId).toHaveValue("kept-through-refresh")
    // Header also exports a non-component value, so its update may reload.
    const header = '<Version version={appVersion ?? {main: "0.0.0"}} />'
    await editAndRestore(
        page,
        resolve(__dirname, "../../../ui-essentials/src/components/Header/Header.tsx"),
        header,
        `<span>verifier-shared-refresh</span>${header}`,
        "verifier-shared-refresh",
        false
    )
    await expect(ballotId).toHaveValue("")
    // Core imports reach bootstrap. Re-running createRoot under Fast Refresh
    // causes DOM deletion exceptions; this edit must take the full reload path.
    await editAndRestore(
        page,
        resolve(__dirname, "../../../ui-core/src/translations/en.ts"),
        'header: "Version:",',
        'header: "verifier-core-reload",',
        "verifier-core-reload",
        false
    )
    await expect(ballotId).toHaveValue("")
})
