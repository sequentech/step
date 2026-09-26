// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The telephone call, driven by a fake.
 *
 * The Admin Portal's emulator had no tests, and the loop is the reason it was worth
 * lifting rather than copying: `free()` on a driver still inside `execute` is a
 * crash in compiled code, and both orders are reachable by closing the panel at the
 * wrong moment. A fake driver makes those orders something a test can arrange.
 */

import {ThemeProvider} from "@mui/material/styles"
import {act, render as mount, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import React from "react"

import theme from "../services/theme"
import {IvrAction, IvrCall, IvrCallDriver, IvrPrompt} from "./IvrCall"

const render = (ui: React.ReactElement) => mount(<ThemeProvider theme={theme}>{ui}</ThemeProvider>)

const says = (text: string): IvrPrompt => ({
    prompt_text: `<speak>${text}</speak>`,
    language: "en-US",
    voice_id: "Joanna",
})

/**
 * A driver that says what it is told to, in order.
 *
 * `execute` resolves the next scripted action, and `held` lets a test keep one
 * unresolved — which is the only way to be *inside* the WebAssembly when something
 * else happens.
 */
class FakeDriver implements IvrCallDriver {
    freed = 0
    inputs: string[] = []
    timeouts = 0
    private held: ((action: IvrAction) => void) | undefined

    constructor(private script: IvrAction[]) {}

    execute(_untilIo: boolean): Promise<IvrAction> {
        const next = this.script.shift()
        if (next) {
            return Promise.resolve(next)
        }
        return new Promise((resolve) => {
            this.held = resolve
        })
    }

    /** Let go of a call that was left inside `execute`. */
    release(action: IvrAction): void {
        const held = this.held
        this.held = undefined
        held?.(action)
    }

    send_input(input: string): void {
        this.inputs.push(input)
    }

    send_timeout(): void {
        this.timeouts++
    }

    free(): void {
        this.freed++
    }
}

const hangUp = (text: string): IvrAction => ({type: "Disconnect", prompt: says(text)})

const asks = (text: string): IvrAction => ({
    type: "ExpectInput",
    prompt: says(text),
    valid_inputs: "1-9",
    max_digits: 4,
    timeout: 5,
})

describe("a call in progress", () => {
    it("shows what was said, in the order it was said", async () => {
        const driver = new FakeDriver([
            {type: "Prompt", prompt: says("Welcome to the election")},
            {type: "Noop"},
            hangUp("Goodbye"),
        ])
        render(<IvrCall start={() => driver} />)

        await screen.findByText("Welcome to the election")
        expect(screen.getByText("Goodbye")).toBeInTheDocument()
    })

    it("strips the root SSML tag and keeps what is inside it", async () => {
        // The pauses are why: somebody reading a transcript to work out why a
        // prompt sounds rushed needs to see the breaks the Lambda inserted.
        const driver = new FakeDriver([hangUp('Press one<break time="500ms"/>or two')])
        render(<IvrCall start={() => driver} />)

        await screen.findByText('Press one<break time="500ms"/>or two')
    })

    it("labels each line with the language, and names the voice on hover", async () => {
        const driver = new FakeDriver([hangUp("Adiós")])
        render(<IvrCall start={() => driver} />)

        await screen.findByText("Adiós")
        expect(screen.getByTitle("en-US, Joanna")).toHaveTextContent("EN")
    })
})

describe("what the caller can press", () => {
    it("takes only the twelve keys a telephone has", async () => {
        const driver = new FakeDriver([asks("Enter your voter id")])
        render(<IvrCall start={() => driver} />)

        const box = await screen.findByTestId("ivr-call-input")
        await userEvent.type(box, "1a2#b*")

        expect(box).toHaveValue("12#*")
    })

    it("sends the keys, then clears the box for the next prompt", async () => {
        const driver = new FakeDriver([asks("Enter your voter id")])
        render(<IvrCall start={() => driver} />)

        const box = await screen.findByTestId("ivr-call-input")
        await userEvent.type(box, "1234{Enter}")

        await waitFor(() => expect(driver.inputs).toEqual(["1234"]))
        expect(box).toHaveValue("")
    })

    it("will not send an empty press", async () => {
        const driver = new FakeDriver([asks("Enter your voter id")])
        render(<IvrCall start={() => driver} sendLabel="Press" />)

        await screen.findByTestId("ivr-call-input")
        expect(screen.getByRole("button", {name: "Press"})).toBeDisabled()
    })

    it("lets the caller's patience run out", async () => {
        const driver = new FakeDriver([asks("Enter your voter id")])
        render(<IvrCall start={() => driver} timeoutLabel="Wait" sendLabel="Press" />)

        await screen.findByTestId("ivr-call-input")
        await userEvent.click(screen.getByRole("button", {name: "Wait"}))

        expect(driver.timeouts).toBe(1)
    })

    it("takes the box away once the line is dead", async () => {
        const driver = new FakeDriver([hangUp("Goodbye")])
        render(<IvrCall start={() => driver} disconnectedLabel="The call ended" />)

        await screen.findByText("The call ended")
        expect(screen.queryByTestId("ivr-call-input")).toBeNull()
    })
})

describe("the driver's memory", () => {
    it("frees a call that hung up", async () => {
        const driver = new FakeDriver([hangUp("Goodbye")])
        render(<IvrCall start={() => driver} />)

        await screen.findByText("Goodbye")
        expect(driver.freed).toBe(1)
    })

    it("frees a call still waiting for a caller, when the panel closes", async () => {
        const driver = new FakeDriver([asks("Enter your voter id")])
        const drawn = render(<IvrCall start={() => driver} />)

        await screen.findByTestId("ivr-call-input")
        drawn.unmount()

        expect(driver.freed).toBe(1)
    })

    it("waits for the WebAssembly to return before freeing it", async () => {
        // The crash this is here to prevent: unmounting while `execute` is still
        // running would free memory the compiled code is about to write to. So
        // disposal is recorded and the release happens when the call comes back.
        const driver = new FakeDriver([])
        const drawn = render(<IvrCall start={() => driver} />)

        drawn.unmount()
        expect(driver.freed).toBe(0)

        await act(async () => {
            driver.release(hangUp("Goodbye"))
        })

        expect(driver.freed).toBe(1)
    })

    it("frees a driver once, however it got there", async () => {
        const driver = new FakeDriver([hangUp("Goodbye")])
        const drawn = render(<IvrCall start={() => driver} />)

        await screen.findByText("Goodbye")
        drawn.unmount()

        expect(driver.freed).toBe(1)
    })
})

describe("when the call cannot be placed at all", () => {
    it("says so rather than rendering an empty transcript", async () => {
        const thrown = jest.spyOn(console, "error").mockImplementation(() => undefined)
        render(
            <IvrCall
                start={() => {
                    throw new Error("no ballot styles")
                }}
            />
        )

        await screen.findByText(/no ballot styles/)
        thrown.mockRestore()
    })

    it("says so when the WebAssembly fails mid-call", async () => {
        const thrown = jest.spyOn(console, "error").mockImplementation(() => undefined)
        const driver = new FakeDriver([])
        render(<IvrCall start={() => driver} />)

        await act(async () => {
            driver.release({type: "Prompt", prompt: says("x")})
        })
        // The next `execute` never resolves, so reject it the way the WebAssembly
        // would if the flow named a phase the Lambda has no engine for.
        await act(async () => {
            driver.release(undefined as unknown as IvrAction)
        })

        await waitFor(() => expect(thrown).toHaveBeenCalled())
        thrown.mockRestore()
    })
})

describe("what the host is told", () => {
    it("reports each state, ending disconnected", async () => {
        const seen: string[] = []
        const driver = new FakeDriver([asks("Enter your voter id"), hangUp("Goodbye")])
        render(<IvrCall start={() => driver} onStatusChange={(s) => seen.push(s)} />)

        await screen.findByTestId("ivr-call-input")
        await userEvent.type(screen.getByTestId("ivr-call-input"), "1{Enter}")

        await waitFor(() => expect(seen).toContain("Disconnected"))
        expect(seen).toEqual(["Running", "ExpectingInput", "Running", "Disconnected"])
    })
})
