// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test as unit, type Page} from "@playwright/test"
import {IPermissions} from "../../../src/types/keycloak"
import {test, expect, TENANT_ID, type AdminPortal} from "../fixtures"
import {appRoutes} from "./routes"
import {serveAdminTenant, SHELL_IDS} from "./data"
import {serveTrusteeWorkerHelper} from "../tally/trustee-startup"

/**
 * One entry per router route. The matrix gives breadth: each route loads with
 * a configured tenant's data and shows its main content, with no page error,
 * console error or unexpected request. Area specs cover the behaviour.
 */
interface Smoke {
    /** The URL to open, with the id of a record the tenant has. */
    url: string
    /** The path a redirecting route settles on. */
    settlesAt?: string
    shows: (page: Page, portal: AdminPortal) => Promise<void>
    checkConsole?: boolean
}

const heading = (page: Page, name: string) =>
    expect(page.getByRole("heading", {name, exact: true}).first()).toBeVisible()
const text = (page: Page, value: string) =>
    expect(page.getByText(value, {exact: true}).first()).toBeVisible()
const cell = (page: Page, name: string) =>
    expect(page.getByRole("cell", {name, exact: true}).first()).toBeVisible()

// Passive effects run after paint; two frames and a task later they have all run.
const nextFrames = (page: Page) =>
    page.evaluate(
        () =>
            new Promise<void>((resolve) =>
                requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(resolve)))
            )
    )

const eventPage = async (page: Page) => {
    await text(page, "Council 2026")
    await text(page, "Election event configuration.")
    await expect(page.getByRole("tab", {name: "Dashboard", exact: true})).toHaveAttribute(
        "aria-selected",
        "true"
    )
}
const eventRedirect = {
    settlesAt: `/sequent_backend_election_event/${SHELL_IDS.event}`,
    shows: eventPage,
}
const settingsPage = async (page: Page) => {
    await text(page, "Settings")
    await cell(page, "Municipal")
}
const electionPage = async (page: Page) => {
    await text(page, "Mayor")
    await text(page, "Election configuration.")
}
const contestPage = async (page: Page) => {
    await text(page, "Mayor contest")
    await text(page, "Contest configuration.")
}
const candidatePage = async (page: Page) => {
    await text(page, "Alice Aldana")
    await text(page, "Candidate configuration.")
}
const reportsPage = async (page: Page) => {
    await text(page, "Reports")
    await cell(page, "Ballot Receipt")
}
const scheduledEventsPage = async (page: Page) => {
    await text(page, "Scheduled Events")
    await cell(page, "Start Voting Period")
}
const usersPage = (page: Page) => cell(page, "maria.lopez")

