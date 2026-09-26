// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, type AdminPortal} from "../fixtures"
import {
    electionEvent,
    EVENT_ID,
    EVENT_ROLES,
    FIXED_TIME,
    mockEvent,
    openEvent,
    TENANT_ID,
    type Row,
} from "./data"

const TASK_ID = "44444444-4444-4444-8444-444444444444"
const DOCUMENT_ID = "33333333-3333-4333-8333-333333333333"
test.use({roles: [...EVENT_ROLES, "election-event-data-tab"]})

/** Serves an event that absorbs `update_sequent_backend_election_event`, like Hasura would. */
function editableEvent(portal: AdminPortal, presentation: Row = {}) {
    let event = electionEvent(
        {alias: "Council"},
        {
            i18n: {
                en: {
                    name: "Council election",
                    alias: "Council",
                    description: "Annual council election",
                },
            },
            ...presentation,
        }
    )
    mockEvent(portal, () => event)
    portal.graphql.on("SetCustomUrls", () => ({
        data: {set_custom_urls: {success: true, message: "Custom URL updated"}},
    }))
    portal.graphql.on("SetVoterAuthentication", () => ({
        data: {set_voter_authentication: {success: true, message: "Updated"}},
    }))
    portal.graphql.on("update_sequent_backend_election_event", ({variables}) => {
        event = {...event, ...(variables._set as Row)}
        return {
            data: {update_sequent_backend_election_event: {affected_rows: 1, returning: [event]}},
        }
    })
}

function customUrlCalls(portal: AdminPortal, prefixes = {login: "", enrollment: "", saml: ""}) {
    const voting = `${portal.origin}/voting/tenant/${TENANT_ID}/event/${EVENT_ID}`
    return [
        {
            origin: `https://${prefixes.login}.vote.example`,
            redirect_to: `${voting}/login`,
            dns_prefix: prefixes.login,
            election_id: EVENT_ID,
            key: "login",
        },
        {
            origin: `https://${prefixes.enrollment}.vote.example`,
            redirect_to: `${voting}/enroll`,
            dns_prefix: prefixes.enrollment,
            election_id: EVENT_ID,
            key: "enrollment",
        },
        {
            origin: `https://${prefixes.saml}.vote.example`,
            redirect_to: `${portal.origin}/keycloak/realms/tenant-${TENANT_ID}-event-${EVENT_ID}/broker/simplesamlphp/endpoint`,
            dns_prefix: prefixes.saml,
            election_id: EVENT_ID,
            key: "saml",
        },
    ]
}

/** The presentation defaults the form writes back on the first save of a bare event. */
const SAVED_PRESENTATION_DEFAULTS = {
    language_conf: {
        enabled_language_codes: ["en"],
        default_language_code: "en",
        language_detection_policy: "browser-detect",
    },
    elections_order: "alphabetical",
    voting_portal_countdown_policy: {policy: "NO_COUNTDOWN"},
    custom_urls: {},
    skip_election_list: false,
    show_user_profile: false,
    show_cast_vote_logs: "hide-logs-tab",
    automatic_recount_policy: "disabled",
    materials: {policy: "off"},
    contest_encryption_policy: "single-contest",
    locked_down: "not-locked-down",
    decoded_ballot_inclusion_policy: "not-included",
    ceremonies_policy: "manual-ceremonies",
    weighted_voting_policy: "disabled-weighted-voting",
    delegated_voting_policy: "disabled",
    voting_portal_datetime_format: "legacy-gb-24h",
    voter_signing_policy: "no-signature",
    voter_certificate_policy: "disabled",
}

async function save(page: Page, portal: AdminPortal) {
    const before = portal.graphql.callsTo("update_sequent_backend_election_event").length
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await expect
        .poll(() => portal.graphql.callsTo("update_sequent_backend_election_event").length)
        .toBe(before + 1)
    return portal.graphql.callsTo("update_sequent_backend_election_event")[before].variables
}

test.beforeEach(({portal}) => {
    portal.settings.CUSTOM_URLS_DOMAIN_NAME = "vote.example"
})

