// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {
    SCENARIOS,
    ScenarioId,
    scenarioSnapshot,
    SnapshotError,
    SnapshotOrigin,
    validateSnapshot,
} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {ScenarioPicker} from "../ScenarioPicker"

const rejection = (() => {
    try {
        validateSnapshot({...scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY), version: 2})
    } catch (error) {
        return error instanceof SnapshotError ? error.issues : [String(error)]
    }
    return []
})()

const meta = {
    title: "Workbench/Scenario picker",
    component: ScenarioPicker,
    args: {
        scenarios: SCENARIOS,
        snapshot: scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY),
        onSelect: fn(),
        onImport: fn(),
        onExport: fn(),
        onReset: fn(),
    },
} satisfies Meta<typeof ScenarioPicker>
export default meta
type Story = StoryObj<typeof meta>

export const Bundled: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        const body = within(canvasElement.ownerDocument.body)
        await expect(canvas.getByText("Bundled", {exact: true})).toBeVisible()
        await userEvent.click(canvas.getByRole("combobox", {name: /^Scenario/}))
        await userEvent.click(await body.findByRole("option", {name: "Kiosk voter"}))
        await expect(args.onSelect).toHaveBeenCalledWith(ScenarioId.KIOSK_VOTER)
        await userEvent.click(canvas.getByRole("button", {name: "Export snapshot"}))
        await expect(args.onExport).toHaveBeenCalledOnce()
        await userEvent.click(canvas.getByRole("button", {name: "Reset"}))
        await expect(args.onReset).toHaveBeenCalledOnce()
    },
}

export const ImportFile: Story = {
    play: async ({args, canvasElement}) => {
        const file = new File(["{}"], "snapshot.json", {type: "application/json"})
        await userEvent.upload(
            canvasElement.querySelector<HTMLInputElement>("input[type=file]")!,
            file
        )
        await expect(args.onImport).toHaveBeenCalledWith(file)
    },
}

export const Imported: Story = {
    args: {
        snapshot: {
            ...scenarioSnapshot(ScenarioId.RANKED_MULTI_CONTEST),
            provenance: {
                origin: SnapshotOrigin.EXPORTED,
                createdAt: "2026-02-01T10:00:00.000Z",
                changes: ["Budget priorities: max_votes = 2"],
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Imported", {exact: true})).toBeVisible()
        await expect(canvas.getByRole("combobox", {name: /^Scenario/})).toHaveTextContent(
            "Ranked multi-contest"
        )
    },
}

/** The issues come from the real snapshot validator. */
export const RejectedImport: Story = {
    args: {importIssues: rejection},
    play: async ({canvasElement}) => {
        const alert = within(canvasElement).getByRole("alert")
        await expect(alert).toHaveTextContent("The snapshot was not imported")
        await expect(alert).toHaveTextContent("version: expected 1, found 2")
    },
}
