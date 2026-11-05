// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {EOverVotePolicy, initCore} from "@sequentech/ui-core"
import {ScenarioId, scenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {validateSelection, type SelectionValidation} from "voting-portal/src/preview/ballotPipeline"
import {loadPreviewSnapshot} from "voting-portal/src/preview/session"
import {store, type RootState} from "voting-portal/src/store/store"
import {WorkbenchEventKind} from "../../state"
import {StateInspector} from "../StateInspector"

const snapshot = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)

const meta = {
    title: "Workbench/State inspector",
    component: StateInspector,
    args: {
        snapshot,
        overrides: {},
        location: `/tenant/${IDS.tenant}/event/${IDS.event}/election/${IDS.election}/vote`,
        production: store.getState(),
        contestNames: {[IDS.contest]: "Council representative"},
        events: [
            {id: 1, kind: WorkbenchEventKind.SESSION, message: "Opened simple-plurality at vote"},
            {id: 2, kind: WorkbenchEventKind.NETWORK, message: "Blocked POST https://example.org"},
        ],
        sequentCore: {
            directory: "/workspaces/step/packages/node_modules/sequent-core",
            wasmBytes: 1994283,
            wasmSha256: "0".repeat(64),
            modifiedAt: "2026-01-15T12:00:00.000Z",
        },
    },
    // The production loader fills the store and the real sequent-core validates its selection.
    loaders: [
        async () => {
            await initCore()
            loadPreviewSnapshot(snapshot, store.dispatch)
            const style = store.getState().ballotStyles[IDS.election]!
            const selection = store.getState().ballotSelections[IDS.election]!
            return {production: store.getState(), validation: validateSelection(style, selection)}
        },
    ],
    render: (args, {loaded}) => {
        const {production, validation} = loaded as {
            production: RootState
            validation: SelectionValidation
        }
        return <StateInspector {...args} production={production} validation={validation} />
    },
} satisfies Meta<typeof StateInspector>
export default meta
type Story = StoryObj<typeof meta>

export const Loaded: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const body = within(canvasElement.ownerDocument.body)
        await expect(canvas.getByRole("region", {name: "Snapshot"})).toHaveTextContent(
            "simple-plurality"
        )
        // An empty plurality ballot is an undervote the voting screen blocks.
        await expect(canvas.getByText("Next blocked")).toBeVisible()
        await expect(canvas.getByLabelText("Production store JSON")).toHaveTextContent(IDS.alice)
        await userEvent.click(canvas.getByRole("combobox", {name: /^Slice/}))
        await userEvent.click(await body.findByRole("option", {name: "elections"}))
        await expect(canvas.getByLabelText("Production store JSON")).toHaveTextContent(
            "Community Council"
        )
        await expect(canvas.getByRole("list", {name: "Workbench events"})).toHaveTextContent(
            "Blocked POST https://example.org"
        )
    },
}

export const Overridden: Story = {
    args: {
        overrides: {[IDS.contest]: {over_vote_policy: EOverVotePolicy.ALLOWED}},
        snapshot: {
            ...snapshot,
            provenance: {
                ...snapshot.provenance,
                changes: ["Council representative: over_vote_policy = allowed"],
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("list", {name: "Snapshot changes"})).toHaveTextContent(
            "over_vote_policy = allowed"
        )
        await expect(canvas.getByLabelText("Policy overrides JSON")).toHaveTextContent(
            '"over_vote_policy": "allowed"'
        )
    },
}

export const NoBallot: Story = {
    render: (args) => <StateInspector {...args} validation={undefined} />,
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No ballot is loaded.")).toBeVisible()
    },
}

export const ValidationError: Story = {
    render: (args) => (
        <StateInspector {...args} validation={new Error("unsupported contest encryption")} />
    ),
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("alert")).toHaveTextContent(
            "unsupported contest encryption"
        )
    },
}
