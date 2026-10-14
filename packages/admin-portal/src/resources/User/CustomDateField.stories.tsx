// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {RecordContextProvider, type RaRecord} from "react-admin"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import CustomDateField from "./CustomDateField"

interface Scenario {
    /** The list row the field reads; none outside a list. */
    record?: RaRecord
    base: string
    source: string
}

const applicant = (dateOfBirth: unknown): RaRecord => ({
    id: "99999999-9999-4999-8999-999999999991",
    applicant_data: {dateOfBirth},
})

const meta = {
    title: "Admin/User/CustomDateField",
    component: CustomDateField,
    args: {record: applicant("1990-05-17"), base: "applicant_data", source: "dateOfBirth"},
    argTypes: {record: {control: "object"}},
    render: ({record, base, source}) => (
        <RecordContextProvider value={record}>
            <CustomDateField base={base} source={source} label="Date of birth" emptyText="-" />
        </RecordContextProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const FormattedDate: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("May 17, 1990")).toBeVisible()
    },
}

export const FirstOfSeveralValues: Story = {
    args: {
        record: {
            id: "88888888-8888-4888-8888-888888888881",
            attributes: {birthdate: ["1985-02-01"]},
        },
        base: "attributes",
        source: "birthdate",
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Feb 1, 1985")).toBeVisible()
    },
}

export const MissingDate: Story = {
    args: {record: applicant(null)},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("-")).toBeVisible()
    },
}

export const InvalidDate: Story = {
    args: {record: applicant("not a date")},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("-")).toBeVisible()
    },
}

export const OutsideARecord: Story = {
    args: {record: undefined},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("-")).toBeVisible()
    },
}
