// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect} from "@playwright/test"
import {spawn, type ChildProcess} from "node:child_process"
import {once} from "node:events"
import {copyFile, mkdir, mkdtemp, readFile, rm, symlink, writeFile} from "node:fs/promises"
import {tmpdir} from "node:os"
import {join, resolve} from "node:path"

test("concurrent real-stack servers keep settings private and preserve their shared dist", async () => {
    const root = await mkdtemp(join(tmpdir(), "ui-stack-settings-"))
    const packages = join(root, "packages")
    const entrypoint = join(packages, "ui-e2e/server.mts")
    const portals = ["voting-portal", "admin-portal", "results-portal", "ballot-verifier"]
    const original = '{"customSetting":"preserved","DISABLE_AUTH":true}\n'
    const children: ChildProcess[] = []
    try {
        await mkdir(join(packages, "ui-e2e"), {recursive: true})
        await copyFile(resolve(__dirname, "../../ui-e2e/server.mts"), entrypoint)
        await symlink(resolve(__dirname, "../../node_modules"), join(packages, "node_modules"))
        for (const portal of portals) {
            const dist = join(packages, portal, "dist")
            await mkdir(dist, {recursive: true})
            await writeFile(join(dist, "index.html"), "<main>Synthetic portal</main>")
            await writeFile(join(dist, "global-settings.json"), original)
        }
        const start = async (tenant: string) => {
            const output = join(root, tenant)
            const child = spawn(process.execPath, ["--experimental-strip-types", entrypoint], {
                env: {
                    ...process.env,
                    STEP_UI_E2E_OUTPUT_DIR: output,
                    SUPER_ADMIN_TENANT_ID: tenant,
                },
                stdio: ["ignore", "ignore", "pipe"],
            })
            children.push(child)
            let errors = ""
            child.stderr!.on("data", (data) => (errors += data.toString()))
            let origins: Record<string, string> | undefined
            await expect
                .poll(
                    async () => {
                        if (child.exitCode !== null) throw new Error(errors)
                        try {
                            origins = JSON.parse(
                                await readFile(join(output, "origins.json"), "utf8")
                            )
                            return true
                        } catch {
                            return false
                        }
                    },
                    {timeout: 10000}
                )
                .toBe(true)
            return {tenant, origins: origins!}
        }
        const first = await start("tenant-a")
        expect(
            (await (await fetch(`${first.origins.voting}/global-settings.json`)).json())
                .DEFAULT_TENANT_ID
        ).toBe(first.tenant)
        const second = await start("tenant-b")
        expect(first.origins.voting).not.toBe(second.origins.voting)
        for (const server of [first, second])
            for (const origin of Object.values(server.origins)) {
                const response = await fetch(`${origin}/global-settings.json`)
                expect(response.status).toBe(200)
                expect.soft(await response.json()).toMatchObject({
                    customSetting: "preserved",
                    DISABLE_AUTH: false,
                    DEFAULT_TENANT_ID: server.tenant,
                    VOTING_PORTAL_URL: server.origins.voting,
                    BALLOT_VERIFIER_URL: `${server.origins.verifier}/`,
                    RESULTS_PORTAL_URL: server.origins.results,
                })
            }
        for (const portal of portals)
            expect
                .soft(await readFile(join(packages, portal, "dist/global-settings.json"), "utf8"))
                .toBe(original)
    } finally {
        for (const child of children) {
            if (child.exitCode !== null || child.signalCode !== null) continue
            const exited = once(child, "exit")
            child.kill("SIGTERM")
            const timeout = setTimeout(() => child.kill("SIGKILL"), 5000)
            try {
                await exited
            } finally {
                clearTimeout(timeout)
            }
        }
        await rm(root, {recursive: true, force: true})
    }
})
