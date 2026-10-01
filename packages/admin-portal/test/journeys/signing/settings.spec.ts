// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, TENANT_ID, type AdminPortal} from "../fixtures"
import {
    ANA,
    EVENT_ID,
    EVENT_URL,
    JOSE,
    MARIA,
    OFOV_GROUP,
    POST,
    SBEI_GROUP,
    SigningServer,
    TRUSTED_CHAIN_PEM,
    capacity,
    displayName,
    expectRole,
    issuersOf,
    leafPem,
    mockSignaturesTab,
    mockSigningEvent,
    rule,
    staffCertificate,
    type ISignaturesTabState,
} from "./data"

const READ = ["election-event-read", "election-read", "election-event-signatures-tab"]
const SECURITY_OFFICER_ID = "c1000000-0000-4000-8000-000000000002"
const REQUEST_ID = "f4000000-0000-4000-8000-000000000001"

function tabState(overrides: Partial<ISignaturesTabState> = {}): ISignaturesTabState {
    return {
        rules: [rule("close-voting", {signatures: 1})],
        capacities: {"close-voting": capacity({waiting: 1})},
        checks: [
            {
                revocation_check: "check" as never,
                crl_unavailable: "refuse" as never,
                registration: "on-first-use" as never,
                post_binding: "one-post" as never,
                revision: 1,
                updated_at: "2026-01-12T08:00:00.000Z",
                updated_by_name: "Sofia Lim",
            },
        ],
        issuers: [],
        certificates: [staffCertificate(MARIA), staffCertificate(JOSE)],
        crls: [
            {
                id: "e2000000-0000-4000-8000-000000000001",
                issuer_fingerprint: "aa".repeat(32),
                url: "http://crl.test-pnpki.invalid/pnpki-individual-ca.crl",
                fetched_at: "2026-01-15T11:00:00.000Z",
                status: "ok" as never,
            },
        ],
        requests: () => [],
        ...overrides,
    }
}

async function openSignatures(page: Page, portal: AdminPortal) {
    await page.goto(`${portal.origin}${EVENT_URL}?lang=en`)
    await page.getByRole("tab", {name: "Signatures", exact: true}).click()
}

const subTabs = (page: Page) =>
    page.getByRole("tab", {name: /^(Protected actions|Certificates|Requests)$/})

const mutations = (portal: AdminPortal) =>
    portal.graphql.calls.filter((call) => call.query.trim().startsWith("mutation"))

