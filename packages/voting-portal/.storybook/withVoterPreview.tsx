// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Decorator} from "@storybook/react-vite"
import {
    ScenarioChannel,
    scenarioSnapshot,
    type ScenarioId,
    type ScenarioSnapshot,
} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {VoterPreview} from "../src/preview/VoterPreview"
import type {PreviewScreen} from "../src/preview/screens"

/** `parameters.voterPreview`: the scenario loaded into the production store for the story. */
export interface VoterPreviewParameters {
    scenario: ScenarioId
    /** Prepares the state of this screen, e.g. an encrypted ballot for review. */
    screen: PreviewScreen
    /** Derives a state variant, such as an empty area, from the scenario snapshot. */
    snapshot?: (snapshot: ScenarioSnapshot) => ScenarioSnapshot
}

/** Toolbar value of `globals.voterChannel` that keeps each scenario's own channel. */
export const SCENARIO_CHANNEL = "scenario"

const isChannel = (value: unknown): value is ScenarioChannel =>
    Object.values(ScenarioChannel).includes(value as ScenarioChannel)

const PreviewHost: React.FC<
    React.PropsWithChildren<{options: VoterPreviewParameters; channel: unknown}>
> = ({options, channel, children}) => {
    const [session] = useState(() => {
        const base = scenarioSnapshot(options.scenario)
        const snapshot = options.snapshot?.(base) ?? base
        return {
            snapshot: isChannel(channel) ? {...snapshot, channel} : snapshot,
            screen: options.screen,
        }
    })
    return <VoterPreview session={session}>{children}</VoterPreview>
}

/** Mounts the shared voter preview for stories that set `parameters.voterPreview`. */
export const withVoterPreview: Decorator = (Story, {parameters, globals, id}) => {
    const options: VoterPreviewParameters | undefined = parameters.voterPreview
    if (!options) return <Story />
    return (
        <PreviewHost
            key={`${id}:${globals.voterChannel}`}
            options={options}
            channel={globals.voterChannel}
        >
            <Story />
        </PreviewHost>
    )
}
