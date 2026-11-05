// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect} from "@playwright/test"
import {mkdtemp, rm, writeFile} from "node:fs/promises"
import {tmpdir} from "node:os"
import {join} from "node:path"
import {createPortAllocator, serveDist} from "../server/static"

test("configured static ports keep parallel workers and their second servers separate", () => {
    const first = createPortAllocator(44000, 44003)
    const second = createPortAllocator(44000, 44003)
    const worker0 = {parallelIndex: 0, config: {workers: 2}}
    const worker1 = {parallelIndex: 1, config: {workers: 2}}
    expect([first(worker0), second(worker1), first(worker0), second(worker1)]).toEqual([
        44000, 44001, 44002, 44003,
    ])
    expect(() => first(worker0)).toThrow("range 44000-44003 is exhausted for worker 0")
    expect(() => second(worker1)).toThrow("range 44000-44003 is exhausted for worker 1")
    // Replacement workers use the same parallel slot after their predecessor exits.
    expect(createPortAllocator(44000, 44003)(worker1)).toBe(44001)
})

test("standalone and single-worker ports retain consecutive allocation", () => {
    const standalone = createPortAllocator(44000, 44001)
    const worker = createPortAllocator(44000, 44001)
    const info = {parallelIndex: 0, config: {workers: 1}}
    expect([standalone(), standalone(), worker(info), worker(info)]).toEqual([
        44000, 44001, 44000, 44001,
    ])
    const ephemeral = createPortAllocator(0)
    expect([ephemeral(), ephemeral(info)]).toEqual([0, 0])
    const last = createPortAllocator(65535)
    expect(last()).toBe(65535)
    expect(() => last()).toThrow("range 65535-65535 is exhausted")
})

test("invalid configured ranges and worker metadata fail before binding", () => {
    for (const base of [-1, 0.5, 65536, Number.NaN])
        expect(() => createPortAllocator(base)).toThrow("STEP_UI_TEST_PORT_BASE")
    for (const limit of [43999, 65536, Number.NaN])
        expect(() => createPortAllocator(44000, limit)).toThrow("STEP_UI_TEST_PORT_LIMIT")
    const allocate = createPortAllocator(44000)
    for (const info of [
        {parallelIndex: -1, config: {workers: 2}},
        {parallelIndex: 2, config: {workers: 2}},
        {parallelIndex: 0, config: {workers: 0}},
    ])
        expect(() => allocate(info)).toThrow("valid Playwright parallelIndex and worker count")
})

// Playwright requires destructuring even without fixture dependencies.
// eslint-disable-next-line no-empty-pattern
test("static servers retain distinct simultaneous listeners within one worker", async ({}, testInfo) => {
    const directory = await mkdtemp(join(tmpdir(), "ui-kit-ports-"))
    const servers: Awaited<ReturnType<typeof serveDist>>[] = []
    try {
        await writeFile(join(directory, "index.html"), "<main>Concurrent fixture</main>")
        servers.push(await serveDist(directory, undefined, testInfo))
        servers.push(await serveDist(directory, undefined, testInfo))
        expect(servers[0].origin).not.toBe(servers[1].origin)
        for (const server of servers)
            expect(await (await fetch(`${server.origin}/index.html`)).text()).toBe(
                "<main>Concurrent fixture</main>"
            )
    } finally {
        await Promise.all(servers.map((server) => server.close()))
        await rm(directory, {recursive: true, force: true})
    }
})
