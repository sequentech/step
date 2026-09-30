// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {createKcPageStory} from "../KcPageStory"
import {expectStickyActions} from "../scanovate/stickyActions"

const {KcPageStory} = createKcPageStory({pageId: "scanovate-confirmation.ftl"})

const meta = {
    title: "Keycloak/Scanovate confirmation",
    component: KcPageStory,
} satisfies Meta<typeof KcPageStory>

export default meta

type Story = StoryObj<typeof meta>

export const Details: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Check the details from your ID"})
        ).toBeVisible()
        await expect(canvas.getByText("Step 4 of 4 · Confirm")).toBeVisible()
        await expect(canvas.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "4")
        await expect(canvas.getByText("January 1, 1990")).toBeVisible()
        await expect(canvas.getByText("ID number")).toBeVisible()
        await expect(canvas.getByText("Driver’s License", {selector: "dd"})).toBeVisible()
        const confirm = canvas.getByRole("button", {name: "Confirm and enroll"})
        await expect(confirm).toHaveAttribute("name", "action")
        await expect(confirm).toHaveAttribute("value", "confirm")
        await expectStickyActions(confirm)
        await expect(canvas.getByRole("button", {name: "Scan my ID again"})).toHaveAttribute(
            "value",
            "retry"
        )
    },
}

// The browser only sends the clicked button's name and value if it's still enabled when it
// collects the form, after the submit handlers have run.
export const ConfirmSubmitsTheAction: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const submitted: (string | null)[] = []
        const onSubmit = (event: SubmitEvent) => {
            event.preventDefault()
            const form = event.target as HTMLFormElement
            submitted.push(new FormData(form, event.submitter).get("action") as string | null)
        }
        window.addEventListener("submit", onSubmit)
        try {
            await userEvent.click(await canvas.findByRole("button", {name: "Confirm and enroll"}))
            await expect(submitted).toEqual(["confirm"])
        } finally {
            window.removeEventListener("submit", onSubmit)
        }
    },
}

export const UnknownDocumentType: Story = {
    args: {kcContext: {documentType: "default"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText(
                "We read these details from your ID. They can’t be changed here; if something is wrong, scan your ID again."
            )
        ).toBeVisible()
        await expect(canvas.queryByText("Driver’s License")).toBeNull()
    },
}

export const Spanish: Story = {
    args: {locale: "es"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {
                level: 1,
                name: "Revise los datos de su documento",
            })
        ).toBeVisible()
        await expect(canvas.getByText("1 de enero de 1990")).toBeVisible()
    },
}

// As voters see it, in the voting portal's theme, for walkthroughs.
export const Voting: Story = {
    args: {kcContext: {themeName: "sequent-ui-voting"}},
}