test.describe("a configuration manager", () => {
    test.use({
        roles: [...READ, "signing-rules-read", "signing-rules-write", "role-read", "role-write"],
    })

    test("asks for 2 of 3 signatures to close voting and adds a role who can sign", async ({
        page,
        portal,
    }) => {
        mockSigningEvent(portal)
        const state = tabState()
        mockSignaturesTab(portal, state)
        portal.graphql.on("getRoles", () => ({
            data: {
                get_roles: {
                    items: [SBEI_GROUP, OFOV_GROUP].map(({id, name}) => ({
                        id,
                        name,
                        permissions: [],
                        access: {view: true, manage: true},
                        attributes: {},
                        client_roles: {},
                    })),
                    total: {aggregate: {count: 2}},
                },
            },
        }))
        portal.graphql.on("SigningPutRule", ({variables}) => {
            state.rules = [
                rule("close-voting", {
                    signatures: Number(variables.signatures),
                    revision: 5,
                    updated_at: "2026-01-15T12:00:00.000Z",
                    updated_by_name: "Carlos Mendez",
                }),
            ]
            state.capacities["close-voting"] = capacity({roles: [SBEI_GROUP, OFOV_GROUP]})
            return {
                data: {
                    signingPutRule: {
                        revision: 5,
                        cancelled: ["f5000000-0000-4000-8000-000000000001"],
                        rule: state.rules[0],
                        short_posts: [],
                        warnings: [],
                    },
                },
            }
        })
        await openSignatures(page, portal)
        await expect(subTabs(page)).toHaveText(["Protected actions"])
        const row = page.locator('tr[data-action="close-voting"]')
        await expect(row.getByRole("cell")).toHaveText([
            "Close voting",
            "Each Post",
            SBEI_GROUP.name,
            "1",
            "1 hour",
            "1",
            "",
        ])
        await expect(
            page.getByText(/^Signing rules are part of this event's configuration version 2\./)
        ).toContainText("by Carlos Mendez")

        await row.getByRole("button", {name: "Edit Close voting"}).click()
        const drawer = page.locator(".MuiDrawer-paper")
        await expect(drawer.getByRole("heading", {name: "Close voting"})).toBeVisible()
        await expect(
            drawer.getByText(
                "1 request is waiting for signatures under the current rule. Saving cancels it; the person who started it starts again."
            )
        ).toBeVisible()
        const save = drawer.getByRole("button", {name: "Save", exact: true})
        await expect(save).toBeDisabled()
        const signatures = drawer.getByRole("spinbutton", {name: "Signatures needed"})
        await signatures.fill("4")
        await expect(
            drawer.getByText("No Post has 4 people who can sign. The most is 3.")
        ).toBeVisible()
        await expect(save).toBeDisabled()
        await signatures.fill("2")
        await expect(
            drawer.getByText(
                "Each signer uses their digital certificate. Every Post has at least 3 people who can sign."
            )
        ).toBeVisible()
        await drawer.getByRole("combobox", {name: "Who can sign"}).click()
        await page.getByRole("option", {name: OFOV_GROUP.name, exact: true}).click()
        await expect(
            drawer.getByText("The number is checked against the new roles when you save.")
        ).toBeVisible()
        await save.click()
        await expect(page.getByText("The signing rule was saved.", {exact: true})).toBeVisible()

        expect(portal.graphql.callsTo("SigningPutRule").map(({variables}) => variables)).toEqual([
            {
                election_event_id: EVENT_ID,
                action: "close-voting",
                requirement: "required",
                signatures: 2,
                requester_signing: "allowed",
                expires_minutes: 60,
                roles: {add: [OFOV_GROUP.id], remove: []},
                expected_revision: 4,
            },
        ])
        await expect(row.getByRole("cell").nth(3)).toHaveText("2")
        await expect(row.getByRole("cell").nth(2)).toHaveText(
            `${SBEI_GROUP.name}${OFOV_GROUP.name}`
        )
        expectRole(portal, "SigningPutRule", "signing-rules-write")
        expectRole(portal, "GetSigningRules", "signing-rules-read")
        expectRole(portal, "GetSigningRuleCapacities", "signing-rules-read")
        expect(portal.graphql.callsTo("GetSigningCertificates")).toHaveLength(0)
        expect(portal.graphql.callsTo("GetSigningRequests")).toHaveLength(0)
    })
})

test.describe("a security officer", () => {
    test.use({
        roles: [
            ...READ,
            "signing-certificates-read",
            "signing-issuers-write",
            "signing-checks-write",
            "signing-certificates-register",
            "signing-certificates-revoke",
        ],
    })

    test("imports the issuers, edits the checks, registers and revokes certificates", async ({
        page,
        portal,
    }) => {
        mockSigningEvent(portal)
        const state = tabState()
        mockSignaturesTab(portal, state)
        portal.graphql.on("getUsers", ({variables}) => {
            const items = [MARIA, JOSE, ANA]
                .filter((person) => person.username.includes(String(variables.username ?? "")))
                .map((person) => ({
                    id: person.userId,
                    username: person.username,
                    email: `${person.username}@example.org`,
                    email_verified: true,
                    enabled: true,
                    first_name: person.firstName,
                    last_name: person.lastName,
                    attributes: {},
                    groups: [],
                    area: null,
                    votes_info: [],
                }))
            return {data: {get_users: {items, total: {aggregate: {count: items.length}}}}}
        })
        portal.graphql.on("SigningImportIssuers", () => {
            state.issuers = issuersOf(TRUSTED_CHAIN_PEM)
            return {data: {signingImportIssuers: {imported: 2, skipped: 0, errors: []}}}
        })
        portal.graphql.on("SigningPutChecks", ({variables}) => {
            state.checks = [
                {...state.checks[0], post_binding: variables.post_binding as never, revision: 2},
            ]
            return {data: {signingPutChecks: {revision: 2}}}
        })
        const anaCertificate = staffCertificate(ANA, {
            election_id: null,
            registration: "security-officer" as never,
            registered_by: SECURITY_OFFICER_ID,
            registered_by_name: "Sofia Lim",
        })
        portal.graphql.on("SigningRegisterCertificate", () => {
            state.certificates = [anaCertificate, ...state.certificates]
            return {data: {signingRegisterCertificate: {certificate_id: anaCertificate.id}}}
        })
        const maria = state.certificates[0]
        portal.graphql.on("SigningRevokeCertificate", ({variables}) => {
            state.certificates = state.certificates.map((certificate) =>
                certificate.id === variables.certificate_id
                    ? {
                          ...certificate,
                          status: "revoked" as never,
                          revoked_at: "2026-01-15T12:00:00.000Z",
                          revoke_reason: String(variables.reason),
                      }
                    : certificate
            )
            return {data: {signingRevokeCertificate: {certificate_id: variables.certificate_id}}}
        })
        await openSignatures(page, portal)
        await expect(subTabs(page)).toHaveText(["Certificates"])

        // Trusted issuers: the test PKI's root and intermediate.
        const issuers = page.getByRole("table", {name: "Trusted issuers"})
        await expect(issuers).toContainText("No trusted issuers yet.")
        await page.getByRole("button", {name: "Import issuer certificates"}).click()
        const importDialog = page.getByRole("dialog", {name: "Import issuer certificates"})
        await importDialog.getByLabel("Choose a certificate file").setInputFiles({
            name: "test-pnpki-chain.pem",
            mimeType: "application/x-pem-file",
            buffer: Buffer.from(TRUSTED_CHAIN_PEM),
        })
        await expect(importDialog.getByText("test-pnpki-chain.pem")).toBeVisible()
        await importDialog.getByRole("button", {name: "Import", exact: true}).click()
        await expect(
            page.getByText("2 issuer certificates imported; 0 were already trusted.")
        ).toBeVisible()
        await expect(issuers.getByRole("row").nth(1)).toContainText(
            "Test PNPKI Individual CAIntermediateTest PNPKI Root CA"
        )
        await expect(issuers.getByRole("row").nth(2)).toContainText(
            "Test PNPKI Root CARootTest PNPKI Root CA"
        )
        expect(portal.graphql.callsTo("SigningImportIssuers")[0].variables).toEqual({
            election_event_id: EVENT_ID,
            pem: TRUSTED_CHAIN_PEM,
            der_base64: null,
        })

        // Checks: certificates may sign for any Post.
        const onePost = page.getByRole("switch", {name: "A certificate signs for one Post only"})
        await expect(onePost).toBeChecked()
        await onePost.click()
        await expect(page.getByText("The certificate checks were saved.")).toBeVisible()
        await expect(onePost).not.toBeChecked()
        expect(portal.graphql.callsTo("SigningPutChecks")[0].variables).toEqual({
            election_event_id: EVENT_ID,
            revocation_check: "check",
            crl_unavailable: "refuse",
            registration: "on-first-use",
            post_binding: "any-post",
            expected_revision: 1,
        })

        // Register Ana's certificate for every Post.
        await page.getByRole("button", {name: "Register a certificate"}).click()
        const registerDialog = page.getByRole("dialog", {name: "Register a certificate"})
        await registerDialog.getByRole("combobox", {name: "Person"}).fill("madrid-3")
        await page.getByRole("option", {name: `${displayName(ANA)} (${ANA.username})`}).click()
        const pem = leafPem(ANA.certificate)
        await registerDialog.getByLabel("Choose a certificate file").setInputFiles({
            name: "ana-cruz.pem",
            mimeType: "application/x-pem-file",
            buffer: Buffer.from(pem),
        })
        await expect(registerDialog.getByRole("textbox", {name: "Certificate (PEM)"})).toHaveValue(
            pem
        )
        await registerDialog.getByRole("button", {name: "Register", exact: true}).click()
        await expect(page.getByText("The certificate was registered.")).toBeVisible()
        expect(portal.graphql.callsTo("SigningRegisterCertificate")[0].variables).toEqual({
            election_event_id: EVENT_ID,
            user_id: ANA.userId,
            election_id: null,
            pem,
            linked_to: null,
        })
        const registered = page.getByRole("table", {name: "Registered certificates"})
        await expect(registered.getByRole("row").nth(1)).toContainText(
            `${displayName(ANA)}${ANA.username}All`
        )
        await expect(registered.getByRole("row").nth(1)).toContainText("By Sofia Lim")

        // Revoke Maria's: her token was lost.
        await registered
            .getByRole("button", {name: `Revoke the certificate of ${displayName(MARIA)}`})
            .click()
        const revokeDialog = page.getByRole("dialog", {
            name: `Revoke the certificate of ${displayName(MARIA)}`,
        })
        await revokeDialog.getByRole("textbox", {name: "Reason"}).fill("Token lost")
        await revokeDialog.getByRole("button", {name: "Revoke", exact: true}).click()
        await expect(page.getByText("The certificate was revoked.")).toBeVisible()
        expect(portal.graphql.callsTo("SigningRevokeCertificate")[0].variables).toEqual({
            election_event_id: EVENT_ID,
            certificate_id: maria.id,
            reason: "Token lost",
        })
        const mariaRow = registered.getByRole("row").filter({hasText: MARIA.username})
        await expect(mariaRow).toContainText("Revoked 1/15/2026")
        await expect(
            mariaRow.getByRole("button", {name: `Revoke the certificate of ${displayName(MARIA)}`})
        ).toHaveCount(0)
        await expect(registered.getByRole("row").filter({hasText: POST})).toHaveCount(2)

        expectRole(portal, "GetSigningCertificates", "signing-certificates-read")
        expectRole(portal, "SigningImportIssuers", "signing-issuers-write")
        expectRole(portal, "SigningPutChecks", "signing-checks-write")
        expectRole(portal, "SigningRegisterCertificate", "signing-certificates-register")
        expectRole(portal, "SigningRevokeCertificate", "signing-certificates-revoke")
        expect(portal.graphql.callsTo("GetSigningRules")).toHaveLength(0)
    })
})

test.describe("an auditor", () => {
    test.use({
        roles: [
            ...READ,
            "signing-rules-read",
            "signing-certificates-read",
            "signing-requests-read",
        ],
    })

    test("reads all three sub-tabs and changes nothing", async ({page, portal}) => {
        mockSigningEvent(portal)
        const server = new SigningServer(portal)
        const request = server.add({
            id: REQUEST_ID,
            action: "close-voting",
            code: "5C1E-77AA",
            required: 2,
            requestedBy: MARIA,
            signers: [MARIA, JOSE],
            subject: {channels: ["ONLINE"], from: ["ONLINE=OPEN"]},
            signed: [MARIA],
        })
        mockSignaturesTab(
            portal,
            tabState({issuers: issuersOf(TRUSTED_CHAIN_PEM), requests: () => [request.row]})
        )
        await openSignatures(page, portal)
        await expect(subTabs(page)).toHaveText(["Protected actions", "Certificates", "Requests"])

        await expect(page.getByText("Read only", {exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: /^Edit /})).toHaveCount(0)
        await page.getByRole("button", {name: "View Close voting"}).click()
        const drawer = page.locator(".MuiDrawer-paper")
        await expect(
            drawer.getByText(
                "Read only. Changing signing rules needs the permission “Signatures: edit protected actions”."
            )
        ).toBeVisible()
        await expect(drawer.getByRole("button", {name: "Save"})).toHaveCount(0)
        await expect(drawer.getByRole("switch", {name: "Needs signatures"})).toBeDisabled()
        await drawer.getByText("Close", {exact: true}).click()

        await page.getByRole("tab", {name: "Certificates", exact: true}).click()
        await expect(page.getByText("Read only", {exact: true})).toHaveCount(3)
        await expect(page.getByRole("table", {name: "Trusted issuers"})).toContainText(
            "Test PNPKI Root CA"
        )
        for (const name of ["Import issuer certificates", "Register a certificate"])
            await expect(page.getByRole("button", {name})).toHaveCount(0)
        await expect(page.getByRole("button", {name: /^Revoke the certificate of /})).toHaveCount(0)
        await expect(page.getByRole("switch", {name: "Check revocation lists"})).toBeDisabled()

        await page.getByRole("tab", {name: "Requests", exact: true}).click()
        const requests = page.getByRole("table", {name: "Requests"})
        await expect(requests.getByRole("row").nth(1)).toContainText(`Closing · ${POST}`)
        await expect(requests.getByRole("row").nth(1)).toContainText("Waiting · 1 of 2")
        await expect(page.getByRole("button", {name: "Export CSV"})).toHaveCount(0)

        expect(mutations(portal)).toEqual([])
        expectRole(portal, "GetSigningRules", "signing-rules-read")
        expectRole(portal, "GetSigningRuleCapacities", "signing-rules-read")
        expectRole(portal, "GetSigningCertificates", "signing-certificates-read")
        expectRole(portal, "GetSigningRequests", "signing-requests-read")
    })
})

test.describe("an OFOV officer", () => {
    test.use({
        roles: [
            ...READ,
            "signing-requests-read",
            "signing-requests-cancel",
            "signing-requests-export",
        ],
    })

    test("exports the requests and cancels a stuck one", async ({page, portal}) => {
        mockSigningEvent(portal)
        const server = new SigningServer(portal)
        const request = server.add({
            id: REQUEST_ID,
            action: "close-voting",
            code: "5C1E-77AA",
            required: 2,
            requestedBy: MARIA,
            signers: [MARIA, JOSE],
            subject: {channels: ["ONLINE"], from: ["ONLINE=OPEN"]},
            signed: [MARIA],
        })
        mockSignaturesTab(portal, tabState({requests: () => [request.row]}))
        const csvKey = `${TENANT_ID}/${EVENT_ID}/signing-requests.csv`
        portal.s3.putBytes(
            "private",
            csvKey,
            new TextEncoder().encode("request,code\n"),
            "text/csv"
        )
        const csvUrl = portal.s3.presign(csvKey, "export")
        portal.graphql.on("SigningExportRequests", () => ({
            data: {
                signingExportRequests: {
                    document_id: "f6000000-0000-4000-8000-000000000001",
                    sha256: "ee".repeat(32),
                    rows: 1,
                    url: csvUrl,
                },
            },
        }))
        await openSignatures(page, portal)
        await expect(subTabs(page)).toHaveText(["Requests"])

        const download = page.waitForEvent("download")
        await page.getByRole("button", {name: "Export CSV"}).click()
        expect((await download).url()).toBe(csvUrl)
        expect(portal.graphql.callsTo("SigningExportRequests")[0].variables).toEqual({
            election_event_id: EVENT_ID,
        })

        await page.getByRole("button", {name: `Closing · ${POST}`, exact: true}).click()
        const panel = page
            .locator(".MuiDrawer-paper")
            .filter({has: page.getByTestId("signing-status")})
        await expect(panel.getByTestId("signing-status")).toHaveText("Waiting · 1 of 2")
        // OFOV can't sign it, only cancel it.
        await expect(panel.getByRole("button", {name: "Sign", exact: true})).toHaveCount(0)
        await panel.getByRole("button", {name: "Cancel request", exact: true}).click()
        const cancel = page.getByRole("dialog", {name: "Cancel this request?"})
        await cancel.getByRole("textbox", {name: "Reason (optional)"}).fill("Stuck since yesterday")
        await cancel.getByRole("button", {name: "Cancel request", exact: true}).click()
        await expect(panel.getByTestId("signing-status")).toHaveText("Cancelled")
        expect(portal.graphql.callsTo("SigningCancel")[0].variables).toEqual({
            request_id: REQUEST_ID,
            reason: "Stuck since yesterday",
        })
        expectRole(portal, "SigningExportRequests", "signing-requests-export")
        expectRole(portal, "SigningCancel", "signing-requests-cancel")
        expectRole(portal, "SigningGetRequest", "signing-requests-read")
        expectRole(portal, "GetSigningRequests", "signing-requests-read")
    })
})

test.describe("without the Signatures tab permission", () => {
    test.use({
        roles: [
            "election-event-read",
            "election-read",
            "signing-rules-read",
            "signing-certificates-read",
            "signing-requests-read",
        ],
    })

    test("the event has no Signatures tab and reads no signing table", async ({page, portal}) => {
        mockSigningEvent(portal)
        mockSignaturesTab(portal, tabState())
        await page.goto(`${portal.origin}${EVENT_URL}?lang=en`)
        await expect(page.getByText("Council election", {exact: true}).first()).toBeVisible()
        await expect(page.getByRole("tab", {name: "Signatures", exact: true})).toHaveCount(0)
        for (const operation of [
            "GetSigningRules",
            "GetSigningRuleCapacities",
            "GetSigningCertificates",
            "GetSigningRequests",
        ])
            expect(portal.graphql.callsTo(operation)).toHaveLength(0)
    })
})
