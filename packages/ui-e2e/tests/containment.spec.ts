// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect} from "@playwright/test"
import {contain, fixture, telemetryUrl} from "./fixtures"

test("network guard permits fixture assets and blocks unexpected HTTP and WebSocket traffic", async ({
    context,
    page,
}) => {
    const violations: string[] = []
    await contain(context, violations)
    await page.goto(fixture.origins.results)
    expect(violations).toEqual([])
    await page.evaluate(async () => {
        await fetch("https://unexpected.example.invalid/forbidden").catch(() => undefined)
        const socket = new WebSocket("wss://unexpected.example.invalid/socket")
        await new Promise<void>((resolve) => {
            socket.onclose = () => resolve()
        })
    })
    expect(violations).toEqual([
        "GET https://unexpected.example.invalid/forbidden",
        "WEBSOCKET wss://unexpected.example.invalid/socket",
    ])
})

test("optional admin telemetry is served locally only for its exact image request", async ({
    context,
    page,
}) => {
    const violations: string[] = []
    const telemetry = {enabled: true, requests: [] as string[]}
    await contain(context, violations, telemetry)
    await page.goto(fixture.origins.results)
    await page.evaluate(async (expected) => {
        await new Promise<void>((resolve) => {
            const pixel = new Image()
            pixel.onload = pixel.onerror = () => resolve()
            pixel.src = expected
        })
        await fetch(expected, {method: "POST"}).catch(() => undefined)
        await fetch(expected + "&unexpected=1").catch(() => undefined)
        await fetch(expected.replace(/domain=.*/, "domain=other.invalid")).catch(() => undefined)
        await fetch(expected.replace("?domain=", "/unexpected?domain=")).catch(() => undefined)
    }, telemetryUrl)
    expect(telemetry.requests).toEqual([telemetryUrl])
    expect(violations).toEqual([
        `POST ${telemetryUrl}`,
        `GET ${telemetryUrl}&unexpected=1`,
        `GET ${telemetryUrl.replace(/domain=.*/, "domain=other.invalid")}`,
        `GET ${telemetryUrl.replace("?domain=", "/unexpected?domain=")}`,
    ])
})
