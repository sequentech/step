// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {
    IvrAction,
    IvrCallDriver,
    IvrEmulatorApi,
    IvrEmulatorConfig,
    IvrPrompt,
} from "@sequentech/ui-essentials"

/**
 * A stand-in for the IVR emulator, for stories and tests: the real one is the IVR Lambda
 * compiled to WebAssembly, built and served by another repository. It greets the caller,
 * asks for a key, and hangs up once pressed; silence asks again. A blacklisted caller is
 * refused, as the real flow's blacklist step does.
 */
const says = (text: string): IvrPrompt => ({
    prompt_text: `<speak>${text}</speak>`,
    language: "en-US",
    voice_id: "Joanna",
})

export class FakeIvrDriver implements IvrCallDriver {
    freed = false
    private queue: IvrAction[]

    constructor(readonly config: IvrEmulatorConfig) {
        this.queue = config.blacklisted_numbers.includes(config.caller_number)
            ? [{type: "Disconnect", prompt: says("This number cannot vote by telephone.")}]
            : [{type: "Prompt", prompt: says("Welcome to the election.")}, this.ask()]
    }

    private ask(): IvrAction {
        return {
            type: "ExpectInput",
            prompt: says("Press 1 to hear your ballot."),
            valid_inputs: "1",
            max_digits: 1,
            timeout: 10,
        }
    }

    async execute(): Promise<IvrAction> {
        return this.queue.shift() ?? {type: "Noop"}
    }

    send_input(input: string): void {
        this.queue.push(
            input === "1"
                ? {
                      type: "Disconnect",
                      prompt: says(`You have ${this.config.ballot_styles.length} ballot.`),
                  }
                : this.ask()
        )
    }

    send_timeout(): void {
        this.queue.push(this.ask())
    }

    free(): void {
        this.freed = true
    }
}

/**
 * The same call behind a PIN, which the Lambda asks for by listing no valid inputs: any
 * digits, up to `max_digits`.
 */
export class FakePinIvrDriver extends FakeIvrDriver {
    private pin: "asking" | "asked" | "given" = "asking"

    async execute(): Promise<IvrAction> {
        if (this.pin !== "asking") return super.execute()
        this.pin = "asked"
        return {
            type: "ExpectInput",
            prompt: says("Enter your 8-digit PIN."),
            valid_inputs: "",
            max_digits: 8,
            timeout: 5,
        }
    }

    send_input(input: string): void {
        // Any PIN will do; the call goes on to the ordinary greeting.
        if (this.pin === "given") super.send_input(input)
        else this.pin = "given"
    }
}

export const fakeIvrEmulator: IvrEmulatorApi = {IvrEmulatorDriver: FakeIvrDriver}

/** A configuration the fake accepts: one ballot style, a caller, nothing blacklisted. */
export const fakeCallConfig = (over: Partial<IvrEmulatorConfig> = {}): IvrEmulatorConfig => ({
    caller_number: "+1234567890",
    contact_id: "00000000-0000-4000-8000-000000000001",
    tenant_id: "tenant",
    election_event_id: "event",
    election_event: JSON.stringify({id: "event"}),
    ballot_styles: [JSON.stringify({id: "style"})],
    open_elections: ["election"],
    blacklisted_numbers: [],
    ...over,
})
