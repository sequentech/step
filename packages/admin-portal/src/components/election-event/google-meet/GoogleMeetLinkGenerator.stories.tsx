// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyClipboard} from "../export-data/__stories__/ClipboardFixture"
import {GoogleMeetLinkGenerator} from "./GoogleMeetLinkGenerator"
import {pending} from "../../../../../ui-essentials/.storybook/screens"

type Props = React.ComponentProps<typeof GoogleMeetLinkGenerator> & {
    /** What the Google Meet action answers. */
    outcome: "link" | "no-link" | "failure" | "pending"
    /** Whether the browser lets the page write to the clipboard. */
    clipboard: "copied" | "refused"
}

const MEET_LINK = "https://meet.admin-story.invalid/abc-defg-hij"

let boundary: ReturnType<typeof graphqlBoundary>
let clipboard: ReturnType<typeof storyClipboard>

const meta = {
    title: "Admin/Election event/Google meet/GoogleMeetLinkGenerator",
    component: GoogleMeetLinkGenerator,
    args: {
        open: true,
        onClose: fn(),
        electionEventName: "Council event",
        outcome: "link",
        clipboard: "copied",
    },
    argTypes: {
        outcome: {control: "inline-radio", options: ["link", "no-link", "failure", "pending"]},
        clipboard: {control: "inline-radio", options: ["copied", "refused"]},
    },
    beforeEach: async ({args}) => {
        clipboard = storyClipboard(args.clipboard)
        boundary = graphqlBoundary(
            {
                GenerateGoogleMeet: () =>
                    args.outcome === "pending"
                        ? pending()
                        : args.outcome === "failure"
                          ? {errors: [new GraphQLError("Google Calendar quota exceeded")]}
                          : {
                                data: {
                                    generate_google_meet: {
                                        meet_link: args.outcome === "link" ? MEET_LINK : null,
                                    },
                                },
                            },
            },
            {schema: true}
        )
        await boundary.ready
        return clipboard.restore
    },
    render: ({outcome: _outcome, clipboard: _clipboard, ...props}) => (
        <AdminStoryProvider boundary={boundary}>
            <GoogleMeetLinkGenerator {...props} />
        </AdminStoryProvider>
    ),
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>

const openDialog = async () => {
    const dialog = await within(document.body).findByRole("dialog", {
        name: "Generate Google Meet Link",
    })
    await waitFor(() => expect(dialog).toBeVisible())
    return within(dialog)
}

/** Schedules the meeting on 1 February 2026 at 10:00 local time, for 90 minutes. */
async function schedule(dialog: ReturnType<typeof within>) {
    // As the browser's date and time pickers do, each input changes to a complete value.
    fireEvent.change(dialog.getByLabelText(/Start Date/), {target: {value: "2026-02-01"}})
    fireEvent.change(dialog.getByLabelText(/Start Time/), {target: {value: "10:00"}})
    const duration = dialog.getByRole("spinbutton", {name: /Duration/})
    await userEvent.clear(duration)
    await userEvent.type(duration, "90")
    await userEvent.type(dialog.getByRole("textbox", {name: /Description/}), "Count review")
    await userEvent.clear(dialog.getByRole("textbox", {name: "Attendee Emails"}))
    await userEvent.type(
        dialog.getByRole("textbox", {name: "Attendee Emails"}),
        "alice@example.com, bob@example.com,"
    )
}

const generationCalls = () => boundary.calls.filter(({name}) => name === "GenerateGoogleMeet")

const linkDefects = {
    expectedFailure: {
        reason: "The success message is an h6 below the dialog's h2 and the generated link field has no label.",
        a11y: ["heading-order", "label"],
    },
}

export const Populated: Story = {
    play: async ({args}) => {
        const dialog = await openDialog()
        await expect(dialog.getByRole("textbox", {name: "Meeting Title"})).toHaveValue(
            "Council event - Meeting"
        )
        await expect(dialog.getByRole("textbox", {name: "Attendee Emails"})).toHaveValue(
            "participant@example.com"
        )
        await expect(dialog.getByRole("spinbutton", {name: /Duration/})).toHaveValue(60)
        await expect(dialog.getByLabelText(/Start Date/)).not.toHaveValue("")
        await expect(dialog.getByRole("button", {name: "Generate Meet Link"})).toBeEnabled()
        expect(boundary.calls).toEqual([])
        expect(args.onClose).not.toHaveBeenCalled()
    },
}

export const GenerateAndCopyLink: Story = {
    parameters: linkDefects,
    play: async () => {
        const dialog = await openDialog()
        await schedule(dialog)
        await userEvent.click(dialog.getByRole("button", {name: "Generate Meet Link"}))
        await expect(
            await dialog.findByText("Google Meet Link Generated Successfully!")
        ).toBeVisible()
        expect(generationCalls()).toEqual([
            {
                name: "GenerateGoogleMeet",
                variables: {
                    summary: "Council event - Meeting",
                    description: "Count review",
                    // The date and time are read in the browser's time zone, which it sends too.
                    startDateTime: new Date("2026-02-01T10:00").toISOString(),
                    endDateTime: new Date("2026-02-01T11:30").toISOString(),
                    timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
                    attendeeEmails: ["alice@example.com", "bob@example.com"],
                },
                headers: {"x-hasura-role": "google-meet-link"},
            },
        ])
        await expect(dialog.getByDisplayValue(MEET_LINK)).toHaveAttribute("readonly")
        expect(dialog.queryByRole("button", {name: "Generate Meet Link"})).toBeNull()
        await userEvent.click(dialog.getByRole("button", {name: "Copy to clipboard"}))
        expect(clipboard.writeText).toHaveBeenCalledWith(MEET_LINK)
        await expect(
            await within(document.body).findByText("Link copied to clipboard!")
        ).toBeInTheDocument()
    },
}

// A refused clipboard write is only logged: the page shows no message either way.
export const CopyRefusedIsNotReported: Story = {
    args: {clipboard: "refused"},
    parameters: linkDefects,
    play: async () => {
        const dialog = await openDialog()
        await userEvent.click(dialog.getByRole("button", {name: "Generate Meet Link"}))
        await userEvent.click(await dialog.findByRole("button", {name: "Copy to clipboard"}))
        await waitFor(() => expect(clipboard.writeText).toHaveBeenCalledWith(MEET_LINK))
        expect(within(document.body).queryByText("Link copied to clipboard!")).toBeNull()
        await expect(dialog.getByDisplayValue(MEET_LINK)).toBeVisible()
    },
}

export const Generating: Story = {
    args: {outcome: "pending"},
    parameters: {
        expectedFailure: {
            reason: "The generate button's progress indicator has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async () => {
        const dialog = await openDialog()
        await userEvent.click(dialog.getByRole("button", {name: "Generate Meet Link"}))
        await expect(await dialog.findByRole("button", {name: "Generating..."})).toBeDisabled()
        expect(generationCalls()).toHaveLength(1)
        expect(dialog.queryByRole("alert")).toBeNull()
    },
}

export const GenerationFailure: Story = {
    args: {outcome: "failure"},
    play: async () => {
        const dialog = await openDialog()
        await userEvent.click(dialog.getByRole("button", {name: "Generate Meet Link"}))
        const alert = await dialog.findByRole("alert")
        await expect(alert).toHaveTextContent(
            "Failed to generate Google Meet link: Google Calendar quota exceeded"
        )
        expect(generationCalls()).toHaveLength(1)
        // The form stays available for another attempt.
        await expect(dialog.getByRole("button", {name: "Generate Meet Link"})).toBeEnabled()
        expect(dialog.queryByText("Google Meet Link Generated Successfully!")).toBeNull()
    },
}

export const LinkMissingFromAnswer: Story = {
    args: {outcome: "no-link"},
    play: async () => {
        const dialog = await openDialog()
        await userEvent.click(dialog.getByRole("button", {name: "Generate Meet Link"}))
        await expect(await dialog.findByRole("alert")).toHaveTextContent("Link is null.")
        expect(generationCalls()).toHaveLength(1)
    },
}

export const RequiredFieldsDisableGeneration: Story = {
    play: async () => {
        const dialog = await openDialog()
        await userEvent.clear(dialog.getByRole("textbox", {name: "Meeting Title"}))
        await expect(dialog.getByRole("button", {name: "Generate Meet Link"})).toBeDisabled()
        expect(boundary.calls).toEqual([])
    },
}

export const Cancel: Story = {
    play: async ({args}) => {
        const dialog = await openDialog()
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        expect(args.onClose).toHaveBeenCalledTimes(1)
        expect(boundary.calls).toEqual([])
    },
}

export const Closed: Story = {
    args: {open: false},
    play: async () => {
        expect(within(document.body).queryByRole("dialog")).toBeNull()
        expect(boundary.calls).toEqual([])
    },
}
