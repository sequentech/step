// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Preview} from "@storybook/react-vite"
import {ScenarioChannel} from "@sequentech/ui-test-kit/fixtures/scenarios"
import preview from "../../ui-essentials/.storybook/preview"
import {initializePreviewLanguages} from "../src/preview/context"
import {SCENARIO_CHANNEL, withVoterPreview} from "./withVoterPreview"

initializePreviewLanguages("en")

export default {
    ...preview,
    // First, so the voter preview sits inside the shared router, theme and language.
    decorators: [withVoterPreview, ...[preview.decorators ?? []].flat()],
    globalTypes: {
        ...preview.globalTypes,
        voterChannel: {
            description: "Channel of the scenario voter",
            toolbar: {
                icon: "user",
                items: [
                    {value: SCENARIO_CHANNEL, title: "Scenario channel"},
                    {value: ScenarioChannel.ONLINE, title: "Online voter"},
                    {value: ScenarioChannel.KIOSK, title: "Kiosk voter"},
                ],
            },
        },
    },
    initialGlobals: {...preview.initialGlobals, voterChannel: SCENARIO_CHANNEL},
} satisfies Preview
