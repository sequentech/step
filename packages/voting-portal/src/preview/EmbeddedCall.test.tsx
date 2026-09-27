// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {IvrEmulatorError, type IvrEmulatorApi} from "@sequentech/ui-essentials"
import {EmbedMessageType, type CallRequest, type EmbedReply} from "./embed"
import {DEFAULT_CALL_LABELS, EmbeddedCall} from "./EmbeddedCall"
import {fakeCallConfig, FakeIvrDriver} from "./fakeIvrEmulator"

const EMULATOR = "https://architect.example/wasm/ivr_emulator_wasm"

afterEach(() => jest.restoreAllMocks())

function placed(
    request: Partial<CallRequest> = {},
    load?: (url: string) => Promise<IvrEmulatorApi>
) {
    const replies: EmbedReply[] = []
    const loads: string[] = []
    const drivers: FakeIvrDriver[] = []
    const api: IvrEmulatorApi = {
        IvrEmulatorDriver: class extends FakeIvrDriver {
            constructor(...args: ConstructorParameters<typeof FakeIvrDriver>) {
                super(...args)
                drivers.push(this)
            }
        },
    }
    const view = render(
        <EmbeddedCall
            request={{config: fakeCallConfig(), emulatorUrl: EMULATOR, ...request}}
            reply={(message) => replies.push(message)}
            load={
                load ??
                (async (url) => {
                    loads.push(url)
                    return api
                })
            }
        />
    )
    const statuses = () =>
        replies.flatMap((reply) => (reply.type === EmbedMessageType.CALLING ? [reply.status] : []))
    return {...view, replies, loads, drivers, statuses}
}

test("places the call against the emulator it is sent, and reports where the call is", async () => {
    const {loads, statuses, drivers} = placed()
    expect(screen.getByRole("status")).toHaveTextContent(DEFAULT_CALL_LABELS.connecting)

    await screen.findByText("Press 1 to hear your ballot.")
    expect(loads).toEqual([EMULATOR])
    expect(screen.getByText("Welcome to the election.")).toBeVisible()
    expect(drivers[0].config).toEqual(fakeCallConfig())
    expect(statuses()[0]).toBe("loading")
    expect(statuses().at(-1)).toBe("ExpectingInput")

    await userEvent.type(screen.getByRole("textbox", {name: "Keys to press"}), "1")
    await userEvent.click(screen.getByRole("button", {name: "Press these keys"}))
    await screen.findByText("You have 1 ballot.")
    expect(screen.getByText("The call ended.")).toBeVisible()
    expect(statuses().at(-1)).toBe("Disconnected")
    expect(drivers[0].freed).toBe(true)
})

test("speaks the framing tool's words, the keypad's name included", async () => {
    placed({
        labels: {
            input: "Teclas",
            placeholder: "Hasta {{maxDigits}} de {{validInputs}} en {{timeout}} s",
            send: "Pulsar",
            timeout: "Esperar",
            disconnected: "Fin",
        },
    })
    const keypad = await screen.findByRole("textbox", {name: "Teclas"})
    expect(keypad).toHaveAttribute("placeholder", "Hasta 1 de 1 en 10 s")
    await userEvent.click(screen.getByRole("button", {name: "Esperar"}))
    await userEvent.type(keypad, "1")
    await userEvent.click(screen.getByRole("button", {name: "Pulsar"}))
    await screen.findByText("Fin")
})

test("keeps the keypad named after a call that fails without a prompt to answer", async () => {
    // IvrCall drops the placeholder when no input is expected, so the keypad's name has
    // to come from somewhere else (step ed59922104).
    const failing: IvrEmulatorApi = {
        IvrEmulatorDriver: class extends FakeIvrDriver {
            async execute(): Promise<never> {
                throw new Error("the Lambda panicked")
            }
        },
    }
    jest.spyOn(console, "error").mockImplementation(() => undefined)
    placed({}, async () => failing)
    expect(await screen.findByRole("alert")).toHaveTextContent("the Lambda panicked")
    const keypad = screen.getByRole("textbox", {name: "Keys to press"})
    expect(keypad).toBeDisabled()
    expect(keypad).not.toHaveAttribute("placeholder")
})

test("an emulator that is not served is absent, not a failure", async () => {
    const {replies} = placed({}, async () => {
        throw new IvrEmulatorError("fetch", "response was 404")
    })
    await screen.findByText(`No telephone emulator is served at ${EMULATOR}.`)
    expect(replies.at(-1)).toEqual({type: EmbedMessageType.CALLING, status: "absent"})
})

test("an emulator that fails to start is reported", async () => {
    const {replies} = placed({}, async () => {
        throw new IvrEmulatorError("init", "init call failed")
    })
    expect(await screen.findByRole("alert")).toHaveTextContent("init call failed")
    expect(replies.at(-1)).toEqual({
        type: EmbedMessageType.FAILED,
        issues: ['Ivr emulator "init" failed: init call failed'],
    })
})
