// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within, type Mock} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EMessagePurpose} from "@/types/messaging"
import {HttpApiSenderEditor} from "./HttpApiSenderEditor"
import {EHttpSection, IHttpApiForm, emptyHttpApiForm, withHttpExample} from "./httpApiSender"

enum EStoryForm {
    EMPTY = "EMPTY",
    EXAMPLE = "EXAMPLE",
    PROBLEMS = "PROBLEMS",
}

interface Scenario {
    form: EStoryForm
    submitted: boolean
    disabled: boolean
    onChange: Mock<(values: IHttpApiForm) => void>
}

const FORMS: Record<EStoryForm, () => IHttpApiForm> = {
    [EStoryForm.EMPTY]: emptyHttpApiForm,
    [EStoryForm.EXAMPLE]: () => ({
        ...withHttpExample(emptyHttpApiForm()),
        templateRequiredFor: [EMessagePurpose.OTP],
        approvedLanguages: {OTP: "en, tl", NOTICE: ""},
    }),
    [EStoryForm.PROBLEMS]: () => ({
        ...emptyHttpApiForm(),
        messageIdPointer: "message_id",
        conversationWindowHours: "a day",
        send: {
            url: "https://api.partner.example/v1/messages",
            headers: {Authorization: "Bearer {{credential.PRIVATE_KEY}}"},
            payload: {},
        },
        sections: {
            ...emptyHttpApiForm().sections,
            [EHttpSection.REPORTS]: {
                auth: {kind: "BASIC"},
                status: {message_id_pointer: "/id", state_pointer: "/status", states: {}},
            },
        },
    }),
}

let graphql: ReturnType<typeof graphqlBoundary>

const JSON_EDITOR_CONTRAST = {
    reason: "json-edit-react's default theme renders item counts and string values with insufficient contrast.",
    a11y: ["color-contrast"],
}

function Fixture({form, submitted, disabled, onChange}: Scenario) {
    const [values, setValues] = useState(FORMS[form])
    return (
        <AdminStoryProvider boundary={graphql}>
            <HttpApiSenderEditor
                values={values}
                submitted={submitted}
                disabled={disabled}
                onChange={(next) => {
                    onChange(next)
                    setValues(next)
                }}
            />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Settings/HttpApiSenderEditor",
    component: HttpApiSenderEditor,
    args: {form: EStoryForm.EXAMPLE, submitted: false, disabled: false, onChange: fn()},
    argTypes: {form: {control: "inline-radio", options: Object.values(EStoryForm)}},
    parameters: {expectedFailure: JSON_EDITOR_CONTRAST, widgets: ["HttpJsonSection"]},
    beforeEach: async () => {
        graphql = graphqlBoundary({})
        await graphql.ready
    },
    render: (args) => <Fixture key={args.form} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const lastChange = (onChange: Scenario["onChange"]) => onChange.mock.calls.at(-1)?.[0]

export const ViberPartner: Story = {
    parameters: {widgets: ["HttpJsonSection", "HttpPlaceholderReference"]},
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/Describes a provider by its HTTP requests/)).toBeVisible()
        await expect(canvas.getByRole("checkbox", {name: "OTPs"})).toBeChecked()
        await expect(
            canvas.getByRole("textbox", {name: "Approved template languages for OTPs"})
        ).toHaveValue("en, tl")
        await expect(
            canvas.getByRole("textbox", {name: "Message ID in the send response"})
        ).toHaveValue("/message_id")
        expect(canvas.queryByRole("alert")).toBeNull()
        await userEvent.click(canvas.getByRole("checkbox", {name: "Notices"}))
        expect(lastChange(args.onChange)?.templateRequiredFor).toEqual([
            EMessagePurpose.OTP,
            EMessagePurpose.NOTICE,
        ])
        await userEvent.click(canvas.getByRole("button", {name: "Placeholder reference"}))
        await expect(await canvas.findByText("{{named_parameters}}")).toBeVisible()
        await expect(canvas.getByText(/written as @name=value/)).toBeVisible()
    },
}

export const AddAndRemoveSections: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Add: Signed token (JWT)"}))
        expect(lastChange(args.onChange)?.sections.JWT).toEqual({
            algorithm: "RS256",
            claims: {},
            lifetime_seconds: 300,
        })
        await userEvent.click(
            canvas.getByRole("button", {name: "Remove: Delivery reports and replies"})
        )
        expect(lastChange(args.onChange)?.sections.REPORTS).toBeNull()
        await expect(
            canvas.getByRole("button", {name: "Add: Delivery reports and replies"})
        ).toBeVisible()
    },
}

export const UseTheExample: Story = {
    args: {form: EStoryForm.EMPTY, submitted: true},
    parameters: {widgets: ["HttpProblems", "HttpJsonSection"]},
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("alert")).toHaveTextContent(
            "url is required: the address of the request."
        )
        await userEvent.click(canvas.getByRole("button", {name: "Worked example: a Viber partner"}))
        await userEvent.click(await canvas.findByRole("button", {name: "Use this example"}))
        expect(lastChange(args.onChange)).toEqual(withHttpExample(emptyHttpApiForm()))
        expect(canvas.queryByRole("alert")).toBeNull()
    },
}

export const ShapeProblems: Story = {
    args: {form: EStoryForm.PROBLEMS},
    parameters: {widgets: ["HttpProblems", "HttpJsonSection"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const alerts = canvas.getAllByRole("alert").map((alert) => alert.textContent)
        expect(alerts).toEqual([
            "headers.Authorization uses a placeholder that does not exist. See the placeholder reference.",
            "payload is not a field of this section.",
            "auth.kind is not valid: kind is URL_KEY, HEADER_SECRET, HMAC_SHA256 or JWT_HS256; header is required except for URL_KEY; encoding is HEX or BASE64.",
            "status.states must map a provider status value to QUEUED, ACCEPTED, DELIVERED, FAILED or UNKNOWN; at least one is needed.",
        ])
        await expect(
            canvas.getByText(
                "Message ID in the send response must be a JSON pointer starting with /, such as /data/id."
            )
        ).toBeVisible()
        await expect(
            canvas.getByText("Conversation window (hours) must be a whole number of hours above 0.")
        ).toBeVisible()
    },
}

export const ReadOnly: Story = {
    args: {disabled: true},
    parameters: {
        expectedFailure: {
            reason:
                "Disabled fields draw their helper text in the theme's disabled grey, and " +
                "json-edit-react's default theme has insufficient contrast.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("checkbox", {name: "OTPs"})).toBeDisabled()
        expect(canvas.queryByRole("button", {name: /^Add: /})).toBeNull()
        expect(canvas.queryByRole("button", {name: /^Remove: /})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Worked example: a Viber partner"}))
        expect(canvas.queryByRole("button", {name: "Use this example"})).toBeNull()
    },
}
