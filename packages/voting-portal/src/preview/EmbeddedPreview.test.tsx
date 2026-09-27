// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import {ScenarioId, scenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {EMBED_PROTOCOL, EMBED_VERSION, EmbedMessageType, embedMessage} from "./embed"
import {EmbeddedPreview} from "./EmbeddedPreview"
import {fakeCallConfig, fakeIvrEmulator} from "./fakeIvrEmulator"
import {PreviewScreen} from "./screens"
import type {VoterPreviewProps} from "./VoterPreview"

// The portal's providers and screens have their own tests; this one is about the messages.
const mockVoterPreview = jest.fn()
const mockVoterPreviewMounted = jest.fn()
jest.mock("./VoterPreview", () => {
    const {useEffect} = jest.requireActual<typeof React>("react")
    return {
        VoterPreview: (props: VoterPreviewProps) => {
            mockVoterPreview(props)
            useEffect(() => mockVoterPreviewMounted(props.language), [])
            return <>{props.children}</>
        },
    }
})
jest.mock("../components/PortalChrome", () => ({
    PortalChrome: ({children}: React.PropsWithChildren) => <div role="banner">{children}</div>,
}))
jest.mock("../routes/TenantEvent", () => {
    const {Outlet} = jest.requireActual("react-router-dom")
    return {__esModule: true, default: () => <Outlet />}
})
jest.mock("../routes/ErrorPage", () => ({ErrorPage: () => <p>error page</p>}))
// Built when the factory runs, before the JSX runtime import is initialized.
jest.mock("../appRoutes", () => {
    const {createElement} = jest.requireActual<typeof React>("react")
    const {Link} = jest.requireActual("react-router-dom")
    return {
        tenantEventRoutes: [
            {path: "election-chooser", element: createElement("p", null, "chooser")},
            {
                path: "election/:electionId/vote",
                element: createElement(Link, {to: "../review", relative: "path"}, "Next"),
            },
            {path: "election/:electionId/review", element: createElement("p", null, "review")},
        ],
    }
})

const eventPath = `/tenant/${IDS.tenant}/event/${IDS.event}`

const showMessage = (fields: Record<string, unknown> = {}) => {
    const {preview, areaId} = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
    return embedMessage({
        type: EmbedMessageType.SHOW,
        document: preview,
        areaId,
        screen: PreviewScreen.VOTE,
        ...fields,
    } as Parameters<typeof embedMessage>[0])
}

/** A parent window: what it was sent, and a way to send to the embed. */
function parent() {
    const received: {message: unknown; origin: string}[] = []
    const host = {
        postMessage: (message: unknown, origin: string) => received.push({message, origin}),
    } as unknown as Window
    const send = (data: unknown, source: unknown = host, origin = "https://architect.example") =>
        act(() => {
            window.dispatchEvent(
                new MessageEvent("message", {data, origin, source: source as Window})
            )
        })
    return {host, received, send}
}

beforeEach(() => {
    mockVoterPreview.mockReset()
    mockVoterPreviewMounted.mockReset()
    window.sessionStorage.clear()
})

test("says it is ready and waits for a document", () => {
    const {host, received} = parent()
    render(<EmbeddedPreview host={host} />)
    expect(received).toEqual([{message: embedMessage({type: EmbedMessageType.READY}), origin: "*"}])
    expect(screen.getByText("Waiting for a ballot to preview")).toBeVisible()
    // Like the portal's publication preview, the embed is a demo session.
    expect(window.sessionStorage.getItem("isDemo")).toBe("true")
})

test("opens the requested screen and reports it to the sender's origin", async () => {
    const {host, received, send} = parent()
    render(<EmbeddedPreview host={host} />)
    send(showMessage({language: "es"}))

    await screen.findByRole("link", {name: "Next"})
    expect(mockVoterPreview).toHaveBeenLastCalledWith(
        expect.objectContaining({
            language: "es",
            session: expect.objectContaining({screen: PreviewScreen.VOTE}),
        })
    )
    expect(received.at(-1)).toEqual({
        message: embedMessage({
            type: EmbedMessageType.SHOWN,
            screen: PreviewScreen.VOTE,
            path: `${eventPath}/election/${IDS.election}/vote`,
        }),
        origin: "https://architect.example",
    })
})

test("each request gets its own providers, so a language does not outlive it", async () => {
    // The portal keeps whether the voter chose a language in VoterPreview's state; a
    // request without a language must get the new document's default, not the last one.
    const {host, send} = parent()
    render(<EmbeddedPreview host={host} />)
    send(showMessage({language: "es"}))
    await screen.findByRole("link", {name: "Next"})
    send(showMessage())
    await screen.findByRole("link", {name: "Next"})
    expect(mockVoterPreviewMounted.mock.calls).toEqual([["es"], [undefined]])
})

