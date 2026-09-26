// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A telephone-voting election event with a small IVR flow and its prompts.
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {eventRecord, type StoryRecord} from "@/__stories__/fixtures"
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
