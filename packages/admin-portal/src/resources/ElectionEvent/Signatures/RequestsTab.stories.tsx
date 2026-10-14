// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {recordDownloads} from "@/__stories__/downloads"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SigningRequestStatus} from "@/lib/signing/types"
import {fakeApi, makePanel} from "@/components/signing/__stories__/fixtures"
import {RequestsTab} from "./RequestsTab"
import {requestStatus, signaturesAccess} from "./signingSettings"
import {
    EXPORT_URL,
    Organization,
    SIGNING_ROLES,
    SignaturesStory,
    applyOverrides,
    organizationOf,
    signingHandlers,
    signingRecords,
    type SigningRole,
} from "./__stories__/SignaturesFixture"

interface Scenario {
    organization: Organization
    role: SigningRole
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof signingRecords>
let saved: ReturnType<typeof recordDownloads>
/** The signing widget's Harvest: the panel a row opens reads its request here. */
let signingApi: ReturnType<typeof fakeApi>

const meta = {
    title: "Admin/Election event/Signatures/RequestsTab",
    component: RequestsTab,
    args: {organization: Organization.Overseas, role: "ofov"},
    argTypes: {
        organization: {control: "inline-radio", options: Object.values(Organization)},
        role: {control: "select", options: Object.keys(SIGNING_ROLES)},
    },
    beforeEach: async ({args}) => {
        const organization = organizationOf(args.organization)
        signingApi = fakeApi(await makePanel({}))
        graphql = graphqlBoundary(signingHandlers(organization))
        data = signingRecords(organization)
        saved = recordDownloads()
        const restoreOverrides = applyOverrides(organization)
        return () => {
            saved.restore()
            restoreOverrides?.()
        }
    },
    render: ({role}) => {
        const roles = new Set<string>(SIGNING_ROLES[role])
        return (
            <SignaturesStory boundary={graphql} data={data} role={role} signingApi={signingApi}>
                <RequestsTab
                    electionEventId={EVENT_ID}
                    access={signaturesAccess((permission) => roles.has(permission))}
                />
            </SignaturesStory>
        )
    },
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const label = (key: string, options?: Record<string, unknown>) => i18n.t(`signing.${key}`, options)

const requestRows = async (canvasElement: HTMLElement) => {
    const table = await within(canvasElement).findByRole("table", {name: label("tab.requests")})
    await within(table).findAllByText(/·/)
    return within(table).getAllByRole("row").slice(1)
}

export const Ofov: Story = {
    play: async ({canvasElement}) => {
        const {requests, posts, countries} = organizationOf(Organization.Overseas)
        const rows = await requestRows(canvasElement)
        expect(rows).toHaveLength(requests.length)
        // Each request names its action, Post and country, from the event's data.
        const first = within(rows[1])
        // Post and country names load after the requests.
        await expect(
            await first.findByRole("button", {
                name: [
                    label("actions.generate-election-returns.short"),
                    posts[0].name,
                    countries[0].name,
                ].join(" · "),
            })
        ).toBeVisible()
        await expect(
            first.getByText(
                label("requests.statusCount", {status: label("status.waiting"), count: 0, total: 3})
            )
        ).toBeVisible()
        const cancelled = within(rows[4])
        await expect(cancelled.getByText(label("status.cancelled"))).toBeVisible()
        await expect(cancelled.getByText(label("cancelReasons.payload-changed"))).toBeVisible()
        // People by the names stored with the rows.
        const [, signed] = requests[2].approvals
        await expect(
            within(rows[2]).getByText(new RegExp(`^${signed.display_name}, `))
        ).toBeVisible()
        await expect(
            within(rows[0]).getByText(requests[0].requested_by_name as string)
        ).toBeVisible()
        await expect(within(rows[0]).getByText(requests[0].code)).toBeVisible()
        // Times in the event's zone with its name, as the signing panel shows them
        // Asia/Manila: 11:02 UTC is 7:02 PM PhST in the canonical admin format.
        await expect(within(rows[0]).getByText("May 8, 2028, 7:02 PM PhST")).toBeVisible()
        await expect(
            within(rows[0]).getByText(
                label("requests.expires", {time: "May 8, 2028, 7:32 PM PhST"})
            )
        ).toBeVisible()
        await expect(
            within(rows[0]).getByText(
                label("requests.lastSignatureBy", {
                    name: requests[0].approvals[0].display_name,
                    time: "May 8, 2028, 7:03 PM PhST",
                })
            )
        ).toBeVisible()
        // A waiting request past its time shows as expired before the job marks it.
        const late = requests.findIndex(
            ({status, expires_at}) =>
                status === SigningRequestStatus.Waiting &&
                !!expires_at &&
                Date.parse(expires_at) < Date.now()
        )
        await expect(
            within(rows[late]).getByText(
                label("requests.statusCount", {
                    status: label("status.expired"),
                    count: 0,
                    total: requests[late].required,
                })
            )
        ).toBeVisible()

        // OFOV cancels stuck requests from the panel, so the list isn't read-only.
        expect(within(canvasElement).queryByText(label("readOnly.chip"))).toBeNull()

        // A row opens the signing widget's request panel, where it can be cancelled.
        await userEvent.click(within(rows[0]).getByText(requests[0].code))
        await waitFor(() => expect(signingApi.getRequest).toHaveBeenCalledWith(requests[0].id))
        await expect(
            await within(document.body).findByRole("heading", {name: / · /, level: 2})
        ).toBeVisible()
        expect(graphql.calls.find(({name}) => name === "GetSigningRequests")).toMatchObject({
            headers: {"x-hasura-role": "signing-requests-read"},
        })
        // The event screen supplies its timezone, so rendering dates needs no extra read.
        expect(graphql.calls.filter(({name}) => name === "SigningEventInfo")).toEqual([])
    },
}

export const FilterByStatus: Story = {
    play: async ({canvasElement}) => {
        await requestRows(canvasElement)
        await userEvent.click(
            within(canvasElement).getByRole("combobox", {name: label("requests.status")})
        )
        await userEvent.click(
            await within(document.body).findByRole("option", {name: label("status.waiting")})
        )
        // As shown: a waiting request past its time counts as expired.
        const waiting = organizationOf(Organization.Overseas).requests.filter(
            (request) => requestStatus(request, new Date()) === SigningRequestStatus.Waiting
        )
        await waitFor(async () =>
            expect(await requestRows(canvasElement)).toHaveLength(waiting.length)
        )
    },
}

export const ExportCsv: Story = {
    play: async ({canvasElement}) => {
        await requestRows(canvasElement)
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: label("requests.exportCsv")})
        )
        await waitFor(() =>
            expect(saved.downloads).toEqual([
                {name: label("requests.exportFileName"), href: EXPORT_URL},
            ])
        )
        expect(
            graphql.calls
                .filter(({name}) => name === "SigningExportRequests")
                .map(({variables}) => variables)
        ).toEqual([{election_event_id: EVENT_ID}])
        // The export's own link: no document read, which staff without admin-user can't make.
        expect(
            graphql.calls.filter(({name}) => name === "GetDocument" || name === "FetchDocument")
        ).toEqual([])
    },
}

export const WithoutExportPermission: Story = {
    args: {role: "requestsReader"},
    play: async ({canvasElement}) => {
        await requestRows(canvasElement)
        expect(
            within(canvasElement).queryByRole("button", {name: label("requests.exportCsv")})
        ).toBeNull()
        await expect(within(canvasElement).getByText(label("readOnly.chip"))).toBeVisible()
    },
}

export const SecondOrganization: Story = {
    args: {organization: Organization.StudentCouncil},
    play: async ({canvasElement}) => {
        const rows = await requestRows(canvasElement)
        const {posts} = organizationOf(Organization.StudentCouncil)
        await expect(
            await within(rows[0]).findByRole("button", {
                name: `${label("actions.close-voting.short")} · ${posts[0].name}`,
            })
        ).toBeVisible()
        // Its event's own zone (Europe/Madrid: 11:02 UTC is 1:02 PM GMT+2).
        await expect(within(rows[0]).getByText("May 8, 2028, 1:02 PM GMT+2")).toBeVisible()
    },
}
