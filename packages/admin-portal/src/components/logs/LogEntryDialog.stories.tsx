// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {LogEntryDialog, type ILogRow} from "./LogEntryDialog"
import {ELogScope, type ILogTarget} from "./logScope"
import {
    ADMIN_KEY,
    DELETED_EVENT_DATABASE,
    DELETED_EVENT_ENTRY,
    EXPORT_ENTRY,
    KEYCLOAK_CONSOLE_ENTRY,
    LOCKOUT_ENTRY,
    SETTINGS_ENTRY,
    STATE_HASH,
    SYSTEM_SIGNATURE,
    TENANT_LOG_ZONE,
    UNKNOWN_ACCOUNT_ENTRY,
    WRITER_STARTED_ENTRY,
    tenantLogRows,
} from "./__stories__/tenantLogFixture"

interface Scenario {
    entry: ILogRow | null
    log: ILogTarget
    zone: string
    myZone: string
    onClose: () => void
}

const TENANT_LOG: ILogTarget = {kind: ELogScope.TENANT}
const SEAL_HASH = `ef187f0b${"3c".repeat(56)}22a65e5b`
const SEALED = "Ballot box of the North area sealed: 1,340 of 1,342 ballots counted."

/** An entry of an election event's log whose kind has an explanation of its own. */
const SEAL_ENTRY: ILogRow = {
    id: 18442,
    created: 1833186098,
    statement_timestamp: 1833186097,
    statement_kind: "BallotBoxSealed",
    user_id: "",
    message: JSON.stringify({
        sender: {name: "", pk: ADMIN_KEY},
        system_signature: SYSTEM_SIGNATURE,
        user_id: null,
        username: null,
        statement: {
            head: {event_type: "SYSTEM", log_type: "INFO", description: SEALED},
            body: {BallotBoxSealed: [null, "north", SEAL_HASH, 1342, 1340, null]},
        },
    }),
}

