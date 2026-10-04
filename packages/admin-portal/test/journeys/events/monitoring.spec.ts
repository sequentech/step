// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, type AdminPortal} from "../fixtures"
import {EVENT_ID, EVENT_ROLES, FIXED_TIME, listOf, mockEvent, openEvent} from "./data"
import {election} from "./ivr-data"

// Every value below is invented; the shapes are those of Harvest's monitoring actions.
const MADRID = "50000000-0000-4000-8000-000000000011"
const PARIS = "50000000-0000-4000-8000-000000000012"
const SNAPSHOT = {revision: 41, as_of: FIXED_TIME, checked_at: FIXED_TIME}
/** Fetched from inside a chart if the frame let it: nothing may ever ask for it. */
const LEAK = "https://leak.invalid/pixel.png"

test.use({
    roles: [
        ...EVENT_ROLES,
        "admin-dashboard-view",
        "election-dashboard-tab",
        "monitoring-view",
        "monitoring-configure",
    ],
    // Playwright's service worker block is an init script that reads
    // `navigator.serviceWorker`, which throws in the chart's sandboxed frame.
    // The portal registers no service worker.
    serviceWorkers: "allow",
})

const SUMMARY = {
    id: "turnout-summary",
    title: "Voter turnout",
    source: "voter_turnout",
    requirements: ["SW-F-0259"],
    query: {template: "summary", measures: ["registered", "voted"]},
    chart: {charts: {voted: {type: "kpi", query: "data", value: "voted"}}, rows: ["voted"]},
    height: 160,
}
const BY_GROUP = {
    id: "turnout-by-group",
    title: "Turnout by group",
    source: "voter_turnout",
    requirements: ["SW-F-0260"],
    selectors: {
        breakdown: {
            label: "Breakdown",
            options: {sex: "Sex", age_band: "Age"},
            default: "age_band",
        },
    },
    query: {
        template: "by_group",
        group_by: {selector: "breakdown"},
        ratio: ["voted", "registered"],
    },
    chart: {charts: {bars: {type: "bar", query: "data", x: "group", y: "pct"}}, rows: ["bars"]},
    height: 240,
}
const BY_GROUP_YAML = `id: turnout-by-group
title: Turnout by group
source: voter_turnout
requirements: [SW-F-0260]
selectors:
  breakdown:
    label: Breakdown
    options: {sex: Sex, age_band: Age}
    default: age_band
query:
  template: by_group
  group_by: {selector: breakdown}
  ratio: [voted, registered]
height: 240
chart:
  charts:
    bars:
      type: bar
      query: data
      x: group
      y: pct
  rows: [bars]
`
const DASHBOARD = {
    id: "overview",
    title: "Monitoring overview",
    requirements: ["SW-F-0247"],
    order: 0,
    selectors: ["region", "post", "country"],
    layout: [
        {widget: SUMMARY.id, width: 12},
        {widget: BY_GROUP.id, width: 12},
    ],
}
const POSTS = [
    {key: MADRID, label: "Madrid", region: "north"},
    {key: PARIS, label: "Paris", region: "south"},
]
const SOURCE = {
    counting_unit: "DISTINCT_VOTERS",
    measures: ["registered", "voted"],
    templates: ["summary", "by_group"],
    dimensions: ["region", "post", "country", "sex", "age_band"],
    producer: "CONNECTED",
    reason: null,
}
/** Each Post's own figures, so a chart shows whose they are. */
const VOTED: Record<string, number> = {[MADRID]: 1111, [PARIS]: 2222, all: 3333}

function chart(voted: number): string {
    // A renderer that let a remote reference through: the frame must still load nothing.
    return (
        `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100" width="100%">` +
        `<image href="${LEAK}" width="10" height="10"/>` +
        `<rect x="10" y="10" width="80" height="80" fill="#2c6fbb"/>` +
        `<text x="100" y="60">${voted}</text></svg>`
    )
}

const table = (voted: number) => ({
    columns: [
        {name: "group", kind: "text"},
        {name: "voted", kind: "integer"},
    ],
    rows: [["18–29", voted]],
})