test("reports the voter's own navigation", async () => {
    const {host, received, send} = parent()
    render(<EmbeddedPreview host={host} />)
    send(showMessage())
    fireEvent.click(await screen.findByRole("link", {name: "Next"}))
    await screen.findByText("review")
    expect(received.at(-1)?.message).toEqual(
        embedMessage({
            type: EmbedMessageType.SHOWN,
            screen: PreviewScreen.REVIEW,
            path: `${eventPath}/election/${IDS.election}/review`,
        })
    )
})

test("a new document opens a new session", async () => {
    const {host, send} = parent()
    render(<EmbeddedPreview host={host} />)
    send(showMessage())
    await screen.findByRole("link", {name: "Next"})
    const first = mockVoterPreview.mock.lastCall[0].session
    send(showMessage({screen: PreviewScreen.CHOOSER}))
    await screen.findByText("chooser")
    expect(mockVoterPreview.mock.lastCall[0].session).not.toBe(first)
})

test("ignores messages from anyone but its parent, and other protocols", () => {
    const {host, received, send} = parent()
    render(<EmbeddedPreview host={host} />)
    send(showMessage(), window)
    send({type: "webpackOk"})
    expect(mockVoterPreview).not.toHaveBeenCalled()
    expect(received).toHaveLength(1)
})

test("refuses a malformed request, saying why, to the sender", () => {
    const {host, received, send} = parent()
    render(<EmbeddedPreview host={host} />)
    send({...showMessage(), version: EMBED_VERSION + 1})
    const issues = [`version: expected ${EMBED_VERSION}, found ${EMBED_VERSION + 1}`]
    expect(received.at(-1)).toEqual({
        message: {protocol: EMBED_PROTOCOL, version: EMBED_VERSION, type: "failed", issues},
        origin: "https://architect.example",
    })
    expect(screen.getByRole("alert")).toHaveTextContent(issues[0])
})

test("reports a document the portal cannot load", async () => {
    const {host, received, send} = parent()
    render(<EmbeddedPreview host={host} />)
    send(showMessage())
    await screen.findByRole("link", {name: "Next"})
    act(() => mockVoterPreview.mock.lastCall[0].onLoadError(new Error("two invalid candidates")))
    expect(received.at(-1)?.message).toEqual(
        embedMessage({type: EmbedMessageType.FAILED, issues: ["two invalid candidates"]})
    )
})

const callMessage = (fields: Record<string, unknown> = {}) =>
    embedMessage({
        type: EmbedMessageType.CALL,
        config: fakeCallConfig(),
        emulatorUrl: "https://architect.example/wasm/ivr_emulator_wasm",
        ...fields,
    } as Parameters<typeof embedMessage>[0])

test("places a call, reporting its status to the sender's origin", async () => {
    const {host, received, send} = parent()
    const loadEmulator = jest.fn(async () => fakeIvrEmulator)
    render(<EmbeddedPreview host={host} loadEmulator={loadEmulator} />)
    send(callMessage({labels: {input: "Teclas"}}))

    expect(await screen.findByRole("textbox", {name: "Teclas"})).toBeEnabled()
    expect(loadEmulator).toHaveBeenCalledWith("https://architect.example/wasm/ivr_emulator_wasm")
    expect(mockVoterPreview).not.toHaveBeenCalled()
    expect(received.at(-1)).toEqual({
        message: embedMessage({type: EmbedMessageType.CALLING, status: "ExpectingInput"}),
        origin: "https://architect.example",
    })
})

test("a document after a call opens the voter's screens, and a call after them the call", async () => {
    const {host, send} = parent()
    render(<EmbeddedPreview host={host} loadEmulator={async () => fakeIvrEmulator} />)
    send(callMessage())
    await screen.findByText("Press 1 to hear your ballot.")
    send(showMessage())
    await screen.findByRole("link", {name: "Next"})
    expect(screen.queryByText("Press 1 to hear your ballot.")).toBeNull()
    send(callMessage())
    await screen.findByText("Press 1 to hear your ballot.")
    expect(screen.queryByRole("link", {name: "Next"})).toBeNull()
})

test("refuses a malformed call, saying why", () => {
    const {host, received, send} = parent()
    render(<EmbeddedPreview host={host} loadEmulator={async () => fakeIvrEmulator} />)
    send(callMessage({emulatorUrl: "/relative"}))
    const issues = ['emulatorUrl: expected an absolute http(s) URL, found "/relative"']
    expect(received.at(-1)?.message).toEqual(embedMessage({type: EmbedMessageType.FAILED, issues}))
    expect(screen.getByRole("alert")).toHaveTextContent(issues[0])
})