test("saves general edits through custom URLs, voter authentication and the event update", async ({
    page,
    portal,
}) => {
    editableEvent(portal)
    await openEvent(page, portal)
    const name = page.getByRole("textbox", {name: "Name", exact: true})
    await expect(name).toHaveValue("Council election")
    await expect(page.getByRole("textbox", {name: "Alias", exact: true})).toHaveValue("Council")
    await expect(page.getByRole("textbox", {name: "Description", exact: true})).toHaveValue(
        "Annual council election"
    )
    await expect(page.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
    await name.fill("City council 2026")
    await page.getByRole("textbox", {name: "Description", exact: true}).fill("Renewed council")

    const update = await save(page, portal)
    expect(update).toEqual({
        _set: {
            description: "Renewed council",
            presentation: {
                ...SAVED_PRESENTATION_DEFAULTS,
                i18n: {
                    en: {
                        name: "City council 2026",
                        alias: "Council",
                        description: "Renewed council",
                    },
                },
            },
        },
        where: {id: {_eq: EVENT_ID}},
    })
    // Custom URLs and voter authentication are always sent before the event update.
    expect(portal.graphql.callsTo("SetCustomUrls").map(({variables}) => variables)).toEqual(
        customUrlCalls(portal)
    )
    expect(
        portal.graphql.callsTo("SetVoterAuthentication").map(({variables}) => variables)
    ).toEqual([{electionEventId: EVENT_ID, enrollment: "", otp: ""}])
    for (const call of portal.graphql.callsTo("SetCustomUrls"))
        expect(call.headers["x-hasura-role"]).toBe("election-event-write")
    // The edit refreshes instead of notifying: the saved values come back and Save disables.
    await expect(page.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
    await expect(name).toHaveValue("City council 2026")
})

test("exports an encrypted archive, tracks its task and reveals the generated password", async ({
    context,
    page,
    portal,
}) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"], {origin: portal.origin})
    editableEvent(portal)
    let status = "IN_PROGRESS"
    const task = () => ({
        id: TASK_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name: "Export Election Event",
        type: "EXPORT_ELECTION_EVENT",
        execution_status: status,
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: status === "IN_PROGRESS" ? null : FIXED_TIME,
        executed_by_user: "synthetic-admin",
        annotations: {},
        labels: {},
        logs: [],
    })
    portal.settings.QUERY_POLL_INTERVAL_MS = 100
    portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
    portal.graphql.on("ExportElectionEvent", () => ({
        data: {
            export_election_event: {
                password: "Generated-Archive-Pass",
                document_id: DOCUMENT_ID,
                task_execution: task(),
            },
        },
    }))
    portal.graphql.on("GetTaskById", () => ({data: {sequent_backend_tasks_execution: [task()]}}))
    await openEvent(page, portal)
    await page.getByRole("button", {name: "Export", exact: true}).click()
    const dialog = page.getByRole("dialog")
    await expect(dialog.getByText("Export Election Event", {exact: true})).toBeVisible()
    await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
    await expect(dialog).toHaveCount(0)
    expect(portal.graphql.callsTo("ExportElectionEvent")).toHaveLength(0)

    await page.getByRole("button", {name: "Export", exact: true}).click()
    await dialog.getByRole("checkbox", {name: "Encrypt with Password"}).check()
    await dialog.getByRole("checkbox", {name: "Include Voters"}).check()
    // Tally needs the bulletin board, so ticking it ticks the board as well.
    await dialog.getByRole("checkbox", {name: "Tally", exact: true}).check()
    await expect(dialog.getByRole("checkbox", {name: "Bulletin Board"})).toBeChecked()
    await dialog.getByRole("button", {name: "Export", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("ExportElectionEvent").length).toBe(1)
    const call = portal.graphql.callsTo("ExportElectionEvent")[0]
    expect(call.variables).toEqual({
        electionEventId: EVENT_ID,
        exportConfigurations: {
            is_encrypted: true,
            encrypt_with_password: true,
            include_voters: true,
            activity_logs: false,
            bulletin_board: true,
            publications: false,
            s3_files: false,
            scheduled_events: false,
            reports: false,
            applications: false,
            tally: true,
            include_certificates: false,
        },
    })
    expect(call.headers["x-hasura-role"]).toBe("election-event-read")
    const password = page.getByRole("dialog", {name: "Password"})
    await expect(password.getByRole("textbox").first()).toHaveValue("Generated-Archive-Pass")
    await password.getByRole("button", {name: "Copy Password"}).first().click()
    await expect(page.getByText("Password copied to clipboard", {exact: true})).toBeVisible()
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("Generated-Archive-Pass")
    await expect(page.getByText("Task: Export Election Event", {exact: true})).toBeVisible()
    await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
    expect(portal.graphql.callsTo("GetTaskById")[0].variables).toEqual({task_id: TASK_ID})
    await password.getByRole("button", {name: "Ok", exact: true}).click()
    await expect(password).toHaveCount(0)
    status = "SUCCESS"
    await page.clock.runFor(250)
    await expect(page.getByText("SUCCESS", {exact: true})).toBeVisible()
})

async function choose(page: Page, combobox: RegExp, option: string) {
    await page.getByRole("combobox", {name: combobox}).click()
    await page.getByRole("option", {name: option, exact: true}).click()
}

test("saves ballot design, channel, language and advanced policy choices in one update", async ({
    page,
    portal,
}) => {
    editableEvent(portal)
    await openEvent(page, portal)
    await page.getByRole("button", {name: "Ballot Design", exact: true}).click()
    await page.getByRole("switch", {name: "Skip Election List Screen"}).check()
    await page.getByRole("switch", {name: "Show User Profile"}).check()
    await choose(page, /^Presentation elections order/, "Random")
    await choose(page, /^Show Cast Vote Logs Tab/, "Show Cast Vote Logs Tab")
    await page.getByRole("textbox", {name: "Logo URL"}).fill("https://cdn.example/logo.svg")
    await page
        .getByRole("textbox", {name: "Redirect Finish URL", exact: true})
        .fill("https://council.example/thanks")
    await page.getByRole("textbox", {name: "Custom CSS"}).fill("header { color: navy; }")
    await page.getByRole("button", {name: "Voting Channels Allowed", exact: true}).click()
    await page.getByRole("switch", {name: "Kiosk"}).check()
    await page.getByRole("radio", {name: "Enabled"}).check()
    await page.getByRole("button", {name: "Language", exact: true}).click()
    await choose(page, /^Language Detection Policy/, "Force Default")
    await page.getByRole("button", {name: "Advanced Configurations", exact: true}).click()
    await choose(page, /^Contest encryption policy/, "Multiple Contests")
    await choose(page, /^Include decoded ballots/, "Include")
    await choose(page, /^Keys\/Tally Ceremonies Policy/, "Allow Automatic Ceremonies")
    await choose(page, /^Weighted Voting Policy/, "Weighted Voting for Areas")
    await choose(page, /^Delegated Voting Policy/, "Enabled")
    await choose(page, /^Voting Portal date & time format/, "ISO Local (yyyy-MM-dd HH:mm)")
    await choose(page, /^Voting Portal Countdown policy/, "Countdown with alert")
    const countdown = page.getByRole("spinbutton", {
        name: "time in seconds before expiration to show countdown",
    })
    await expect(countdown).toBeEnabled()
    await countdown.fill("30")
    await page
        .getByRole("spinbutton", {name: "time in seconds before expiration to show Logout alert"})
        .fill("120")
    await choose(page, /^Voter Signing Policy/, "With signature")
    await choose(page, /^Voter Digital Certificate Policy/, "Enabled")
    await choose(page, /^Enrollment/, "Enabled")
    await choose(page, /^OTP/, "Disabled")

    const update = await save(page, portal)
    expect(update).toEqual({
        _set: {
            presentation: {
                ...SAVED_PRESENTATION_DEFAULTS,
                i18n: {
                    en: {
                        name: "Council election",
                        alias: "Council",
                        description: "Annual council election",
                    },
                },
                language_conf: {
                    enabled_language_codes: ["en"],
                    default_language_code: "en",
                    language_detection_policy: "force-default",
                },
                skip_election_list: true,
                show_user_profile: true,
                elections_order: "random",
                show_cast_vote_logs: "show-logs-tab",
                logo_url: "https://cdn.example/logo.svg",
                redirect_finish_url: "https://council.example/thanks",
                css: "header { color: navy; }",
                automatic_recount_policy: "enabled",
                contest_encryption_policy: "multiple-contests",
                decoded_ballot_inclusion_policy: "included",
                ceremonies_policy: "automated-ceremonies",
                weighted_voting_policy: "areas-weighted-voting",
                delegated_voting_policy: "enabled",
                voting_portal_datetime_format: "iso-local",
                voting_portal_countdown_policy: {
                    policy: "COUNTDOWN_WITH_ALERT",
                    countdown_anticipation_secs: 30,
                    countdown_alert_anticipation_secs: 120,
                },
                voter_signing_policy: "with-signature",
                voter_certificate_policy: "enabled",
                enrollment: "enabled",
                otp: "disabled",
            },
            voting_channels: {online: true, kiosk: true, early_voting: false, telephone: false},
        },
        where: {id: {_eq: EVENT_ID}},
    })
    expect(
        portal.graphql.callsTo("SetVoterAuthentication").map(({variables}) => variables)
    ).toEqual([{electionEventId: EVENT_ID, enrollment: "enabled", otp: "disabled"}])
})

/**
 * Records unhandled promise rejections instead of letting them surface as page errors, for the
 * tests that pin a rejected save: they assert the list explicitly, so nothing is hidden.
 */
async function captureRejections(page: Page) {
    await page.addInitScript(() => {
        const log: string[] = []
        Object.assign(window, {unhandledRejections: log})
        window.addEventListener("unhandledrejection", (event) => {
            log.push(String(event.reason instanceof Error ? event.reason.message : event.reason))
            event.preventDefault()
        })
    })
    return () =>
        page.evaluate(
            () => (window as unknown as {unhandledRejections: string[]}).unhandledRejections
        )
}

test("applies typed custom URL prefixes and reports each record's outcome", async ({
    page,
    portal,
}) => {
    editableEvent(portal)
    portal.graphql.on("SetCustomUrls", ({variables}) => ({
        data: {
            set_custom_urls:
                variables.key === "saml"
                    ? {success: false, message: "SAML prefix already taken"}
                    : {success: true, message: "Custom URL updated"},
        },
    }))
    await openEvent(page, portal)
    await page.getByRole("button", {name: "Custom URLs Prefix", exact: true}).click()
    const urls = page.getByRole("region").filter({hasText: "Login:"})
    await expect(urls.getByText(".vote.example", {exact: true})).toHaveCount(3)
    await urls.getByRole("textbox").nth(0).fill("council")
    await urls.getByRole("textbox").nth(1).fill("council-enrol")
    await urls.getByRole("textbox").nth(2).fill("council-saml")
    const update = await save(page, portal)
    expect(portal.graphql.callsTo("SetCustomUrls").map(({variables}) => variables)).toEqual(
        customUrlCalls(portal, {
            login: "council",
            enrollment: "council-enrol",
            saml: "council-saml",
        })
    )
    expect((update._set as {presentation: Row}).presentation.custom_urls).toEqual({
        login: "council",
        enrollment: "council-enrol",
        saml: "council-saml",
    })
    await expect(urls.getByText("SAML prefix already taken", {exact: true})).toBeVisible()
    await expect(urls.getByText("SUCCESS", {exact: true})).toHaveCount(2)
    await expect(urls.getByText("ERROR", {exact: true})).toHaveCount(1)
})

// The prefixes sent on save come from local state seeded with empty strings, not from the
// record, so the backend rewrites the event's existing DNS records to an empty prefix.
test.fail(
    "keeps the saved custom URL prefixes when saving an unrelated change",
    async ({page, portal}) => {
        editableEvent(portal, {
            custom_urls: {login: "council", enrollment: "council-enrol", saml: "council-saml"},
        })
        await openEvent(page, portal)
        await page.getByRole("button", {name: "Custom URLs Prefix", exact: true}).click()
        await expect(
            page.getByRole("region").filter({hasText: "Login:"}).getByRole("textbox").first()
        ).toHaveValue("council")
        await page.getByRole("button", {name: "General", exact: true}).click()
        await page.getByRole("textbox", {name: "Description", exact: true}).fill("Renewed council")
        await save(page, portal)
        expect(portal.graphql.callsTo("SetCustomUrls").map(({variables}) => variables)).toEqual(
            customUrlCalls(portal, {
                login: "council",
                enrollment: "council-enrol",
                saml: "council-saml",
            })
        )
    }
)

// A failed SetVoterAuthentication is not awaited, so its error escapes as an unhandled rejection.
test.fail(
    "handles a failed voter authentication update without an unhandled rejection",
    async ({page, portal}) => {
        const rejections = await captureRejections(page)
        editableEvent(portal)
        portal.graphql.on("SetVoterAuthentication", () => ({
            errors: [{message: "Keycloak unavailable"}],
        }))
        await openEvent(page, portal)
        await page.getByRole("textbox", {name: "Description", exact: true}).fill("Renewed council")
        await save(page, portal)
        expect(portal.graphql.callsTo("SetVoterAuthentication")).toHaveLength(1)
        await expect(page.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
        expect(await rejections()).toEqual([])
    }
)

const CONFIGURED_POLICY = {
    configured: true,
    minimum_length: 10,
    maximum_length: 64,
    include_uppercase: true,
    include_lowercase: true,
    include_digits: false,
    include_special_characters: false,
}

function passwordPolicy(portal: AdminPortal, policy: Row, updated = true) {
    portal.graphql.on("GetRealmPasswordPolicy", () => ({
        data: {get_realm_password_policy: policy},
    }))
    portal.graphql.on("UpdateRealmPasswordPolicy", () => ({
        data: {update_realm_password_policy: {updated}},
    }))
}

test("updates a configured password policy", async ({page, portal}) => {
    editableEvent(portal)
    passwordPolicy(portal, CONFIGURED_POLICY)
    await openEvent(page, portal)
    await page.getByRole("button", {name: "Password Policy", exact: true}).click()
    const minimum = page.getByRole("spinbutton", {name: "Minimum length"})
    await expect(minimum).toHaveValue("10")
    await expect(page.getByRole("spinbutton", {name: "Maximum length"})).toHaveValue("64")
    await expect(page.getByRole("checkbox", {name: "Include digits"})).not.toBeChecked()
    await expect(page.getByText(/No password policy is configured/)).toHaveCount(0)
    expect(portal.graphql.callsTo("GetRealmPasswordPolicy")[0]).toMatchObject({
        variables: {election_event_id: EVENT_ID},
        headers: {"x-hasura-role": "election-event-read"},
    })
    await minimum.fill("14")
    await page.getByRole("spinbutton", {name: "Maximum length"}).fill("100")
    await page.getByRole("checkbox", {name: "Include digits"}).check()
    await page.getByRole("checkbox", {name: "Include uppercase letters"}).uncheck()
    await save(page, portal)
    const updates = portal.graphql.callsTo("UpdateRealmPasswordPolicy")
    expect(updates.map(({variables}) => variables)).toEqual([
        {
            election_event_id: EVENT_ID,
            minimum_length: 14,
            maximum_length: 100,
            include_uppercase: false,
            include_lowercase: true,
            include_digits: true,
            include_special_characters: false,
        },
    ])
    expect(updates[0].headers["x-hasura-role"]).toBe("election-event-write")
    await expect.poll(() => portal.graphql.callsTo("GetRealmPasswordPolicy").length).toBe(2)
})

test("applies the defaults when saving an unconfigured password policy", async ({page, portal}) => {
    editableEvent(portal)
    passwordPolicy(portal, {...CONFIGURED_POLICY, configured: false})
    await openEvent(page, portal)
    await page.getByRole("button", {name: "Password Policy", exact: true}).click()
    await expect(
        page.getByText("No password policy is configured. Saving will apply the defaults below.", {
            exact: true,
        })
    ).toBeVisible()
    await page.getByRole("checkbox", {name: "Include special characters"}).check()
    await save(page, portal)
    expect(
        portal.graphql.callsTo("UpdateRealmPasswordPolicy").map(({variables}) => variables)
    ).toEqual([
        {
            election_event_id: EVENT_ID,
            minimum_length: 10,
            maximum_length: 64,
            include_uppercase: true,
            include_lowercase: true,
            include_digits: false,
            include_special_characters: true,
        },
    ])
})

// An aborted save rethrows from the form's transform, which surfaces as an unhandled rejection.
test.fail(
    "blocks an invalid password length range without an unhandled rejection",
    async ({page, portal}) => {
        const rejections = await captureRejections(page)
        editableEvent(portal)
        passwordPolicy(portal, CONFIGURED_POLICY)
        await openEvent(page, portal)
        await page.getByRole("button", {name: "Password Policy", exact: true}).click()
        await page.getByRole("spinbutton", {name: "Minimum length"}).fill("80")
        await expect(
            page.getByText("Minimum length cannot exceed maximum length.", {exact: true}).first()
        ).toBeVisible()
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(
            page
                .getByRole("alert")
                .filter({hasText: "Minimum length cannot exceed maximum length."})
        ).toBeVisible()
        expect(portal.graphql.callsTo("UpdateRealmPasswordPolicy")).toHaveLength(0)
        expect(portal.graphql.callsTo("update_sequent_backend_election_event")).toHaveLength(0)
        expect(await rejections()).toEqual([])
    }
)

// Same aborted-save rejection as above, reached through a policy Keycloak did not apply.
test.fail(
    "reports a password policy Keycloak did not apply without an unhandled rejection",
    async ({page, portal}) => {
        const rejections = await captureRejections(page)
        editableEvent(portal)
        passwordPolicy(portal, CONFIGURED_POLICY, false)
        await openEvent(page, portal)
        await page.getByRole("button", {name: "Password Policy", exact: true}).click()
        await page.getByRole("checkbox", {name: "Include digits"}).check()
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(
            page.getByRole("alert").filter({hasText: "Error updating Keycloak password policy"})
        ).toBeVisible()
        expect(portal.graphql.callsTo("UpdateRealmPasswordPolicy")).toHaveLength(1)
        expect(portal.graphql.callsTo("update_sequent_backend_election_event")).toHaveLength(0)
        expect(await rejections()).toEqual([])
    }
)

test("shows a password policy load error", async ({page, portal}) => {
    editableEvent(portal)
    portal.graphql.on("GetRealmPasswordPolicy", () => ({
        errors: [{message: "Keycloak unavailable"}],
    }))
    await openEvent(page, portal)
    await page.getByRole("button", {name: "Password Policy", exact: true}).click()
    await expect(
        page.getByText("Error loading Keycloak password policy", {exact: true})
    ).toBeVisible()
})

test.describe("results website policy", () => {
    test.use({roles: [...EVENT_ROLES, "election-event-data-tab", "publish-results-write"]})

    test("configures an authenticated area-based results website", async ({page, portal}) => {
        editableEvent(portal)
        portal.graphql.on("ConfigureResultsWebsitePolicy", ({variables}) => ({
            data: {configureResultsWebsitePolicy: variables},
        }))
        await openEvent(page, portal)
        await page.getByRole("button", {name: "Advanced Configurations", exact: true}).click()
        await choose(page, /^Results Website Disabled/, "Enabled")
        await choose(page, /^Results Website Access/, "Authenticated access")
        await choose(page, /^Results Website Visibility/, "Area based")
        const update = await save(page, portal)
        const policy = portal.graphql.callsTo("ConfigureResultsWebsitePolicy")
        expect(policy.map(({variables}) => variables)).toEqual([
            {
                election_event_id: EVENT_ID,
                status: "enabled",
                access: "authenticated",
                visibility_scope: "area_based",
            },
        ])
        expect(policy[0].headers["x-hasura-role"]).toBe("publish-results-write")
        const presentation = (update._set as {presentation: Row}).presentation
        expect(JSON.parse(String(presentation.results_website))).toEqual({
            status: "enabled",
            access: "authenticated",
            visibility_scope: "area_based",
        })
    })

    // Same aborted-save rejection as the invalid password policy.
    test.fail(
        "rejects public results limited to areas without an unhandled rejection",
        async ({page, portal}) => {
            const rejections = await captureRejections(page)
            editableEvent(portal)
            await openEvent(page, portal)
            await page.getByRole("button", {name: "Advanced Configurations", exact: true}).click()
            await choose(page, /^Results Website Visibility/, "Area based")
            await page.getByRole("button", {name: "Save", exact: true}).click()
            await expect(
                page
                    .getByRole("alert")
                    .filter({hasText: "Public results must use full event visibility"})
            ).toBeVisible()
            expect(portal.graphql.callsTo("ConfigureResultsWebsitePolicy")).toHaveLength(0)
            expect(portal.graphql.callsTo("update_sequent_backend_election_event")).toHaveLength(0)
            expect(await rejections()).toEqual([])
        }
    )
})

test.describe("Keycloak realm attributes", () => {
    const attributes = {frontendUrl: "https://login.example", displayName: "Council"}

    test.describe("read only", () => {
        test.use({
            roles: [
                "admin-user",
                "election-event-read",
                "election-read",
                "election-event-data-tab",
                "keycloak-realm-attributes-read",
            ],
        })

        test("shows the realm attributes without an editor or save button", async ({
            page,
            portal,
        }) => {
            editableEvent(portal)
            portal.graphql.on("GetRealmAttributes", () => ({
                data: {get_realm_attributes: {attributes}},
            }))
            await openEvent(page, portal)
            await page.getByRole("button", {name: "Keycloak realm attributes", exact: true}).click()
            await expect(
                page.getByRole("region").filter({hasText: '"frontendUrl": "https://login.example"'})
            ).toBeVisible()
            await expect(page.getByRole("button", {name: "Save", exact: true})).toHaveCount(0)
            expect(portal.graphql.callsTo("GetRealmAttributes")[0]).toMatchObject({
                variables: {election_event_id: EVENT_ID},
                headers: {"x-hasura-role": "keycloak-realm-attributes-read"},
            })
        })

        test("reports realm attributes that fail to load", async ({page, portal}) => {
            editableEvent(portal)
            portal.graphql.on("GetRealmAttributes", () => ({
                errors: [{message: "Keycloak unavailable"}],
            }))
            await openEvent(page, portal)
            await page.getByRole("button", {name: "Keycloak realm attributes", exact: true}).click()
            await expect(
                page.getByText("Error loading Keycloak realm attributes", {exact: true})
            ).toBeVisible()
        })
    })
})

test.describe("Google Meet links", () => {
    test.use({roles: [...EVENT_ROLES, "election-event-data-tab", "google-meet-link"]})

    test("generates a meeting link for the event and copies it", async ({
        context,
        page,
        portal,
    }) => {
        await context.grantPermissions(["clipboard-read", "clipboard-write"], {
            origin: portal.origin,
        })
        editableEvent(portal)
        portal.graphql.on("GenerateGoogleMeet", () => ({
            data: {generate_google_meet: {meet_link: "https://meet.google.com/abc-defg-hij"}},
        }))
        await openEvent(page, portal)
        await page.getByRole("button", {name: "Google Meet", exact: true}).click()
        const dialog = page.getByRole("dialog", {name: "Generate Google Meet Link"})
        await expect(dialog.getByRole("textbox", {name: "Meeting Title"})).toHaveValue(
            "Council election - Meeting"
        )
        // One hour after the fixed clock, in the UTC test time zone.
        await expect(dialog.getByLabel("Start Date")).toHaveValue("2026-01-15")
        await expect(dialog.getByLabel("Start Time")).toHaveValue("13:00")
        await dialog.getByRole("textbox", {name: /^Description/}).fill("Trustee briefing")
        await dialog.getByRole("spinbutton", {name: /^Duration/}).fill("90")
        await dialog
            .getByRole("textbox", {name: "Attendee Emails"})
            .fill("ana@example.com, ben@example.com ,")
        await dialog.getByRole("button", {name: "Generate Meet Link", exact: true}).click()
        await expect(
            dialog.getByRole("heading", {name: "Google Meet Link Generated Successfully!"})
        ).toBeVisible()
        await expect(dialog.getByRole("textbox")).toHaveValue(
            "https://meet.google.com/abc-defg-hij"
        )
        const calls = portal.graphql.callsTo("GenerateGoogleMeet")
        expect(calls.map(({variables}) => variables)).toEqual([
            {
                summary: "Council election - Meeting",
                description: "Trustee briefing",
                startDateTime: "2026-01-15T13:00:00.000Z",
                endDateTime: "2026-01-15T14:30:00.000Z",
                timeZone: "UTC",
                attendeeEmails: ["ana@example.com", "ben@example.com"],
            },
        ])
        expect(calls[0].headers["x-hasura-role"]).toBe("google-meet-link")
        await dialog.getByRole("button", {name: "Copy to clipboard"}).click()
        await expect(page.getByText("Link copied to clipboard!", {exact: true})).toBeVisible()
        expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
            "https://meet.google.com/abc-defg-hij"
        )
    })

    test("explains a failed or empty meeting link", async ({page, portal}) => {
        editableEvent(portal)
        portal.graphql.once("GenerateGoogleMeet", () => ({
            errors: [{message: "Calendar API disabled"}],
        }))
        portal.graphql.once("GenerateGoogleMeet", () => ({
            data: {generate_google_meet: {meet_link: null}},
        }))
        await openEvent(page, portal)
        await page.getByRole("button", {name: "Google Meet", exact: true}).click()
        const dialog = page.getByRole("dialog", {name: "Generate Google Meet Link"})
        const generate = dialog.getByRole("button", {name: "Generate Meet Link", exact: true})
        await generate.click()
        await expect(
            dialog.getByText("Failed to generate Google Meet link: Calendar API disabled")
        ).toBeVisible()
        await generate.click()
        await expect(dialog.getByText("Link is null.", {exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("GenerateGoogleMeet")).toHaveLength(2)
    })
})

test("imports candidates from an uploaded file and tracks the task", async ({page, portal}) => {
    editableEvent(portal)
    const key = "documents/candidates-upload"
    const url = portal.s3.presign(key, "candidates-import")
    portal.s3.override(
        (request) =>
            request.method === "PUT" &&
            request.key === key &&
            request.query["X-Amz-Signature"] === "candidates-import",
        {status: 200},
        1
    )
    portal.graphql.on("GetUploadUrl", () => ({
        data: {get_upload_url: {url, document_id: DOCUMENT_ID}},
    }))
    portal.graphql.on("ImportCandidates", () => ({
        data: {
            import_candidates: {
                error_msg: null,
                document_id: DOCUMENT_ID,
                task_execution: {
                    id: TASK_ID,
                    tenant_id: TENANT_ID,
                    election_event_id: EVENT_ID,
                    name: "Import Candidates",
                    type: "IMPORT_CANDIDATES",
                    execution_status: "IN_PROGRESS",
                    created_at: FIXED_TIME,
                    start_at: FIXED_TIME,
                    end_at: null,
                    executed_by_user: "synthetic-admin",
                    annotations: {},
                    labels: {},
                    logs: [],
                },
            },
        },
    }))
    portal.graphql.on("GetTaskById", () => ({data: {sequent_backend_tasks_execution: []}}))
    await openEvent(page, portal)
    await page.getByRole("button", {name: "Import Candidates", exact: true}).click()
    const drawer = page.getByRole("dialog")
    await drawer.getByRole("textbox", {name: "Integrity Check (SHA-256)"}).fill("cd".repeat(32))
    const content = Buffer.from("Candidate,Contest\nAna Example,Council\n")
    const upload = page.waitForRequest(
        (request) => request.url() === url && request.method() === "PUT"
    )
    await drawer.locator('input[type="file"]').setInputFiles({
        name: "candidates.csv",
        mimeType: "text/csv",
        buffer: content,
    })
    expect((await upload).postDataBuffer()).toEqual(content)
    await drawer.getByRole("button", {name: "Import", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("ImportCandidates").length).toBe(1)
    expect(portal.graphql.callsTo("ImportCandidates")[0].variables).toEqual({
        documentId: DOCUMENT_ID,
        electionEventId: EVENT_ID,
        sha256: "cd".repeat(32),
    })
    expect(portal.graphql.callsTo("GetUploadUrl")[0].variables).toEqual({
        name: "candidates.csv",
        media_type: "text/csv",
        size: content.length,
        is_public: false,
    })
    await expect(page.getByText("Task: Import Candidates", {exact: true})).toBeVisible()
})
