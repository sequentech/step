// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {initCore} from "@sequentech/ui-core"
import {ScenarioId, scenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {PipelineStatus, PipelineStep} from "voting-portal/src/preview/ballotPipeline"
import {loadPreviewSnapshot, sampleSelection} from "voting-portal/src/preview/session"
import {store} from "voting-portal/src/store/store"
import {ComputationKind, PipelinePanel, WASM_RUNNER, type PipelineInput} from "../PipelinePanel"

const meta = {
    title: "Workbench/Pipeline panel",
    component: PipelinePanel,
    args: {runner: WASM_RUNNER, wasmReady: true},
    // The production loader provides the ballot; the sample selection marks Alice.
    loaders: [
        async () => {
            await initCore()
            loadPreviewSnapshot(scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY), store.dispatch)
            const ballotStyle = store.getState().ballotStyles[IDS.election]!
            return {input: {ballotStyle, selection: sampleSelection(ballotStyle)}}
        },
    ],
    render: (args, {loaded}) => (
        <PipelinePanel {...args} input={(loaded as {input: PipelineInput}).input} />
    ),
} satisfies Meta<typeof PipelinePanel>
export default meta
type Story = StoryObj<typeof meta>

const run = async (canvasElement: HTMLElement) => {
    const canvas = within(canvasElement)
    await userEvent.click(canvas.getByRole("button", {name: "Run ballot pipeline"}))
    return canvas.getByRole("table", {name: "Pipeline steps"})
}

/** The actual sequent-core encrypts, hashes and decodes the ballot. */
export const SequentCore: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("sequent-core WASM")).toBeVisible()
        await expect(canvas.getByText("Council representative: Alice Example (0)")).toBeVisible()
        const rows = within(await run(canvasElement))
            .getAllByRole("row")
            .slice(1)
        await expect(rows.map((row) => row.textContent)).toEqual([
            expect.stringMatching(/^interpretpassed0 errors and 0 alerts in 1 contests/),
            expect.stringMatching(/^checkpassedNext allowed, without a confirmation dialog/),
            expect.stringMatching(/^encryptpassedAuditable ballot version \d+/),
            expect.stringMatching(/^hashpassed[0-9a-f]{64}/),
            expect.stringMatching(/^decodepassedThe decoded ballot matches the selection/),
        ])
        await expect(
            canvas.getByText("Decoded: Council representative: Alice Example (0)")
        ).toBeVisible()
    },
}

/** Mocked computation: a fixed report with a failed encryption, not sequent-core. */
export const MockedEncryptionFailure: Story = {
    args: {
        runner: {
            kind: ComputationKind.MOCK,
            run: () => ({
                steps: [
                    [PipelineStep.INTERPRET, PipelineStatus.PASSED, "0 errors and 0 alerts"],
                    [PipelineStep.CHECK, PipelineStatus.PASSED, "Next allowed"],
                    [PipelineStep.ENCRYPT, PipelineStatus.FAILED, "unsupported policy"],
                    [PipelineStep.HASH, PipelineStatus.SKIPPED, ""],
                    [PipelineStep.DECODE, PipelineStatus.SKIPPED, ""],
                ].map(([step, status, detail]) => ({
                    step: step as PipelineStep,
                    status: status as PipelineStatus,
                    detail,
                    durationMs: 0,
                })),
            }),
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Mocked computation")).toBeVisible()
        const table = within(await run(canvasElement))
        await expect(table.getByRole("row", {name: /encrypt/})).toHaveTextContent(
            "failedunsupported policy"
        )
    },
}

export const WaitingForSequentCore: Story = {
    args: {wasmReady: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Loading sequent-core…")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Run ballot pipeline"})).toBeDisabled()
    },
}

export const NoBallot: Story = {
    render: (args) => <PipelinePanel {...args} input={undefined} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Open an election screen to load a ballot.")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Run ballot pipeline"})).toBeDisabled()
    },
}
