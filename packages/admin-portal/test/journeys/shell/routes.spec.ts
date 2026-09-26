// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test as unit, type Page} from "@playwright/test"
import {IPermissions} from "../../../src/types/keycloak"
import {test, expect, TENANT_ID} from "../fixtures"
import {appRoutes} from "./routes"
import {serveAdminTenant, SHELL_IDS} from "./data"

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
    shows: (page: Page) => Promise<void>
    /** A defect that stops the route from rendering, pinned as an expected failure. */
    defect?: {reason: string; violation?: RegExp; operation?: string; consoleErrors?: RegExp[]}
    /**
     * Console errors of a pinned defect; any other console error still fails.
     * `settled` waits until the work that logs them has finished.
     */
    consoleDefect?: {reason: string; errors: RegExp[]; settled: (page: Page) => Promise<void>}
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
const upsertAreaDefect = {
    reason: "UpsertArea as a route view gets no electionEventId, sends an invalid query and renders nothing",
    operation: "sequent_backend_area_extended",
    violation:
        /^Invalid GraphQL operation sequent_backend_area_extended: Variable "\$electionEventId"/,
}

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
        shows: (page) => heading(page, "Braid Trustee Node"),
        consoleDefect: {
            reason: "the bundle fetches braid-wasm's rayon worker helper from a build-time file:// URL",
            errors: [
                /^Fetch API cannot load file:\/\/\/.*\/braid-wasm\/snippets\/.*\/workerHelpers\.no-bundler\.js/,
            ],
            settled: (page) =>
                expect(
                    page.getByText(/braid-wasm loaded and thread pool initialized|WASM init failed/)
                ).toBeVisible(),
        },
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
        consoleDefect: {
            reason: "EditBallotStyle renders the text status column in a JSON input",
            errors: [/^react-json-view error: src property must be a valid json object/],
            settled: nextFrames,
        },
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
        defect: upsertAreaDefect,
    },
    "/sequent_backend_area/:id": {
        url: `/sequent_backend_area/${SHELL_IDS.area}`,
        shows: (page) => text(page, "Area configuration."),
        defect: upsertAreaDefect,
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
        defect: {
            reason: "CreateTenant is a drawer that stays closed without its isDrawerOpen prop",
        },
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
        defect: {
            reason: "ShowDocument's JsonField reads labels from a missing record prop and crashes",
            consoleErrors: [
                /^TypeError: Cannot read properties of undefined \(reading 'labels'\)\n/,
            ],
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
        consoleDefect: {
            reason: "TemplateFormContent fetches a default template before any type is chosen",
            errors: [
                /^Error fetching template data: TypeError: Cannot read properties of undefined \(reading 'toLowerCase'\)/,
            ],
            settled: nextFrames,
        },
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
    "/user/:id": {url: `/user/${SHELL_IDS.user}`, shows: usersPage, defect: upsertAreaDefect},
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

    for (const [path, smoke] of Object.entries(SMOKE)) {
        test(`${path} renders its main content`, async ({page, portal}) => {
            serveAdminTenant(portal)
            const consoleErrors = await open(page, portal, smoke)
            const known = smoke.defect?.violation
            try {
                if (known) {
                    await expect
                        .poll(
                            () =>
                                portal.violations.list().some((entry) => known.test(entry)) ||
                                portal.graphql.callsTo(smoke.defect!.operation!).length > 0
                        )
                        .toBe(true)
                    const failures = portal.violations.list()
                    expect(
                        failures.filter((entry) => !known.test(entry)),
                        "unrelated service requests"
                    ).toEqual([])
                    if (failures.length) {
                        test.fail(true, smoke.defect!.reason)
                        expect(failures, "the route must send a valid scoped query").toEqual([])
                    }
                }
                expect(portal.violations.list(), "unexpected service requests").toEqual([])
                if (smoke.defect && !known) test.fail(true, smoke.defect.reason)
                await smoke.shows(page)
                if (smoke.settlesAt)
                    await expect(page).toHaveURL((url) => url.pathname === smoke.settlesAt)
                await (smoke.consoleDefect?.settled ?? nextFrames)(page)
                const expected = smoke.consoleDefect?.errors ?? []
                for (const error of expected)
                    expect(
                        consoleErrors.some((message) => error.test(message)),
                        `${error}`
                    ).toBe(true)
                expect(
                    consoleErrors.filter(
                        (message) => !expected.some((error) => error.test(message))
                    ),
                    "console errors"
                ).toEqual([])
                if (known) test.fail(true, smoke.defect!.reason)
            } finally {
                // Only the defect's own request is waived; any other one still fails teardown.
                if (known) {
                    const others = portal.violations.list().filter((entry) => !known.test(entry))
                    portal.violations.clear()
                    others.forEach((entry) => portal.violations.add(entry))
                }
                const expected = [
                    ...(smoke.consoleDefect?.errors ?? []),
                    ...(smoke.defect?.consoleErrors ?? []),
                ]
                const unrelated = consoleErrors.filter(
                    (message) => !expected.some((error) => error.test(message))
                )
                // A rendering defect must not absorb another failure from the route.
                if (unrelated.length) test.info().expectedStatus = "passed"
                expect(unrelated, "unrelated console errors").toEqual([])
            }
        })

        const consoleDefect = smoke.consoleDefect
        if (consoleDefect)
            test(`${path} logs no console errors`, async ({page, portal}) => {
                serveAdminTenant(portal)
                const consoleErrors = await open(page, portal, smoke)
                await smoke.shows(page)
                await consoleDefect.settled(page)
                expect(
                    consoleErrors.filter(
                        (message) => !consoleDefect.errors.some((error) => error.test(message))
                    ),
                    "unrelated console errors"
                ).toEqual([])
                test.fail(true, consoleDefect.reason)
                expect(consoleErrors).toEqual([])
            })
    }
})
