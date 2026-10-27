/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import {ResumePublication} from "./ResumePublication"

let mockRecord: Record<string, unknown>
let mockWrite = true
let mockRead = true
jest.mock("react-admin", () => ({useRecordContext: () => mockRecord}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("./usePublishPermissions", () => ({
    usePublishPermissions: () => ({canReadPublish: mockRead, canWritePublish: mockWrite}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (_key: string, options: {defaultValue: string}) => options.defaultValue,
    }),
}))

beforeEach(() => {
    mockRead = true
    mockWrite = true
    mockRecord = {
        id: "publication",
        tenant_id: "tenant",
        election_event_id: "event",
        is_generated: true,
        published_at: null,
        deleted_at: null,
    }
})

it("opens only the existing held artifact selected in this event", () => {
    const resume = jest.fn()
    render(<ResumePublication electionEventId="event" onResume={resume} />)
    fireEvent.click(screen.getByRole("button", {name: "Resume approval"}))
    expect(resume).toHaveBeenCalledWith("publication")
    expect(resume).toHaveBeenCalledTimes(1)
})

it.each([
    ["is_generated", false],
    ["is_generated", undefined],
    ["published_at", "2026-10-27T00:00:00Z"],
    ["published_at", undefined],
    ["deleted_at", "2026-10-27T00:00:00Z"],
    ["deleted_at", undefined],
    ["tenant_id", "other"],
    ["election_event_id", "other"],
])("keeps history unavailable for approval when %s is %s", (field, value) => {
    mockRecord[field as string] = value
    render(<ResumePublication electionEventId="event" onResume={jest.fn()} />)
    expect(screen.queryByRole("button")).toBeNull()
})

it.each(["read", "write"])("requires %s permission", (permission) => {
    if (permission === "read") mockRead = false
    else mockWrite = false
    render(<ResumePublication electionEventId="event" onResume={jest.fn()} />)
    expect(screen.queryByRole("button")).toBeNull()
})
