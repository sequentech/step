// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {createKcPageStory} from "../KcPageStory"

const {KcPageStory} = createKcPageStory({pageId: "registration-manual-finish.ftl"})
const {KcPageStory: Enrolled} = createKcPageStory({pageId: "registration-finish.ftl"})
const {KcPageStory: Rejected} = createKcPageStory({pageId: "registration-rejected-finish.ftl"})
const {KcPageStory: Validated} = createKcPageStory({pageId: "message-finish.ftl"})

const meta = {
    title: "Keycloak/Enrollment finish",
    component: KcPageStory,
} satisfies Meta<typeof KcPageStory>

export default meta

type Story = StoryObj<typeof meta>

async function expectFinished(canvasElement: HTMLElement, outcome: string) {
    const canvas = within(canvasElement)
    // Every step of the enrollment is done.
    const progress = canvas.getByRole("progressbar", {name: "Enrollment progress"})
    await expect(progress).toHaveAttribute("aria-valuenow", "4")
    await expect(progress).toHaveAttribute("aria-valuetext", outcome)
    await expect(progress.querySelectorAll(".done")).toHaveLength(4)
    await expect(canvas.getByText(outcome)).toBeVisible()
    await expect(canvas.getByRole("link", {name: "Click here"})).toHaveAttribute(
        "id",
        "loginContinueLink"
    )
}

export const ManualVerification: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Manual Verification required"})
        ).toBeVisible()
        await expectFinished(canvasElement, "Enrollment under review")
        await expect(canvas.getByText(/some required information is missing/)).toBeVisible()
        const mismatches = within(
            canvas.getByRole("region", {
                name: "The following fields did not match in the registry:",
            })
        )
        await expect(mismatches.getByText("SANTOS")).toBeVisible()
        // A field the voter left empty.
        await expect(mismatches.getByText("Date of birth")).toBeVisible()
        await expect(mismatches.getByText("Empty")).toBeVisible()
        await expect(
            canvas.getByText(/Please expect to receive SMS or email for further instructions/)
        ).toBeVisible()
    },
}

// No voter in the registry matched: no field was compared, there's none to list.
export const NoMatchingVoter: Story = {
    args: {kcContext: {enrollmentOutcome: {reason: "NO_VOTER"}}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByText(/does not match any existing user/)).toBeVisible()
        await expect(canvas.queryByRole("region")).toBeNull()
    },
}

export const Congratulations: Story = {
    render: (args) => <Enrolled locale={args.locale} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Congratulations!"})
        ).toBeVisible()
        await expectFinished(canvasElement, "Enrollment complete")
        await expect(canvas.getByText(/You are successfully validated/)).toBeVisible()
        await expect(canvas.queryByRole("region")).toBeNull()
    },
}

export const Disapproved: Story = {
    render: (args) => <Rejected locale={args.locale} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Application Disapproved"})
        ).toBeVisible()
        await expectFinished(canvasElement, "Enrollment not approved")
        await expect(canvas.getByText(/already completed the enrollment process/)).toBeVisible()
    },
}

export const AlreadyValidated: Story = {
    render: (args) => <Validated locale={args.locale} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "Register"})
        await expectFinished(canvasElement, "Enrollment")
        await expect(canvas.getByText(/Your record has already been validated/)).toBeVisible()
    },
}

export const Spanish: Story = {
    args: {locale: "es"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {
                level: 1,
                name: "Se requiere verificación manual",
            })
        ).toBeVisible()
        await expect(canvas.getByText("Inscripción en revisión")).toBeVisible()
        await expect(canvas.getByText("Vacío")).toBeVisible()
    },
}

// As voters see it, in the voting portal's theme, for walkthroughs.
export const Voting: Story = {
    args: {kcContext: {themeName: "sequent-ui-voting"}},
}