/** Registers the view's monitoring actions; `electionId` in a request pins its Post. */
function mockMonitoring(portal: AdminPortal) {
    portal.graphql.on("MonitoringListDashboards", () => ({
        data: {
            monitoringListDashboards: {
                mode: "CONFIGURED",
                dashboards: [
                    {
                        id: DASHBOARD.id,
                        title: DASHBOARD.title,
                        requirements: DASHBOARD.requirements,
                        widget_count: DASHBOARD.layout.length,
                    },
                ],
                snapshot: SNAPSHOT,
            },
        },
    }))
    portal.graphql.on("MonitoringGetDashboard", ({variables}) => ({
        data: {
            monitoringGetDashboard: {
                dashboard: DASHBOARD,
                dashboard_revision: 3,
                widgets: {
                    [SUMMARY.id]: {definition: SUMMARY, revision: 2},
                    [BY_GROUP.id]: {definition: BY_GROUP, revision: 2},
                },
                theme: {id: "default", revision: 1},
                settings: {time_zone: "UTC", unknown_label: "Unknown", selectors: {}},
                settings_revision: 1,
                scope_options: {
                    regions: [
                        {key: "north", label: "North"},
                        {key: "south", label: "South"},
                    ],
                    posts: POSTS,
                    countries: [{key: "ES", label: "Spain"}],
                },
                restricted: false,
                pinned_post: variables.electionId ?? null,
                sources: {voter_turnout: SOURCE},
                snapshot: SNAPSHOT,
                event_days: ["2026-01-15"],
            },
        },
    }))
    portal.graphql.on("MonitoringRenderWidget", ({variables}) => {
        const scope = (variables.scope ?? {}) as Record<string, string>
        const post = (variables.electionId as string | null) ?? scope.post ?? "all"
        const voted = VOTED[post] ?? 0
        return {
            data: {
                monitoringRenderWidget: {
                    state: "RENDERED",
                    reason: null,
                    svg: chart(voted),
                    table: table(voted),
                    tables: null,
                    notices: [],
                    diagnostics: [],
                    ignored_selectors: [],
                    render_ms: 12,
                    snapshot_revision: SNAPSHOT.revision,
                    as_of: FIXED_TIME,
                },
            },
        }
    })
}

/** Registers the editor's actions: the widget is at revision 2 until it is saved. */
function mockEditor(portal: AdminPortal) {
    let revision = 2
    let yaml = BY_GROUP_YAML
    portal.graphql.on("MonitoringEditorGetConfig", ({variables}) => ({
        data: {
            monitoringGetConfig: {
                yaml,
                revision: (variables.revision as number | null) ?? revision,
                origin: "EDITOR",
                author: {id: "u-ana", name: "Ana Reyes"},
                created_at: FIXED_TIME,
            },
        },
    }))
    portal.graphql.on("MonitoringEditorRenderWidget", () => ({
        data: {
            monitoringRenderWidget: {
                state: "RENDERED",
                reason: null,
                svg: chart(VOTED.all),
                table: table(VOTED.all),
                notices: [],
                diagnostics: [],
                ignored_selectors: [],
                render_ms: 42,
                snapshot_revision: SNAPSHOT.revision,
                as_of: FIXED_TIME,
            },
        },
    }))
    portal.graphql.on("MonitoringEditorValidateConfig", () => ({
        data: {monitoringValidateConfig: {result: "VALID", problems: [], preview: null}},
    }))
    portal.graphql.on("MonitoringEditorSaveConfig", ({variables}) => {
        revision += 1
        yaml = String(variables.yaml)
        return {data: {monitoringSaveConfig: {revision, generation: 7, warnings: []}}}
    })
}

/** Every request made from inside a chart frame, and every one to the leak URL. */
function watchFrames(page: Page) {
    const leaked: string[] = []
    page.context().on("request", (request) => {
        const frame = request.frame()
        const fromChart = frame !== page.mainFrame() && frame.url().startsWith("about:srcdoc")
        if (fromChart || request.url().startsWith(LEAK)) leaked.push(request.url())
    })
    return leaked
}

async function openConfigure(page: Page) {
    const card = page.locator(`[data-widget-id='${BY_GROUP.id}']`)
    await card.getByRole("button", {name: `Actions for ${BY_GROUP.title}`}).click()
    await page.getByRole("menuitem", {name: "Configure widget"}).click()
    const dialog = page.getByRole("dialog", {name: /Configure widget/})
    await expect(dialog.getByRole("textbox", {name: "Title"})).toHaveValue(BY_GROUP.title)
    return dialog
}

