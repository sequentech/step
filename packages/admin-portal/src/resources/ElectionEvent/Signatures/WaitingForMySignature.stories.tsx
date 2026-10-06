// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {fakeApi, makePanel} from "@/components/signing/__stories__/fixtures"
import {SIGNING_ACTIONS, SigningRequestStatus} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"
import {WaitingForMySignature} from "./WaitingForMySignature"
import {
    Organization,
    applyOverrides,
    organizationOf,
    signingHandlers,
    signingRecords,
    userIdOf,
} from "./__stories__/SignaturesFixture"

interface Scenario {
    organization: Organization
    /** The signed-in member, by username. */
    username: string
    /** What they hold: sign permissions, never the Signatures tab. */
    permissions: IPermissions[]
}

/** An SBEI of the overseas event: the Post actions, without the Signatures tab. */
const SBEI = [
    IPermissions.ELECTION_EVENT_READ,
    IPermissions.SIGN_CLOSE_VOTING,
    IPermissions.SIGN_GENERATE_ELECTION_RETURNS,
    // A trustee's requests aren't listed here: their ceremony step opens them.
    IPermissions.SIGN_KEY_CEREMONY,
]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof signingRecords>
let signingApi: ReturnType<typeof fakeApi>

const meta = {
    title: "Admin/Election event/Signatures/WaitingForMySignature",
    component: WaitingForMySignature,
    args: {organization: Organization.Overseas, username: "sbei-wellington-1", permissions: SBEI},
    argTypes: {
        organization: {control: "inline-radio", options: Object.values(Organization)},
    },
    beforeEach: async ({args}) => {
        const organization = organizationOf(args.organization)
        signingApi = fakeApi(await makePanel({}))
        graphql = graphqlBoundary(signingHandlers(organization))
        data = signingRecords(organization)
        return applyOverrides(organization)
    },
    render: ({username, permissions}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={permissions}
            auth={{
                isAuthenticated: true,
                userId: userIdOf(username),
                username,
                tenantId: TENANT_ID,
            }}
            signingApi={signingApi}
        >
            <WaitingForMySignature electionEventId={EVENT_ID} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const label = (key: string, options?: Record<string, unknown>) => i18n.t(`signing.${key}`, options)

/** Opens the list from its button, which counts the requests left to sign. */
async function openList(canvasElement: HTMLElement, toSign: number) {
    await userEvent.click(
        await within(canvasElement).findByRole("button", {
            name: label("waiting.buttonCount", {count: toSign}),
        })
    )
    const heading = await within(document.body).findByRole("heading", {
        name: label("waiting.title"),
    })
    return within(heading.closest(".MuiDrawer-paper") as HTMLElement)
}

/** The rows of the list, by their request id. */
const rows = (list: ReturnType<typeof within>): HTMLElement[] =>
    list.getAllByRole("button").filter((row: HTMLElement) => !!row.dataset.request)

/** The roles the list's queries were sent with: one per sign permission whose requests Hasura lists. */
const queryRoles = () =>
    graphql.calls
        .filter(({name}) => name === "GetWaitingSigningRequests")
        .map(({headers}) => headers["x-hasura-role"])

export const Sbei: Story = {
    parameters: {widgets: ["WaitingList"]},
    play: async ({canvasElement}) => {
        const {requests, posts, countries} = organizationOf(Organization.Overseas)
        // Waiting, of the actions held, not past their time: close voting at Wellington,
        // which this member signed, and the election returns of Madrid, Spain.
        const [closing, returns] = requests
        const list = await openList(canvasElement, 1)
        await waitFor(() =>
            expect(rows(list).map((row) => row.dataset.request)).toEqual([closing.id, returns.id])
        )
        const [signedRow, toSignRow] = rows(list).map((row) => within(row))
        await expect(
            await toSignRow.findByText(
                [
                    label("actions.generate-election-returns.short"),
                    posts[0].name,
                    countries[0].name,
                ].join(" · ")
            )
        ).toBeVisible()
        await expect(toSignRow.getByText(returns.code)).toBeVisible()
        expect(toSignRow.queryByText(label("waiting.signedByYou"))).toBeNull()
        // Each one's count, and its expiry in the event's zone (Asia/Manila).
        await expect(
            signedRow.getByText(
                label("requests.statusCount", {
                    status: label("status.waiting"),
                    count: 1,
                    total: closing.required,
                })
            )
        ).toBeVisible()
        await expect(
            signedRow.getByText(label("requests.expires", {time: "May 8, 2028, 19:32 PhST"}))
        ).toBeVisible()
        await expect(signedRow.getByText(label("waiting.signedByYou"))).toBeVisible()
        // One query per sign permission held whose requests Hasura lists, as that role.
        expect(new Set(queryRoles())).toEqual(
            new Set([IPermissions.SIGN_CLOSE_VOTING, IPermissions.SIGN_GENERATE_ELECTION_RETURNS])
        )

        // A row opens the request's panel.
        await userEvent.click(rows(list)[1])
        await waitFor(() => expect(signingApi.getRequest).toHaveBeenCalledWith(returns.id))
    },
}

export const AnotherMember: Story = {
    args: {username: "sbei-madrid-2"},
    play: async ({canvasElement}) => {
        // Nothing signed yet: both left to sign.
        const list = await openList(canvasElement, 2)
        await waitFor(() => expect(rows(list)).toHaveLength(2))
        expect(list.queryByText(label("waiting.signedByYou"))).toBeNull()
    },
}

export const OneAction: Story = {
    args: {permissions: [IPermissions.ELECTION_EVENT_READ, IPermissions.SIGN_OPEN_VOTING]},
    play: async ({canvasElement}) => {
        // Its only waiting request is past its time: nothing to sign.
        const list = await openList(canvasElement, 0)
        await expect(await list.findByText(label("waiting.empty"))).toBeVisible()
        expect(new Set(queryRoles())).toEqual(new Set([IPermissions.SIGN_OPEN_VOTING]))
        const expired = organizationOf(Organization.Overseas).requests.filter(
            ({action, status}) =>
                SIGNING_ACTIONS[action].signPermission === IPermissions.SIGN_OPEN_VOTING &&
                status === SigningRequestStatus.Waiting
        )
        expect(expired).toHaveLength(1)
    },
}

export const WithoutSignPermission: Story = {
    args: {
        permissions: [
            IPermissions.ELECTION_EVENT_READ,
            IPermissions.SIGNING_REQUESTS_READ,
            IPermissions.SIGN_TALLY_KEY,
        ],
    },
    play: async ({canvasElement}) => {
        // No action whose requests a signer finds here: no entry, and no query.
        await new Promise((resolve) => setTimeout(resolve, 200))
        expect(within(canvasElement).queryByRole("button")).toBeNull()
        expect(queryRoles()).toEqual([])
    },
}

export const SecondOrganization: Story = {
    args: {
        organization: Organization.StudentCouncil,
        username: "returning-officer-law",
        permissions: [IPermissions.ELECTION_EVENT_READ, IPermissions.SIGN_CLOSE_VOTING],
    },
    play: async ({canvasElement}) => {
        const {requests, posts} = organizationOf(Organization.StudentCouncil)
        const list = await openList(canvasElement, 1)
        const [row] = rows(list).map((element) => within(element))
        await expect(
            await row.findByText(`${label("actions.close-voting.short")} · ${posts[0].name}`)
        ).toBeVisible()
        await expect(row.getByText(requests[0].code)).toBeVisible()
    },
}