export const SMOKE: Record<string, Smoke> = {
    "/": {url: "/", ...eventRedirect},
    "/tenant": {
        url: "/tenant",
        shows: async (page) => {
            await heading(page, "Select Tenant")
            await expect(page.getByRole("textbox", {name: "Tenant Name"})).toBeVisible()
        },
    },
    "/user-roles": {
        url: "/user-roles",
        shows: async (page) => {
            await text(page, "Users and Roles")
            await usersPage(page)
        },
    },
    "/trustee": {
        url: "/trustee",
        shows: async (page) => {
            await heading(page, "Braid Trustee Node")
            await expect(
                page.getByText(/braid-wasm loaded and thread pool initialized/)
            ).toBeVisible()
        },
        checkConsole: true,
    },
    "/messages": {url: "/messages", shows: (page) => text(page, "Messages")},
    "/settings/*": {url: "/settings", shows: settingsPage},
    "/sequent_backend_election_event": {url: "/sequent_backend_election_event", ...eventRedirect},
    "/sequent_backend_election_event/:id": {
        url: `/sequent_backend_election_event/${SHELL_IDS.event}`,
        shows: eventPage,
    },
    "/sequent_backend_election_event/:id/show": {
        url: `/sequent_backend_election_event/${SHELL_IDS.event}/show`,
        shows: eventPage,
    },
    "/sequent_backend_election_type/create": {
        url: "/sequent_backend_election_type/create",
        shows: (page) => text(page, "Create Election Type"),
    },
    "/sequent_backend_election_type/:id": {
        url: `/sequent_backend_election_type/${SHELL_IDS.electionType}`,
        shows: settingsPage,
    },
    "/sequent_backend_election_type/:id/show": {
        url: `/sequent_backend_election_type/${SHELL_IDS.electionType}/show`,
        shows: settingsPage,
    },
    "/sequent_backend_election": {url: "/sequent_backend_election", ...eventRedirect},
    "/sequent_backend_election/create": {
        url: "/sequent_backend_election/create",
        shows: (page) => text(page, "Create an Election"),
    },
    "/sequent_backend_election/:id": {
        url: `/sequent_backend_election/${SHELL_IDS.election}`,
        shows: electionPage,
    },
    "/sequent_backend_election/:id/show": {
        url: `/sequent_backend_election/${SHELL_IDS.election}/show`,
        shows: electionPage,
    },
    "/sequent_backend_contest": {url: "/sequent_backend_contest", ...eventRedirect},
    "/sequent_backend_contest/create": {
        url: "/sequent_backend_contest/create",
        shows: (page) => text(page, "Create a Contest"),
    },
    "/sequent_backend_contest/:id": {
        url: `/sequent_backend_contest/${SHELL_IDS.contest}`,
        shows: contestPage,
    },
    "/sequent_backend_contest/:id/show": {
        url: `/sequent_backend_contest/${SHELL_IDS.contest}/show`,
        shows: contestPage,
    },
    "/sequent_backend_candidate": {url: "/sequent_backend_candidate", ...eventRedirect},
    "/sequent_backend_candidate/create": {
        url: "/sequent_backend_candidate/create",
        shows: (page) => text(page, "Create a Candidate"),
    },
    "/sequent_backend_candidate/:id": {
        url: `/sequent_backend_candidate/${SHELL_IDS.candidate}`,
        shows: candidatePage,
    },
    "/sequent_backend_candidate/:id/show": {
        url: `/sequent_backend_candidate/${SHELL_IDS.candidate}/show`,
        shows: candidatePage,
    },
    "/sequent_backend_ballot_style": {
        url: "/sequent_backend_ballot_style",
        shows: async (page) => {
            await heading(page, "Ballot Styles")
            await cell(page, "PUBLISHED")
        },
    },
    "/sequent_backend_ballot_style/create": {
        url: "/sequent_backend_ballot_style/create",
        shows: (page) => text(page, "Ballot Style creation"),
    },
    "/sequent_backend_ballot_style/:id": {
        url: `/sequent_backend_ballot_style/${SHELL_IDS.ballotStyle}`,
        shows: async (page) => {
            await text(page, "Ballot Style configuration")
            await text(page, SHELL_IDS.ballotStyle)
        },
        checkConsole: true,
    },
    "/sequent_backend_area": {
        url: "/sequent_backend_area",
        shows: async (page) => {
            await cell(page, "North district")
            await cell(page, "Mayor contest")
        },
    },
    "/sequent_backend_area/create": {
        url: "/sequent_backend_area/create",
        shows: (page) => text(page, "Area configuration."),
    },
    "/sequent_backend_area/:id": {
        url: `/sequent_backend_area/${SHELL_IDS.area}`,
        shows: async (page, portal) => {
            await text(page, "Area configuration.")
            await expect(page.getByRole("textbox", {name: "Name", exact: true})).toHaveValue(
                "North district"
            )
            await expect(
                page.locator(".area-contest").getByText("Mayor contest", {exact: true})
            ).toBeVisible()
            expect(
                portal.graphql
                    .callsTo("sequent_backend_area_extended")
                    .map(({variables}) => variables)
            ).toEqual([{electionEventId: SHELL_IDS.event, areaId: SHELL_IDS.area}])
        },
    },
    "/sequent_backend_area_contest": {
        url: "/sequent_backend_area_contest",
        shows: async (page) => {
            await heading(page, "Area Contests")
            await cell(page, "North district")
        },
    },
    "/sequent_backend_area_contest/create": {
        url: "/sequent_backend_area_contest/create",
        shows: (page) => text(page, "Area Contest creation"),
    },
    "/sequent_backend_area_contest/:id": {
        url: `/sequent_backend_area_contest/${SHELL_IDS.areaContest}`,
        shows: (page) => text(page, SHELL_IDS.areaContest),
    },
    "/sequent_backend_tenant": {
        url: "/sequent_backend_tenant",
        shows: async (page) => {
            await heading(page, "Tenants")
            await cell(page, "synthetic")
        },
    },
    "/sequent_backend_tenant/create": {
        url: "/sequent_backend_tenant/create",
        shows: (page) => expect(page.getByRole("textbox", {name: "Slug"})).toBeVisible(),
    },
    "/sequent_backend_tenant/:id": {
        url: `/sequent_backend_tenant/${TENANT_ID}`,
        shows: async (page) => {
            await text(page, "Customer configuration")
            await text(page, TENANT_ID)
        },
    },
    "/sequent_backend_document": {
        url: "/sequent_backend_document",
        shows: async (page) => {
            await heading(page, "Documents")
            await cell(page, "results.pdf")
        },
    },
    "/sequent_backend_document/create": {
        url: "/sequent_backend_document/create",
        shows: (page) => text(page, "Document creation"),
    },
    "/sequent_backend_document/:id/show": {
        url: `/sequent_backend_document/${SHELL_IDS.document}/show`,
        shows: async (page) => {
            await text(page, "results.pdf")
            await text(page, "application/pdf")
        },
    },
    "/sequent_backend_notification": {
        url: "/sequent_backend_notification",
        shows: (page) => expect(page.getByRole("columnheader", {name: "Schedule"})).toBeVisible(),
    },
    "/sequent_backend_notification/:id": {
        url: `/sequent_backend_notification/${SHELL_IDS.notification}`,
        shows: (page) => expect(page.getByRole("columnheader", {name: "Schedule"})).toBeVisible(),
    },
    "/sequent_backend_template": {
        url: "/sequent_backend_template",
        shows: async (page) => {
            await text(page, "Templates")
            await cell(page, "ballot-receipt")
        },
    },
    "/sequent_backend_template/create": {
        url: "/sequent_backend_template/create",
        shows: (page) => text(page, "Create a Template"),
        checkConsole: true,
    },
    "/sequent_backend_template/:id": {
        url: `/sequent_backend_template/${SHELL_IDS.template}`,
        shows: async (page) => {
            await text(page, "Edit a Template")
            await expect(page.getByRole("textbox", {name: "Template Alias"})).toHaveValue(
                "ballot-receipt"
            )
        },
    },
    "/sequent_backend_scheduled_event": {
        url: "/sequent_backend_scheduled_event",
        shows: scheduledEventsPage,
    },
    "/sequent_backend_scheduled_event/:id": {
        url: `/sequent_backend_scheduled_event/${SHELL_IDS.scheduledEvent}`,
        shows: scheduledEventsPage,
    },
    "/sequent_backend_report": {url: "/sequent_backend_report", shows: reportsPage},
    "/sequent_backend_report/create": {url: "/sequent_backend_report/create", shows: reportsPage},
    "/sequent_backend_report/:id": {
        url: `/sequent_backend_report/${SHELL_IDS.report}`,
        shows: reportsPage,
    },
    "/user": {url: "/user", shows: usersPage},
    "/user/:id": {
        url: `/user/${SHELL_IDS.user}`,
        shows: async (page) => {
            await expect(
                page.getByRole("dialog").getByRole("textbox", {name: "Email", exact: true})
            ).toHaveValue("maria.lopez@example.test")
        },
    },
}