for (const width of [390, 1280]) {
    test.describe(`at ${width} px`, () => {
        test.use({viewport: {width, height: 900}})

        test("draws the configured dashboard's widgets in frames that load nothing", async ({
            page,
            portal,
        }) => {
            const leaked = watchFrames(page)
            mockEvent(portal)
            mockMonitoring(portal)
            await openEvent(page, portal)
            await expect(page.getByRole("heading", {name: DASHBOARD.title})).toBeVisible()
            for (const widget of [SUMMARY, BY_GROUP]) {
                const card = page.locator(`[data-widget-id='${widget.id}']`)
                await expect(card.getByRole("heading", {name: widget.title})).toBeVisible()
                const frame = card.locator("iframe")
                await expect(frame).toHaveAttribute("title", `${widget.title} chart`)
                // An empty sandbox: no script, no same origin, no navigation.
                await expect(frame).toHaveAttribute("sandbox", "")
                await expect(frame).toHaveAttribute("srcdoc", /3333/)
                await expect(frame).toBeVisible()
            }
            // The event page offers the whole scope.
            for (const selector of ["Region", "Post", "Country"])
                await expect(page.getByRole("combobox", {name: selector})).toBeVisible()
            const renders = portal.graphql.callsTo("MonitoringRenderWidget")
            expect(renders.map((call) => call.variables.widgetId).sort()).toEqual(
                expect.arrayContaining([SUMMARY.id, BY_GROUP.id])
            )
            for (const call of renders) {
                expect(call.variables).toMatchObject({
                    electionEventId: EVENT_ID,
                    electionId: null,
                    dashboardId: DASHBOARD.id,
                    snapshotRevision: SNAPSHOT.revision,
                })
            }
            // Let a frame that could fetch have done so before looking.
            await page.waitForLoadState("networkidle")
            expect(leaked).toEqual([])
        })

        test("saves a form edit as YAML at the revision it was loaded from", async ({
            page,
            portal,
        }) => {
            mockEvent(portal)
            mockMonitoring(portal)
            mockEditor(portal)
            await openEvent(page, portal)
            const dialog = await openConfigure(page)
            await dialog.getByRole("textbox", {name: "Title"}).fill("Turnout per group")
            await dialog.getByRole("tab", {name: "YAML"}).click()
            await expect(dialog.getByRole("textbox", {name: "YAML"})).toContainText(
                "title: Turnout per group"
            )
            await dialog.getByRole("button", {name: "Save widget"}).click()
            await expect(dialog.getByText("Widget saved as revision 3")).toBeVisible()
            const [save] = portal.graphql.callsTo("MonitoringEditorSaveConfig")
            expect(save.variables).toMatchObject({
                election_event_id: EVENT_ID,
                kind: "widget",
                key: BY_GROUP.id,
                expected_revision: 2,
            })
            expect(String(save.variables.yaml)).toContain("title: Turnout per group")
            expect(String(save.variables.yaml)).not.toContain("title: Turnout by group")
            // The editor's actions run as the configure role, not the viewer's default.
            expect(save.headers["x-hasura-role"]).toBe("monitoring-configure")
        })

        test("opens the conflict dialog when someone saved first", async ({page, portal}) => {
            mockEvent(portal)
            mockMonitoring(portal)
            mockEditor(portal)
            portal.graphql.once("MonitoringEditorSaveConfig", () => ({
                errors: [
                    {
                        message: "conflict",
                        extensions: {
                            code: "CONFLICT",
                            current_revision: 3,
                            author: {id: "u-luis", name: "Luis Santos"},
                            time: FIXED_TIME,
                        },
                    },
                ],
            }))
            await openEvent(page, portal)
            const dialog = await openConfigure(page)
            await dialog.getByRole("textbox", {name: "Title"}).fill("Turnout per group")
            await dialog.getByRole("button", {name: "Save widget"}).click()
            const conflict = page.getByRole("dialog", {name: "Someone saved first"})
            await expect(conflict).toBeVisible()
            await expect(conflict.getByText(/Luis Santos saved revision 3/)).toBeVisible()
            expect(portal.graphql.callsTo("MonitoringEditorSaveConfig")[0].variables).toMatchObject(
                {expected_revision: 2}
            )
            await expect(conflict.getByRole("button", {name: "Keep editing"})).toBeVisible()
        })

        test("pins an election's Post and shows only its figures", async ({page, portal}) => {
            const leaked = watchFrames(page)
            mockEvent(portal)
            mockMonitoring(portal)
            const madrid = election(MADRID, "Madrid")
            portal.graphql.on(
                "sequent_backend_election",
                listOf("sequent_backend_election", [madrid])
            )
            portal.graphql.on("election_tree", () => ({
                data: {sequent_backend_election: [madrid]},
            }))
            portal.graphql.on("contest_tree", () => ({data: {sequent_backend_contest: []}}))
            await page.goto(`${portal.origin}/sequent_backend_election/${MADRID}?lang=en`)
            await expect(page.getByRole("heading", {name: DASHBOARD.title})).toBeVisible()
            for (const widget of [SUMMARY, BY_GROUP]) {
                const frame = page.locator(`[data-widget-id='${widget.id}'] iframe`)
                await expect(frame).toHaveAttribute("sandbox", "")
                await expect(frame).toHaveAttribute("srcdoc", /1111/)
                await expect(frame).not.toHaveAttribute("srcdoc", /2222|3333/)
            }
            // The Post is the election's: neither it nor its Region is offered.
            await expect(page.getByRole("combobox", {name: "Country"})).toBeVisible()
            await expect(page.getByRole("combobox", {name: "Post"})).toHaveCount(0)
            await expect(page.getByRole("combobox", {name: "Region"})).toHaveCount(0)
            const card = page.locator(`[data-widget-id='${BY_GROUP.id}']`)
            await card.getByRole("button", {name: `Actions for ${BY_GROUP.title}`}).click()
            await page.getByRole("menuitem", {name: "View data"}).click()
            await expect(page.getByRole("dialog").getByText(/North · Madrid/)).toBeVisible()
            for (const call of [
                ...portal.graphql.callsTo("MonitoringListDashboards"),
                ...portal.graphql.callsTo("MonitoringGetDashboard"),
                ...portal.graphql.callsTo("MonitoringRenderWidget"),
            ]) {
                expect(call.variables).toMatchObject({
                    electionEventId: EVENT_ID,
                    electionId: MADRID,
                })
                expect((call.variables.scope as Record<string, string> | undefined)?.post).toBe(
                    undefined
                )
            }
            await page.waitForLoadState("networkidle")
            expect(leaked).toEqual([])
        })
    })
}
