// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useState} from "react"
import type {Meta, StoryObj} from "@storybook/react"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ApolloLink, Observable} from "@apollo/client"
import type {RaRecord} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {AuthContext} from "@/providers/AuthContextProvider"
import {EditUserForm} from "./EditUserForm"

const USER_ID = "33333333-3333-4333-8333-333333333333"
const AREA_ID = "44444444-4444-4444-8444-444444444444"
const attributes = [
    {name: "username", display_name: "Username", annotations: {}, validations: {}},
    {name: "email", display_name: "Email", annotations: {}, validations: {}},
    {
        name: "security-answer",
        display_name: "Security answer",
        annotations: {"sequent.secret": "true"},
        validations: {},
    },
]
const record = {
    id: USER_ID,
    username: "alice",
    email: "alice@example.test",
    enabled: true,
    email_verified: true,
    attributes: {"security-answer": ["redacted"]},
    area: {id: AREA_ID, name: "Central"},
}
interface Scenario {
    permissions: string[]
    voted: boolean
    reveal: "success" | "failure" | "pending"
}
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>
let pendingRequests = 0
let release: (() => Promise<void>) | undefined
function Fixture({permissions}: Scenario) {
    const auth = useContext(AuthContext)
    const [allowed, setAllowed] = useState(permissions)
    const [mounted, setMounted] = useState(true)
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <button
                onClick={() =>
                    setAllowed(
                        allowed.filter((permission) => permission !== "voter-secret-attribute-read")
                    )
                }
            >
                Revoke secret read permission
            </button>
            <button onClick={() => setMounted(false)}>Close editor</button>
            <AuthContext.Provider
                value={{
                    ...auth,
                    isAuthorized: (_super, _tenant, permission) =>
                        allowed.includes(String(permission)),
                }}
            >
                {mounted && (
                    <EditUserForm
                        id={USER_ID}
                        electionEventId={EVENT_ID}
                        rolesList={[]}
                        userAttributes={attributes}
                        userAttributeGroups={[]}
                        record={record}
                    />
                )}
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}
const meta = {
    title: "Admin/Voter editor guards",
    component: Fixture,
    args: {permissions: ["voter-secret-attribute-read"], voted: false, reveal: "success"},
    beforeEach: ({args}) => {
        pendingRequests = 0
        release = undefined
        boundary = graphqlBoundary({
            ListUserRoles: () => ({data: {list_user_roles: []}}),
            RevealVoterSecretAttribute: () => {
                if (args.reveal === "failure")
                    throw new Error("Synthetic secret service unavailable")
                return {
                    data: {
                        reveal_voter_secret_attribute: {
                            attribute_name: "security-answer",
                            values: ["synthetic answer"],
                        },
                    },
                }
            },
        })
        if (args.reveal === "pending") {
            boundary.client.setLink(
                new ApolloLink((operation, forward) => {
                    if (operation.operationName !== "RevealVoterSecretAttribute")
                        return forward(operation)
                    return new Observable((observer) => {
                        pendingRequests++
                        release = () =>
                            new Promise<void>((resolve, reject) => {
                                forward(operation).subscribe({
                                    next: (value) => observer.next(value),
                                    error: (error) => {
                                        observer.error(error)
                                        reject(error)
                                    },
                                    complete: () => {
                                        observer.complete()
                                        resolve()
                                    },
                                })
                            })
                    })
                }).concat(boundary.client.link)
            )
        }
        data = dataBoundary({
            getList: async <RecordType extends RaRecord>(resource: string) => {
                if (resource === "sequent_backend_cast_vote")
                    return {
                        data: (args.voted
                            ? [{id: "cast-vote", voter_id_string: USER_ID}]
                            : []) as unknown as RecordType[],
                        total: args.voted ? 1 : 0,
                    }
                if (resource === "sequent_backend_area")
                    return {
                        data: [{id: AREA_ID, name: "Central"}] as unknown as RecordType[],
                        total: 1,
                    }
                if (resource === "sequent_backend_election")
                    return {data: [] as RecordType[], total: 0}
                data.unexpected.push(resource)
                throw new Error("Unexpected voter editor list")
            },
            getMany: async <RecordType extends RaRecord>(
                resource: string,
                {ids}: {ids: (string | number)[]}
            ) => {
                if (resource !== "sequent_backend_area" || ids.some((id) => id !== AREA_ID)) {
                    data.unexpected.push(resource)
                    throw new Error("Unexpected voter editor area")
                }
                return {data: [{id: AREA_ID, name: "Central"}] as unknown as RecordType[]}
            },
        })
        const services = boundary
        const records = data
        return () => {
            expect(services.unexpected).toEqual([])
            expect(records.unexpected).toEqual([])
        }
    },
    render: (args) => <Fixture {...args} />,
} satisfies Meta<Scenario>
export default meta
type Story = StoryObj<typeof meta>
async function loaded(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await canvas.findByDisplayValue("alice")
    await waitFor(() =>
        expect(data.calls.some((call) => call.args[0] === "sequent_backend_cast_vote")).toBe(true)
    )
    expect(
        data.calls.find((call) => call.args[0] === "sequent_backend_cast_vote")?.args[1]
    ).toMatchObject({
        filter: {tenant_id: TENANT_ID, election_event_id: EVENT_ID, voter_id_string: USER_ID},
    })
    return canvas
}
function secret(canvasElement: HTMLElement) {
    return within(canvasElement).getByLabelText("Security answer")
}
function expectRevealRequest() {
    expect(boundary.calls.filter((call) => call.name === "RevealVoterSecretAttribute")).toEqual([
        {
            name: "RevealVoterSecretAttribute",
            variables: {
                tenantId: TENANT_ID,
                electionEventId: EVENT_ID,
                userId: USER_ID,
                attributeName: "security-answer",
            },
            headers: {},
        },
    ])
}
export const SecretNeedsReadPermission: Story = {
    args: {permissions: []},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        expect(secret(canvasElement)).toHaveValue("")
        expect(secret(canvasElement)).toHaveAttribute("type", "password")
        expect(secret(canvasElement)).toHaveAttribute("readonly")
        expect(canvas.queryByRole("button", {name: "Reveal"})).not.toBeInTheDocument()
        expect(boundary.calls.map((call) => call.name)).toEqual(["ListUserRoles"])
    },
}
export const RevealThenHideErasesPlaintext: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Reveal"}))
        await waitFor(() => expect(secret(canvasElement)).toHaveValue("synthetic answer"))
        expectRevealRequest()
        expect(secret(canvasElement)).toHaveAttribute("readonly")
        await userEvent.click(canvas.getByRole("button", {name: "Hide"}))
        expect(secret(canvasElement)).toHaveValue("")
        expect(secret(canvasElement)).toHaveAttribute("type", "password")
        await userEvent.click(canvas.getByRole("button", {name: "Reveal"}))
        await waitFor(() => expect(secret(canvasElement)).toHaveValue("synthetic answer"))
        expect(
            boundary.calls.filter((call) => call.name === "RevealVoterSecretAttribute")
        ).toHaveLength(2)
    },
}
export const RevealFailureKeepsSecretHidden: Story = {
    args: {reveal: "failure"},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Reveal"}))
        await waitFor(() =>
            expect(
                boundary.calls.filter((call) => call.name === "RevealVoterSecretAttribute")
            ).toHaveLength(1)
        )
        await waitFor(() => expect(canvas.getByRole("button", {name: "Reveal"})).toBeEnabled())
        await waitFor(() =>
            expect(
                within(document.body).getByText("The encrypted voter field could not be revealed")
            ).toBeVisible()
        )
        expect(secret(canvasElement)).toHaveValue("")
        expect(secret(canvasElement)).toHaveAttribute("type", "password")
    },
}
export const RevokingPermissionErasesVisibleSecret: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Reveal"}))
        await waitFor(() => expect(secret(canvasElement)).toHaveValue("synthetic answer"))
        await userEvent.click(canvas.getByRole("button", {name: "Revoke secret read permission"}))
        await waitFor(() => expect(secret(canvasElement)).toHaveValue(""))
        expect(canvas.queryByRole("button", {name: "Reveal"})).not.toBeInTheDocument()
        expect(canvas.queryByRole("button", {name: "Hide"})).not.toBeInTheDocument()
    },
}
export const LateRevealCannotRestoreRevokedSecret: Story = {
    args: {reveal: "pending"},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Reveal"}))
        await waitFor(() => expect(pendingRequests).toBe(1))
        expect(canvas.getByRole("button", {name: "Reveal"})).toBeDisabled()
        await userEvent.click(canvas.getByRole("button", {name: "Revoke secret read permission"}))
        if (!release) throw new Error("No pending secret request")
        await release()
        await waitFor(() => expect(secret(canvasElement)).not.toBeDisabled())
        expect(secret(canvasElement)).toHaveValue("")
        expectRevealRequest()
    },
}
export const LateRevealCannotReopenClosedEditor: Story = {
    args: {reveal: "pending"},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Reveal"}))
        await waitFor(() => expect(pendingRequests).toBe(1))
        await userEvent.click(canvas.getByRole("button", {name: "Close editor"}))
        if (!release) throw new Error("No pending secret request")
        await release()
        expect(canvas.queryByLabelText("Security answer")).not.toBeInTheDocument()
        expect(canvas.queryByDisplayValue("synthetic answer")).not.toBeInTheDocument()
        expectRevealRequest()
    },
}
export const EmailEditorCanEditBeforeVoting: Story = {
    args: {permissions: ["voter-email-tlf-edit"]},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        expect(canvas.getByRole("textbox", {name: "Email"})).toBeEnabled()
        expect(canvas.getByRole("textbox", {name: "Username"})).toBeDisabled()
    },
}
export const EmailEditorCannotEditAfterVoting: Story = {
    args: {permissions: ["voter-email-tlf-edit"], voted: true},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await waitFor(() => expect(canvas.getByRole("textbox", {name: "Email"})).toBeDisabled())
        expect(canvas.getByRole("textbox", {name: "Username"})).toBeDisabled()
    },
}
export const VotedEditPermissionAllowsFields: Story = {
    args: {permissions: ["voter-voted-edit"], voted: true},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        expect(canvas.getByRole("textbox", {name: "Email"})).toBeEnabled()
        expect(canvas.getByRole("textbox", {name: "Username"})).toBeDisabled()
        expect(canvas.getByRole("checkbox", {name: "Enabled *"})).toBeEnabled()
    },
}