const byKind = (kind: string, id: number) => {
    const row = tenantLogRows().find((entry) => entry.statement_kind === kind && entry.id === id)
    if (!row) throw new Error(`The tenant log fixture has no ${kind} entry ${id}`)
    return row
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Logs/LogEntryDialog",
    component: LogEntryDialog,
    args: {
        entry: SETTINGS_ENTRY,
        log: TENANT_LOG,
        zone: TENANT_LOG_ZONE,
        myZone: "America/New_York",
        onClose: fn(),
    },
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({myZone, ...args}: Scenario) => (
        <AdminStoryProvider boundary={graphql}>
            <MyTimeZoneProvider zone={myZone}>
                <LogEntryDialog {...args} />
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const dialog = async () => {
    const element = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

/** The row of the changes table that names `path`. */
const changeRow = (view: ReturnType<typeof within>, path: string) =>
    within(view.getByText(path).closest("tr") as HTMLElement)

/** A Settings change: who, when, where, and each field's old and new value. */
export const SettingsChange: Story = {
    parameters: {
        widgets: ["EntryContent", "Facts", "Fact", "Section", "ChangesTable", "Shortened"],
    },
    play: async () => {
        const view = await dialog()
        await expect(
            within(document.body).getByRole("dialog", {name: "Log entry 5,259"})
        ).toBeVisible()
        expect(
            view.getByText("Settings changed: Languages, Integrations and Voting Channels")
        ).toBeVisible()
        expect(view.getByLabelText("Statement kind: AdminChange")).toBeVisible()
        expect(view.getByLabelText("Event Type: USER")).toBeVisible()
        expect(view.getByLabelText("Log Type: INFO")).toBeVisible()
        // The log's zone, my time and UTC.
        expect(view.getByText(/10:21:37 AM GMT\+1/)).toBeVisible()
        expect(view.getByText(/4:21:37 AM EST · my time/)).toBeVisible()
        expect(view.getByText("2028-02-03T09:21:37Z UTC")).toBeVisible()
        expect(view.getByText("Rosa Marin (rosa.marin)")).toBeVisible()
        expect(view.getByText("Role admin-user")).toBeVisible()
        expect(view.getByText("Admin Portal (through Hasura)")).toBeVisible()
        expect(view.getByText("Tenant settings: Northwind Staff Association")).toBeVisible()
        expect(view.getByText("Updated")).toBeVisible()

        expect(view.getAllByRole("columnheader").map(({textContent}) => textContent)).toEqual([
            "Field",
            "Old value",
            "New value",
        ])
        const languages = changeRow(view, "settings.language_conf")
        expect(languages.getByText("Languages")).toBeVisible()
        expect(
            languages.getByText("enabled_language_codes: en, es; default_language_code: en")
        ).toBeVisible()
        const secret = changeRow(view, "settings.gapi_key")
        expect(secret.getByText("Google API key (Integrations)")).toBeVisible()
        expect(secret.getByText("Changed. Secret values are never logged.")).toBeVisible()
        const kiosk = changeRow(view, "voting_channels.kiosk")
        expect(kiosk.getByText("No")).toBeVisible()
        expect(kiosk.getByText("Yes")).toBeVisible()

        expect(view.getByText("Tenant log")).toBeVisible()
        expect(view.getByText("Rosa Marin's admin key")).toBeVisible()
        // Shortened on the line, whole in the title.
        expect(view.getByTitle(ADMIN_KEY)).toHaveTextContent("MCowBQYDK2VwAyEA9sLw…4W5c=")
        expect(view.getByTitle(SYSTEM_SIGNATURE)).toBeVisible()
    },
}

/** A long text keeps only its SHA-256 before and after; a field without a label shows its path. */
export const LongTextAsHashes: Story = {
    args: {entry: byKind("AdminChange", 5220)},
    play: async () => {
        const view = await dialog()
        const body = changeRow(view, "template.email.html_body")
        expect(body.getAllByText(/^SHA-256 [0-9a-f]{64}$/)).toHaveLength(2)
        const subject = changeRow(view, "template.email.subject")
        expect(subject.getByText("Your enrollment is approved")).toBeVisible()
        expect(view.getByText("Template: Enrollment approved")).toBeVisible()
    },
}

/** A value that was not set before shows as (none). */
export const RoleGiven: Story = {
    args: {entry: byKind("AdminChange", 5222)},
    play: async () => {
        const view = await dialog()
        const groups = changeRow(view, "groups")
        expect(groups.getByText("(none)")).toBeVisible()
        expect(groups.getByText("Staff")).toBeVisible()
        expect(view.getByText("Admin Portal (through Harvest)")).toBeVisible()
        expect(view.getByText("Granted")).toBeVisible()
        expect(view.getByText("Staff account: lena.ortiz")).toBeVisible()
    },
}

/** A change made in Keycloak's own console: the operator, the operation and the new value. */
export const ChangedInKeycloaksConsole: Story = {
    args: {entry: KEYCLOAK_CONSOLE_ENTRY},
    play: async () => {
        const view = await dialog()
        expect(view.getByText("Change in Keycloak")).toBeVisible()
        expect(view.getByText("Keycloak's admin console, outside the Admin Portal")).toBeVisible()
        expect(view.getByText("ops.lund · master")).toBeVisible()
        expect(view.getByText("security-admin-console · 203.0.113.41")).toBeVisible()
        expect(view.getByText("CREATE GROUP_MEMBERSHIP")).toBeVisible()
        expect(view.getByText("Security Officer")).toBeVisible()
        expect(view.getByText(/"path": "\/Security Officer"/)).toBeVisible()
        expect(view.getByText(/^users\/.*\/groups\//)).toBeVisible()
        expect(view.getByText("Ops Lund's admin key")).toBeVisible()
    },
}

/** What Step did in Keycloak for a change made in the portal: a SYSTEM entry. */
export const ChangedByStepsServiceAccount: Story = {
    args: {entry: byKind("KeycloakAdminEvent", 5221)},
    play: async () => {
        const view = await dialog()
        expect(
            view.getByText("Step's service account, for a change made in the Admin Portal")
        ).toBeVisible()
        expect(view.getByLabelText("Event Type: SYSTEM")).toBeVisible()
        expect(view.getByText("The log's system key only")).toBeVisible()
    },
}

/** The tenant log keeps a deleted event's last entry and state hash. */
export const DeletedElectionEvent: Story = {
    args: {entry: DELETED_EVENT_ENTRY},
    play: async () => {
        const view = await dialog()
        expect(view.getByText("The deleted election event's log")).toBeVisible()
        expect(view.getByText("18442")).toBeVisible()
        expect(view.getByTitle(STATE_HASH)).toBeVisible()
        expect(view.getByText(DELETED_EVENT_DATABASE)).toBeVisible()
        expect(
            view.getByText("Unloaded and kept until it is erased under the retention policy")
        ).toBeVisible()
        expect(view.getByText("Election event: Training 2027")).toBeVisible()
        expect(view.getByText("Admin Portal (a Windmill task)")).toBeVisible()
        expect(view.getByText("Deleted")).toBeVisible()
        expect(view.queryByText("Old and new values")).toBeNull()
    },
}

/** A lockout: the Keycloak event, the failures in a row, the client and the address. */
export const Lockout: Story = {
    args: {entry: LOCKOUT_ENTRY},
    parameters: {widgets: ["Mono"]},
    play: async () => {
        const view = await dialog()
        expect(view.getByText("Sign-in and account security")).toBeVisible()
        expect(view.getByText("USER_DISABLED_BY_TEMPORARY_LOCKOUT")).toBeVisible()
        expect(view.getByText("user_temporarily_disabled")).toBeVisible()
        expect(view.getByText("Failures in a row")).toBeVisible()
        expect(view.getByText("30")).toBeVisible()
        expect(view.getByText("admin-portal")).toBeVisible()
        expect(view.getByText("203.0.113.77")).toBeVisible()
        expect(view.getByText("openid-connect · code")).toBeVisible()
        expect(view.getByLabelText("Log Type: ERROR")).toBeVisible()
    },
}

/** A try at an account that doesn't exist: the name typed is not recorded. */
export const UnknownAccount: Story = {
    args: {entry: UNKNOWN_ACCOUNT_ENTRY},
    play: async () => {
        const view = await dialog()
        expect(view.getByText("Unknown account (the name typed is not recorded)")).toBeVisible()
        expect(view.getByText("user_not_found")).toBeVisible()
        expect(view.queryByText(/^User ID/)).toBeNull()
    },
}

export const Exported: Story = {
    args: {entry: EXPORT_ENTRY},
    play: async () => {
        const view = await dialog()
        expect(view.getByText("Export")).toBeVisible()
        expect(view.getByText("CSV")).toBeVisible()
        expect(
            view.getByText("One account, with its entries in every election event")
        ).toBeVisible()
        expect(view.getByText("rosa.marin")).toBeVisible()
        expect(view.getByText("14")).toBeVisible()
        expect(view.getByText(TENANT_LOG_ZONE)).toBeVisible()
        expect(view.getByText("2028-01-31T23:00:00Z")).toBeVisible()
        expect(view.getByText("Ana Vega's admin key")).toBeVisible()
    },
}

export const LogWriterStarted: Story = {
    args: {entry: WRITER_STARTED_ENTRY},
    play: async () => {
        const view = await dialog()
        expect(view.getByText("Log writer")).toBeVisible()
        expect(view.getByText("10.1.0")).toBeVisible()
        expect(view.getByText("5254")).toBeVisible()
        expect(view.getByText("The log's system key only")).toBeVisible()
    },
}

export const AdminPublicKey: Story = {
    args: {entry: byKind("AdminPublicKey", 5200)},
    play: async () => {
        const view = await dialog()
        expect(view.getByText("Published admin key")).toBeVisible()
        expect(view.getAllByTitle(ADMIN_KEY)).toHaveLength(2)
    },
}

/** An election event's entry: the same popup, in the event's log, with the kind's explanation. */
export const EventEntryWithItsExplanation: Story = {
    args: {
        entry: SEAL_ENTRY,
        log: {kind: ELogScope.EVENT, electionEventId: EVENT_ID},
        zone: "Asia/Manila",
        myZone: "Asia/Manila",
    },
    play: async () => {
        const view = await dialog()
        await expect(
            within(document.body).getByRole("dialog", {name: "Log entry 18,442"})
        ).toBeVisible()
        expect(view.getByText(SEALED)).toBeVisible()
        expect(view.getByText(`Seal hash: ${SEAL_HASH}`)).toBeVisible()
        expect(view.getByText("1340 of 1342 ballots counted.")).toBeVisible()
        expect(view.getByText("Election event log")).toBeVisible()
        expect(view.getByText(EVENT_ID)).toBeVisible()
        // The viewer is in the log's zone: no second time.
        expect(view.queryByText(/my time/)).toBeNull()
    },
}

/** A message that isn't the JSON the popup knows is shown as it is. */
export const UnreadableMessage: Story = {
    args: {
        entry: {
            id: 7,
            created: 1833186098,
            statement_timestamp: 1833186097,
            statement_kind: "AdminChange",
            message: "not a JSON message",
        },
    },
    play: async () => {
        const view = await dialog()
        expect(view.getByText("not a JSON message")).toBeVisible()
        expect(view.getByLabelText("Statement kind: AdminChange")).toBeVisible()
        expect(view.queryByText("Log and signatures")).toBeNull()
        expect(view.queryByText("Who")).toBeNull()
    },
}

export const CloseButton: Story = {
    play: async ({args}) => {
        const view = await dialog()
        expect(view.getAllByRole("button")).toHaveLength(1)
        await userEvent.click(view.getByRole("button", {name: "Close"}))
        expect(args.onClose).toHaveBeenCalled()
    },
}

export const EscapeCloses: Story = {
    play: async ({args}) => {
        await dialog()
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(args.onClose).toHaveBeenCalled())
    },
}

export const Closed: Story = {
    args: {entry: null},
    play: async () => {
        expect(within(document.body).queryByRole("dialog")).toBeNull()
    },
}
