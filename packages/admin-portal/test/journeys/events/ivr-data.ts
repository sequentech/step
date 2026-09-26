// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {electionEvent, EVENT_ID, FIXED_TIME, TENANT_ID, type Row} from "./data"

export const IVR_CONFIG =
    '{"flow":[{"phase":"welcome","name":"greeting","prompt_key":"welcome"},{"phase":"menu","name":"main","prompt_key":"menu"}],"voice":"Joanna"}'
export const IVR_PROMPTS =
    '{"en":{"welcome":"Welcome to the council election","extra":"Please hold","closing":"Goodbye"},"es":{"welcome":"Bienvenido a la elección del consejo","extra":"Espere, por favor"}}'
export const IVR_PHONE = "+15550001111"
export const IVR_ANNOTATIONS = {
    "ivr:config": IVR_CONFIG,
    "ivr:phone-number": IVR_PHONE,
    "ivr:prompts": IVR_PROMPTS,
}

/** An event that shows the IVR tab: telephone voting on, English and Spanish enabled. */
export function ivrEvent(annotations: Record<string, string> = IVR_ANNOTATIONS): Row {
    return electionEvent(
        {
            voting_channels: {online: true, kiosk: false, early_voting: false, telephone: true},
            annotations,
        },
        {language_conf: {enabled_language_codes: ["en", "es"], default_language_code: "en"}}
    )
}

export function blocklistEntry(id: string, phone: string, reason: string | null): Row {
    return {
        id,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        phone_e164: phone,
        reason,
        created_by: IDS.voter,
        created_at: "2026-01-10T09:30:00Z",
        updated_at: "2026-01-10T09:30:00Z",
    }
}

export const NORTH_AREA = {
    id: IDS.area,
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    name: "North district",
    description: null,
    type: null,
    parent_id: null,
    annotations: {},
    labels: {},
    presentation: {},
    created_at: FIXED_TIME,
    last_updated_at: FIXED_TIME,
}

export function election(id: string, name: string): Row {
    return {
        id,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name,
        alias: null,
        description: null,
        presentation: {i18n: {en: {name}}},
        status: {},
        contests: [],
        contests_aggregate: {aggregate: {count: 0}, nodes: []},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
}

export function ballotStyle(id: string, electionId: string, ballotEml: string | null): Row {
    return {
        id,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: electionId,
        area_id: IDS.area,
        ballot_publication_id: "70000000-0000-4000-8000-000000000001",
        ballot_eml: ballotEml,
        ballot_signature: null,
        status: null,
        annotations: {},
        labels: {},
        deleted_at: null,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
}

export interface PromptInfo {
    prompt_text: string
    language: string
    voice_id: string
}

export type EmulatorStep =
    | {type: "Prompt"; prompt: PromptInfo}
    | {
          type: "ExpectInput"
          prompt: PromptInfo
          valid_inputs: string
          max_digits: number
          timeout: number
      }
    | {type: "Disconnect"; prompt: PromptInfo}
    | {type: "Noop"}
    /** Not an emulator action: makes the scripted `execute` call reject with `message`. */
    | {type: "Fail"; message: string}

/** What the call does once it connects, after each DTMF input and after a timeout. */
export interface EmulatorScript {
    connect: EmulatorStep[]
    input: Record<string, EmulatorStep[]>
    timeout: EmulatorStep[]
}

export interface EmulatorLog {
    binaries: string[]
    configs: Record<string, unknown>[]
    inputs: string[]
    timeouts: number
    freed: number
}

// The wasm-bindgen module is not part of the production bundle, so the journeys load
// this stand-in from the same URL. It replays the script and records what the portal
// hands to the driver.
function emulatorModule(script: EmulatorScript, failInit: boolean): string {
    return `
const script = ${JSON.stringify(script)};
const log = {binaries: [], configs: [], inputs: [], timeouts: 0, freed: 0};
window.__ivrEmulator = log;
export default async function load({module_or_path}) {
    log.binaries.push(new URL(module_or_path.url).pathname);
    if (${failInit}) throw new Error("invalid wasm binary");
}
export function init() {}
export class IvrEmulatorDriver {
    constructor(config) {
        log.configs.push(config);
        this.steps = [...script.connect];
    }
    async execute() {
        const step = this.steps.shift();
        if (!step) throw new Error("The scripted call has no more steps");
        if (step.type === "Fail") throw new Error(step.message);
        return step;
    }
    send_input(input) {
        log.inputs.push(input);
        this.steps = [...(script.input[input] ?? [])];
    }
    send_timeout() {
        log.timeouts += 1;
        this.steps = [...script.timeout];
    }
    free() {
        log.freed += 1;
    }
}
`
}

/**
 * Serves the scripted emulator module and a placeholder binary. The module response waits
 * for `ready`, so a test can look at the loading state first.
 */
export async function serveEmulator(
    page: Page,
    script: EmulatorScript,
    {ready = Promise.resolve(), failInit = false}: {ready?: Promise<void>; failInit?: boolean} = {}
) {
    await page.route("**/wasm/ivr_emulator_wasm.js", async (route) => {
        await ready
        await route.fulfill({
            contentType: "text/javascript",
            body: emulatorModule(script, failInit),
        })
    })
    await page.route("**/wasm/ivr_emulator_wasm_bg.wasm", (route) =>
        route.fulfill({
            contentType: "application/wasm",
            body: Buffer.from([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]),
        })
    )
}

export function emulatorLog(page: Page): Promise<EmulatorLog> {
    return page.evaluate(() => (window as unknown as {__ivrEmulator: EmulatorLog}).__ivrEmulator)
}
