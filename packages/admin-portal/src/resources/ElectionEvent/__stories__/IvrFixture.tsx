// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A telephone-voting election event with a small IVR flow and its prompts.
import type {Sequent_Backend_Ballot_Style, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, STORY_IDS, eventRecord, storyId, type StoryRecord} from "@/__stories__/fixtures"
import type {Action, EmulatorConfig, IvrEmulatorApi, PromptInfo} from "@/services/IvrEmulator"
import {
    IVR_CONFIG_ANNOTATION,
    IVR_PHONE_NUMBER_ANNOTATION,
    IVR_PROMPTS_ANNOTATION,
} from "@/utils/ivr"

export const IVR_PHONE = "+34600000001"

/** The flow announces the two prompts it names, which become required. */
export const IVR_CONFIG = {
    flow: [
        {phase: "greeting", name: "welcome", prompt_key: "welcome_prompt"},
        {phase: "farewell", name: "goodbye", prompt_key: "goodbye_prompt"},
    ],
}

export const IVR_PROMPTS: Record<string, Record<string, string>> = {
    en: {welcome_prompt: "Welcome to the council vote", menu_hint: "Press 1 to continue"},
    es: {welcome_prompt: "Bienvenido a la votación", menu_hint: "Pulse 1 para continuar"},
}

export interface IvrEventOptions {
    /** Whether the event has the IVR configuration, phone number and prompts. */
    configured?: boolean
    /** Replaces the stored prompts annotation. */
    prompts?: Record<string, Record<string, string>>
}

export function ivrEvent({
    configured = true,
    prompts = IVR_PROMPTS,
}: IvrEventOptions = {}): StoryRecord<Sequent_Backend_Election_Event> {
    return eventRecord(undefined, {
        voting_channels: {online: true, kiosk: false, early_voting: false, telephone: true},
        annotations: configured
            ? {
                  [IVR_CONFIG_ANNOTATION]: JSON.stringify(IVR_CONFIG),
                  [IVR_PHONE_NUMBER_ANNOTATION]: IVR_PHONE,
                  [IVR_PROMPTS_ANNOTATION]: JSON.stringify(prompts),
              }
            : {},
    })
}

/** The JSON editor's default theme greys its item counts and values below AA contrast. */
export const jsonEditorDefects = {
    expectedFailure: {
        reason: "json-edit-react's default theme renders item counts and string values with insufficient contrast.",
        a11y: ["color-contrast"],
    },
}

/** The unnamed icon button of a row that shows the MUI icon with this test ID. */
export const iconButton = (row: HTMLElement, icon: string) =>
    row.querySelector<HTMLElement>(`[data-testid="${icon}"]`)?.closest("button") ?? null

export const BALLOT_EML = '{"id":"council-ballot"}'

/** The published ballot style of the first election in the first area. */
export const ballotStyleRecord: StoryRecord<Sequent_Backend_Ballot_Style> = {
    id: storyId(9, 1),
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    election_id: STORY_IDS.election,
    area_id: STORY_IDS.area,
    ballot_publication_id: storyId(9, 2),
    ballot_eml: BALLOT_EML,
    created_at: FIXED_TIME,
    deleted_at: null,
}

export interface IvrScript {
    /** Actions of the call before the first input. */
    start: Action[]
    /** Actions after each DTMF input, by input. */
    input: (value: string) => Action[]
    /** Actions after a timeout. */
    timeout: Action[]
    /** Makes executing the call fail with this message. */
    failure?: string
}

/** What the emulator driver received: its configuration and the caller's inputs. */
export interface IvrSession {
    configs: EmulatorConfig[]
    inputs: string[]
    freed: number
}

const prompt = (text: string, language = "en-US"): PromptInfo => ({
    prompt_text: `<speak>${text}</speak>`,
    language,
    voice_id: "story-voice",
})

/** A call that greets, asks for the voter ID, and hangs up after it or retries on timeout. */
export const IVR_SCRIPT: IvrScript = {
    start: [
        {type: "Prompt", prompt: prompt("Welcome to the council vote")},
        {type: "Noop"},
        {
            type: "ExpectInput",
            prompt: prompt("Enter your voter ID"),
            valid_inputs: "0-9",
            max_digits: 3,
            timeout: 10,
        },
    ],
    input: (value) => [
        {type: "Prompt", prompt: prompt(`Voter ${value} accepted`)},
        {type: "Disconnect", prompt: prompt("Adiós", "es-ES")},
    ],
    timeout: [
        {
            type: "ExpectInput",
            prompt: prompt("No input received, enter your voter ID"),
            valid_inputs: "0-9",
            max_digits: 3,
            timeout: 10,
        },
    ],
}

/**
 * A call that asks for a PIN, which any digits answer: the Lambda then lists no
 * valid inputs. Its prompts are SSML with a pause and a part in another language,
 * as the Lambda writes them.
 */
export const IVR_PIN_SCRIPT: IvrScript = {
    start: [
        {
            type: "ExpectInput",
            prompt: {
                prompt_text:
                    '<speak>Enter your PIN<break time="500ms"/>then press hash. ' +
                    '<lang xml:lang="es-ES">O marque su PIN</lang></speak>',
                language: "en-US",
                voice_id: "story-voice",
            },
            valid_inputs: "",
            max_digits: 8,
            timeout: 10,
        },
    ],
    input: () => [{type: "Disconnect", prompt: prompt("PIN accepted")}],
    timeout: [],
}

let current: {script: IvrScript; session: IvrSession} | undefined

/**
 * The emulator's WASM driver, replaced by a scripted call: each `execute`
 * returns the next action queued by the start of the call, an input or a timeout.
 */
class StoryIvrDriver {
    private queue: Action[]
    private readonly script: IvrScript
    private readonly session: IvrSession

    constructor(config: EmulatorConfig) {
        if (!current) throw new Error("No IVR story session")
        this.script = current.script
        this.session = current.session
        this.session.configs.push(config)
        this.queue = [...this.script.start]
    }

    free() {
        this.session.freed += 1
    }

    [Symbol.dispose]() {
        this.free()
    }

    attributes() {
        return {}
    }

    async execute(_untilIo: boolean): Promise<Action> {
        if (this.script.failure) throw new Error(this.script.failure)
        return this.queue.shift() ?? {type: "Noop"}
    }

    send_input(input: string) {
        this.session.inputs.push(input)
        this.queue.push(...this.script.input(input))
    }

    send_timeout() {
        this.queue.push(...this.script.timeout)
    }
}

/** An emulator API whose driver plays `script`; the returned session records its use. */
export function ivrEmulatorApi(script: IvrScript = IVR_SCRIPT): {
    api: IvrEmulatorApi
    session: IvrSession
} {
    const session: IvrSession = {configs: [], inputs: [], freed: 0}
    current = {script, session}
    return {api: {IvrEmulatorDriver: StoryIvrDriver}, session}
}
