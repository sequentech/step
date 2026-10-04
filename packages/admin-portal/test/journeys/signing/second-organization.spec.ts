// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The same screens for two organizations, set up only by configuration (the
// PR 13 fixtures): the sample preset, and a student council with its own
// roles, rules, Post term, renamed action and display name. Every expectation
// comes from the organization's file, so a hardcoded label or name fails.
import type {Page} from "@playwright/test"
import {FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID, type AdminPortal} from "../fixtures"
import {
    ER_BASE,
    EVENT_URL,
    ER_ROLES,
    SPAIN,
    SPAIN_ID,
    SigningServer,
    capacity,
    mockSignaturesTab,
    mockSigningEvent,
    openCertificate,
    openRequest,
    organization,
    organizationRules,
    overrideOf,
    rule,
    sha256,
    signInAs,
    signingDialog,
    type IOrganization,
    type Person,
} from "./data"

const ER = "generate-election-returns"
const CLIENT_NAMES = /COMELEC/i

/** The tenant as its settings configure it: display name and translation overrides. */
function mockTenant(portal: AdminPortal, org: IOrganization) {
    const tenant = {
        id: TENANT_ID,
        slug: org.name,
        annotations: {},
        settings: {
            display_name: org.tenant.display_name,
            i18n: org.tenant.i18n,
            language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
        },
        is_active: true,
        labels: {},
        voting_channels: ["ONLINE"],
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        test: true,
    }
    portal.graphql.on("GetTenantById", () => ({data: {sequent_backend_tenant: [tenant]}}))
    portal.graphql.on("sequent_backend_tenant", () => ({
        data: {
            sequent_backend_tenant: [tenant],
            sequent_backend_tenant_aggregate: {aggregate: {count: 1}},
        },
    }))
}

/** The organization's first Post and its signers for the election returns. */
function signersOf(org: IOrganization): {post: string; people: Person[]; groups: string[]} {
    const post = org.posts[0]
    const groups = org.groups.filter((group) => group.permissions.includes(`sign-${ER}`))
    const signer = org.signers.find((entry) => groups.some((group) => group.name === entry.group))
    const people = (signer?.per_post ?? []).map((title, index) => ({
        userId: `77777777-7777-4777-8777-00000000000${index + 1}`,
        username: `${org.name}-${post.key}-${index + 1}`,
        firstName: title,
        lastName: `${index + 1}`,
        title,
        certificate: "rosa-mendoza-foreign.p12",
    }))
    return {post: post.name, people, groups: groups.map((group) => group.name)}
}

for (const name of ["post-qualification", "student-council"]) {
    const org = organization(name)
    const label = overrideOf(org, `signing.actions.${ER}.label`) ?? "Generate election returns"
    const short = overrideOf(org, `signing.actions.${ER}.short`) ?? "Election returns"
    const postTerm = overrideOf(org, "signing.terms.post") ?? "Post"
    const {post, people, groups} = signersOf(org)
    const erRule = organizationRules(org).find((entry) => entry.action === ER)

    /** No client name in the copy, unless this organization's configuration says it. */
    async function expectNoClientName(page: Page) {
        const text = await page.locator("body").innerText()
        if (!CLIENT_NAMES.test(JSON.stringify(org.tenant))) expect(text).not.toMatch(CLIENT_NAMES)
    }

    test.describe(`${org.tenant.display_name} (${name})`, () => {
        test.describe("its configuration manager", () => {
            test.use({
                roles: [
                    "election-event-read",
                    "election-read",
                    "election-event-signatures-tab",
                    "signing-rules-read",
                ],
            })

            test("sees the action, its Post term, roles and number as configured", async ({
                page,
                portal,
            }) => {
                mockTenant(portal, org)
                mockSigningEvent(portal, {}, post)
                mockSignaturesTab(portal, {
                    rules: organizationRules(org).map((entry) =>
                        rule(entry.action, {
                            requirement: entry.requirement as never,
                            signatures: entry.signatures,
                        })
                    ),
                    capacities: {
                        [ER]: capacity({
                            max: people.length,
                            posts: org.posts.map((entry) => ({
                                election_id: entry.key,
                                name: entry.name,
                                count: people.length,
                            })),
                            roles: groups.map((group, index) => ({
                                id: `b2000000-0000-4000-8000-00000000000${index + 1}`,
                                name: group,
                                path: `/${group}`,
                            })),
                        }),
                    },
                    checks: [],
                    issuers: [],
                    certificates: [],
                    crls: [],
                    requests: () => [],
                })
                await page.goto(`${portal.origin}${EVENT_URL}?lang=en`)
                await page.getByRole("tab", {name: "Signatures", exact: true}).click()
                const row = page.locator(`tr[data-action="${ER}"]`)
                await expect(row.getByRole("cell").nth(0)).toHaveText(label)
                await expect(row.getByRole("cell").nth(1)).toHaveText(
                    `Each ${postTerm} and country`
                )
                await expect(row.getByRole("cell").nth(2)).toHaveText(groups.join(""))
                await expect(row.getByRole("cell").nth(3)).toHaveText(String(erRule?.signatures))
                await row.getByRole("button", {name: `View ${label}`}).click()
                await expect(
                    page.locator(".MuiDrawer-paper").getByRole("heading", {name: label})
                ).toBeVisible()
                await expectNoClientName(page)
            })
        })

        test.describe("its signer", () => {
            test.use({roles: ER_ROLES})

            test("is told to use the certificate the organization registered", async ({
                page,
                portal,
            }) => {
                mockTenant(portal, org)
                signInAs(portal, people[0])
                const server = new SigningServer(portal, people)
                const state = server.add({
                    id: "f7000000-0000-4000-8000-000000000001",
                    action: ER,
                    code: "2E9C-51D0",
                    required: erRule?.signatures ?? people.length,
                    requestedBy: people[0],
                    signers: people,
                    areaId: SPAIN_ID,
                    areaName: SPAIN,
                    postName: post,
                    subject: {
                        document_sha256: sha256(ER_BASE),
                        report_type: "ELECTORAL_RESULTS",
                        template_id: null,
                    },
                    document: {name: `${short}, ${post}.pdf`, bytes: ER_BASE},
                })
                mockSigningEvent(portal, {}, post)
                mockSignaturesTab(portal, {
                    rules: [],
                    capacities: {},
                    checks: [],
                    issuers: [],
                    certificates: [],
                    crls: [],
                    requests: () => [state.row],
                })
                const panel = await openRequest(page, portal, `${short} · ${post} · ${SPAIN}`)
                await expect(panel.getByRole("heading", {level: 2})).toHaveText(
                    `${short} · ${post} · ${SPAIN}`
                )
                await panel.getByRole("button", {name: "Sign", exact: true}).click()
                const dialog = signingDialog(page)
                await dialog.getByRole("checkbox").check()
                await dialog.getByRole("button", {name: "Continue", exact: true}).click()
                // A personal certificate from a commercial CA, not the organization's issuer.
                await openCertificate(page, people[0].certificate)
                await expect(
                    dialog.getByText(
                        `Use the certificate ${org.tenant.display_name} registered for you. Certificates from other issuers are not accepted.`
                    )
                ).toBeVisible()
                await expect(dialog.getByRole("button", {name: "Sign", exact: true})).toBeDisabled()
                await expectNoClientName(page)
            })
        })
    })
}