unit("the smoke matrix has exactly one entry per router route", () => {
    const routes = appRoutes().map((route) => route.path)
    expect(new Set(routes).size, "duplicate route paths").toBe(routes.length)
    expect(Object.keys(SMOKE).sort()).toEqual(routes.sort())
})

test.describe("route smoke", () => {
    test.use({roles: ["admin-user", ...Object.values(IPermissions)]})

    async function open(page: Page, portal: {origin: string}, smoke: Smoke) {
        const consoleErrors: string[] = []
        page.on("console", (message) => {
            if (message.type() === "error") consoleErrors.push(message.text())
        })
        // The administrator has already picked the tenant's event in the sidebar.
        await page.addInitScript(
            (id) => localStorage.setItem("selected-election-event-id", id),
            SHELL_IDS.event
        )
        const separator = smoke.url.includes("?") ? "&" : "?"
        await page.goto(`${portal.origin}${smoke.url}${separator}lang=en`)
        return consoleErrors
    }

    async function checkRoute(page: Page, portal: AdminPortal, smoke: Smoke) {
        serveAdminTenant(portal)
        if (smoke.url === "/trustee")
            await page.addInitScript(() => {
                Object.defineProperty(navigator, "hardwareConcurrency", {value: 2})
            })
        const unroute =
            smoke.url === "/trustee"
                ? await serveTrusteeWorkerHelper(page.context(), portal)
                : undefined
        let consoleErrors: string[] = []
        try {
            consoleErrors = await open(page, portal, smoke)
            await smoke.shows(page, portal)
            if (smoke.settlesAt)
                await expect(page).toHaveURL((url) => url.pathname === smoke.settlesAt)
            await nextFrames(page)
        } finally {
            await unroute?.()
            expect(portal.violations.list(), "unexpected service requests").toEqual([])
            expect(consoleErrors, "console errors").toEqual([])
        }
    }

    for (const [path, smoke] of Object.entries(SMOKE)) {
        test(`${path} renders its main content`, async ({page, portal}) => {
            await checkRoute(page, portal, smoke)
        })
        if (smoke.checkConsole)
            test(`${path} logs no console errors`, async ({page, portal}) => {
                await checkRoute(page, portal, smoke)
            })
    }
})
