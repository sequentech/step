// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Locator, Page} from "@playwright/test"
import {test, expect, type AdminPortal} from "../fixtures"
import {electionEvent, EVENT_ID, EVENT_ROLES, listOf, mockEvent, openEvent, TENANT_ID} from "./data"
import {
    ballotStyle,
    blocklistEntry,
    election,
    emulatorLog,
    IVR_ANNOTATIONS,
    IVR_PHONE,
    IVR_PROMPTS,
    ivrEvent,
    NORTH_AREA,
    serveEmulator,
    type EmulatorScript,
    type PromptInfo,
} from "./ivr-data"

const IVR_ROLES = [...EVENT_ROLES, "election-event-ivr-tab"]
const BLOCKLIST_ROLES = [
    "phone-blacklist-read",
    "phone-blacklist-create",
    "phone-blacklist-update",
    "phone-blacklist-delete",
]
test.use({roles: [...IVR_ROLES, ...BLOCKLIST_ROLES]})

type Annotations = Record<string, string>

/** Serves an IVR event whose annotations follow each saved update. */
function mockIvrEvent(portal: AdminPortal, initial: Annotations = IVR_ANNOTATIONS) {
    let annotations = initial
    mockEvent(portal, () => ivrEvent(annotations))
    portal.graphql.on("update_sequent_backend_election_event", ({variables}) => {
        annotations = (variables._set as {annotations: Annotations}).annotations
        return {
            data: {
                update_sequent_backend_election_event: {
                    affected_rows: 1,
                    returning: [ivrEvent(annotations)],
                },
            },
        }
    })
}

function savedAnnotations(portal: AdminPortal) {
    return portal.graphql
        .callsTo("update_sequent_backend_election_event")
        .map((call) => call.variables)
}

// Row icon buttons have no accessible names: edit comes first, delete second.
const edit = (row: Locator) => row.getByRole("button").first()
const remove = (row: Locator) => row.getByRole("button").nth(1)

async function openIvr(page: Page, portal: AdminPortal, subTab?: string) {
    await openEvent(page, portal, "IVR")
    if (subTab) await page.getByRole("tab", {name: subTab, exact: true}).click()
}

test.describe("tab visibility", () => {
    test.use({roles: [...IVR_ROLES, "election-event-cas-tab", "phone-blacklist-read"]})
    // Certificates becomes the first tab when IVR is hidden, so its list loads.
    test.beforeEach(({portal}) => {
        portal.graphql.on(
            "sequent_backend_certificate_authority",
            listOf("sequent_backend_certificate_authority", [])
        )
    })

    test("the IVR tab needs the telephone voting channel", async ({page, portal}) => {
        mockEvent(portal, electionEvent({}, {voter_certificate_policy: "enabled"}))
        await openEvent(page, portal)
        await expect(page.getByRole("tab", {name: "Certificates", exact: true})).toBeVisible()
        await expect(page.getByRole("tab", {name: "IVR", exact: true})).toHaveCount(0)
    })

    test.describe("without the IVR tab role", () => {
        test.use({roles: [...EVENT_ROLES, "election-event-cas-tab"]})

        test("the IVR tab stays hidden", async ({page, portal}) => {
            mockEvent(
                portal,
                electionEvent(
                    {voting_channels: {online: true, telephone: true}},
                    {voter_certificate_policy: "enabled"}
                )
            )
            await openEvent(page, portal)
            await expect(page.getByRole("tab", {name: "Certificates", exact: true})).toBeVisible()
            await expect(page.getByRole("tab", {name: "IVR", exact: true})).toHaveCount(0)
        })
    })

    test.describe("without phone-blacklist-read", () => {
        test.use({roles: IVR_ROLES})

        test("the blocklist sub-tab stays hidden", async ({page, portal}) => {
            mockIvrEvent(portal)
            await openIvr(page, portal)
            await expect(page.getByRole("tab")).toHaveText([
                "IVR",
                "Configuration",
                "Prompts",
                "Emulator",
            ])
        })
    })
})

test.describe("configuration", () => {
    test("edits the phone number and flow and saves them into the event annotations", async ({
        page,
        portal,
    }) => {
        mockIvrEvent(portal)
        await openIvr(page, portal)
        const phone = page.getByLabel("Configured phone number")
        const save = page.getByRole("button", {name: "Save", exact: true})
        const cancel = page.getByRole("button", {name: "Cancel", exact: true})
        await expect(phone).toHaveValue(IVR_PHONE)
        await expect(page.getByText('"Joanna"', {exact: true})).toBeVisible()
        await expect(save).toBeDisabled()
        await expect(cancel).toBeDisabled()

        await phone.fill("+15550009999")
        await cancel.click()
        await expect(phone).toHaveValue(IVR_PHONE)
        await expect(save).toBeDisabled()

        await phone.fill("+15550002222")
        await page.getByText('"Joanna"', {exact: true}).dblclick()
        await page.keyboard.press("ControlOrMeta+a")
        await page.keyboard.type("Matthew")
        await page.keyboard.press("Enter")
        await expect(page.getByText('"Matthew"', {exact: true})).toBeVisible()
        await save.click()

        await expect(page.getByText("Saved successfully", {exact: true})).toBeVisible()
        expect(savedAnnotations(portal)).toEqual([
            {
                _set: {
                    annotations: {
                        "ivr:config":
                            '{"flow":[{"phase":"welcome","name":"greeting","prompt_key":"welcome"},{"phase":"menu","name":"main","prompt_key":"menu"}],"voice":"Matthew"}',
                        "ivr:phone-number": "+15550002222",
                        "ivr:prompts": IVR_PROMPTS,
                    },
                },
                where: {id: {_eq: EVENT_ID}},
            },
        ])
        await expect(phone).toHaveValue("+15550002222")
        await expect(save).toBeDisabled()
    })

    test("a new flow step starts with an empty phase and name", async ({page, portal}) => {
        mockIvrEvent(portal)
        await openIvr(page, portal)
        // The JSON editor's icon buttons have no accessible names; the add icon is the last one.
        const flowHeader = page
            .locator(".jer-collection-header-row")
            .filter({has: page.getByText("flow:", {exact: true})})
        await flowHeader.hover()
        await flowHeader.locator(".jer-edit-buttons > div").last().click()
        await expect(flowHeader).toContainText("3 items")
        await page.getByRole("button", {name: "Save", exact: true}).click()

        await expect(page.getByText("Saved successfully", {exact: true})).toBeVisible()
        expect(savedAnnotations(portal)).toEqual([
            {
                _set: {
                    annotations: {
                        "ivr:config":
                            '{"flow":[{"phase":"welcome","name":"greeting","prompt_key":"welcome"},{"phase":"menu","name":"main","prompt_key":"menu"},{"phase":"","name":""}],"voice":"Joanna"}',
                        "ivr:phone-number": IVR_PHONE,
                        "ivr:prompts": IVR_PROMPTS,
                    },
                },
                where: {id: {_eq: EVENT_ID}},
            },
        ])
    })

    test("a rejected save keeps the edits and reports the failure", async ({page, portal}) => {
        mockEvent(portal, ivrEvent())
        portal.graphql.on("update_sequent_backend_election_event", () => ({
            errors: [{message: "permission denied"}],
        }))
        await openIvr(page, portal)
        const phone = page.getByLabel("Configured phone number")
        await phone.fill("+15550002222")
        await page.getByRole("button", {name: "Save", exact: true}).click()

        await expect(page.getByText("Failed to save", {exact: true})).toBeVisible()
        expect(savedAnnotations(portal)).toHaveLength(1)
        await expect(phone).toHaveValue("+15550002222")
        await expect(page.getByRole("button", {name: "Save", exact: true})).toBeEnabled()
    })
})

test.describe("prompts", () => {
    const rows = (page: Page) => page.getByRole("row")
    const row = (page: Page, key: string) =>
        page.getByRole("row").filter({has: page.getByRole("cell", {name: key, exact: true})})

    test("lists required prompts first and adds a prompt to every language", async ({
        page,
        portal,
    }) => {
        mockIvrEvent(portal)
        await openIvr(page, portal, "Prompts")
        // "menu" is required by the flow but has no text yet, so it is listed empty.
        await expect(rows(page)).toHaveText(
            [
                "Key Value Actions",
                "menu",
                "welcome Welcome to the council election",
                "closing Goodbye",
                "extra Please hold",
            ],
            {useInnerText: true}
        )
        await expect(row(page, "welcome").getByRole("button")).toHaveCount(1)
        await expect(row(page, "extra").getByRole("button")).toHaveCount(2)

        await page.getByRole("button", {name: "Add", exact: true}).click()
        const key = page.getByLabel("Key", {exact: true})
        const value = page.getByLabel("Value", {exact: true})
        const saveDrawer = page.getByRole("button", {name: "Save", exact: true})
        await expect(key).toHaveValue("new_prompt_key")
        await expect(saveDrawer).toBeDisabled()
        await key.fill("closing")
        await value.fill("Thank you for voting")
        await expect(saveDrawer).toBeDisabled()
        await key.fill("thanks")
        await saveDrawer.click()
        await expect(row(page, "thanks")).toHaveText("thanks Thank you for voting", {
            useInnerText: true,
        })

        await page.getByLabel("Select Language").click()
        await page.getByRole("option", {name: "Spanish", exact: true}).click()
        await expect(rows(page)).toHaveText(
            [
                "Key Value Actions",
                "menu",
                "welcome Bienvenido a la elección del consejo",
                "extra Espere, por favor",
                "thanks Thank you for voting",
            ],
            {useInnerText: true}
        )

        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page.getByText("Saved successfully", {exact: true})).toBeVisible()
        expect(savedAnnotations(portal)).toEqual([
            {
                _set: {
                    annotations: {
                        ...IVR_ANNOTATIONS,
                        "ivr:prompts":
                            '{"en":{"welcome":"Welcome to the council election","extra":"Please hold","closing":"Goodbye","menu":"","thanks":"Thank you for voting"},"es":{"welcome":"Bienvenido a la elección del consejo","extra":"Espere, por favor","menu":"","thanks":"Thank you for voting"}}',
                    },
                },
                where: {id: {_eq: EVENT_ID}},
            },
        ])
        await expect(page.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
    })

    test("edits a required prompt and deletes an optional one", async ({page, portal}) => {
        mockIvrEvent(portal)
        await openIvr(page, portal, "Prompts")
        await edit(row(page, "menu")).click()
        await expect(page.getByLabel("Key", {exact: true})).toHaveValue("menu")
        const saveDrawer = page.getByRole("button", {name: "Save", exact: true})
        await expect(saveDrawer).toBeDisabled()
        await page.getByLabel("Value", {exact: true}).fill("Press 1 to vote")
        await saveDrawer.click()
        await expect(row(page, "menu")).toHaveText("menu Press 1 to vote", {useInnerText: true})

        await remove(row(page, "extra")).click()
        const dialog = page.getByRole("dialog")
        await expect(dialog).toContainText("Are you sure you want to delete this item?")
        await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
        await expect(row(page, "extra")).toBeVisible()
        await remove(row(page, "extra")).click()
        await page.getByRole("dialog").getByRole("button", {name: "Delete", exact: true}).click()
        await expect(row(page, "extra")).toHaveCount(0)

        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page.getByText("Saved successfully", {exact: true})).toBeVisible()
        expect(savedAnnotations(portal)).toEqual([
            {
                _set: {
                    annotations: {
                        ...IVR_ANNOTATIONS,
                        "ivr:prompts":
                            '{"en":{"welcome":"Welcome to the council election","closing":"Goodbye","menu":"Press 1 to vote"},"es":{"welcome":"Bienvenido a la elección del consejo","menu":""}}',
                    },
                },
                where: {id: {_eq: EVENT_ID}},
            },
        ])
    })

    test("a rejected save reports the failure and cancel restores the stored prompts", async ({
        page,
        portal,
    }) => {
        mockEvent(portal, ivrEvent())
        portal.graphql.on("update_sequent_backend_election_event", () => ({
            errors: [{message: "permission denied"}],
        }))
        await openIvr(page, portal, "Prompts")
        await remove(row(page, "closing")).click()
        await page.getByRole("dialog").getByRole("button", {name: "Delete", exact: true}).click()
        await expect(row(page, "closing")).toHaveCount(0)
        await page.getByRole("button", {name: "Save", exact: true}).click()

        await expect(page.getByText("Failed to save", {exact: true})).toBeVisible()
        expect(savedAnnotations(portal)).toHaveLength(1)
        await page.getByRole("button", {name: "Cancel", exact: true}).click()
        await expect(row(page, "closing")).toHaveText("closing Goodbye", {useInnerText: true})
        await expect(page.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
    })

    test("pages through long prompt lists", async ({page, portal}) => {
        const prompts = Object.fromEntries(
            Array.from({length: 12}, (_, index) => [
                `prompt_${String(index + 1).padStart(2, "0")}`,
                `Prompt ${index + 1}`,
            ])
        )
        mockIvrEvent(portal, {"ivr:prompts": JSON.stringify({en: prompts})})
        await openIvr(page, portal, "Prompts")
        await expect(page.getByText("1–10 of 12", {exact: true})).toBeVisible()
        await expect(rows(page)).toHaveCount(11)
        await page.getByRole("button", {name: "Go to next page"}).click()
        await expect(rows(page)).toHaveText(
            ["Key Value Actions", "prompt_11 Prompt 11", "prompt_12 Prompt 12"],
            {useInnerText: true}
        )
        await page.getByRole("combobox", {name: /Rows per page/}).click()
        await page.getByRole("option", {name: "25", exact: true}).click()
        await expect(page.getByText("1–12 of 12", {exact: true})).toBeVisible()
        await expect(rows(page)).toHaveCount(13)
    })

    test("unreadable stored prompts open as an empty list", async ({page, portal}) => {
        mockIvrEvent(portal, {
            "ivr:config": '{"flow":{"phase":"welcome"}}',
            "ivr:prompts": "{not json",
        })
        await openIvr(page, portal, "Prompts")
        await expect(page.getByText("No prompts created yet", {exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
    })

    test("an event without prompts offers to add the first one", async ({page, portal}) => {
        mockIvrEvent(portal, {})
        await openIvr(page, portal, "Prompts")
        await expect(page.getByText("No prompts created yet", {exact: true})).toBeVisible()
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const saveDrawer = page.getByRole("button", {name: "Save", exact: true})
        await page.getByLabel("Value", {exact: true}).fill("Hello")
        await page.getByLabel("Key", {exact: true}).fill(" ")
        await expect(saveDrawer).toBeDisabled()
        await page.getByLabel("Key", {exact: true}).fill("greeting")
        await saveDrawer.click()
        await expect(row(page, "greeting")).toHaveText("greeting Hello", {useInnerText: true})
        await expect(page.getByLabel("Select Language")).toBeVisible()

        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page.getByText("Saved successfully", {exact: true})).toBeVisible()
        expect(savedAnnotations(portal)).toEqual([
            {
                _set: {
                    annotations: {
                        "ivr:prompts": '{"en":{"greeting":"Hello"},"es":{"greeting":"Hello"}}',
                    },
                },
                where: {id: {_eq: EVENT_ID}},
            },
        ])
    })
})

test.describe("blocklist", () => {
    const FIRST_ID = "b0000000-0000-4000-8000-000000000001"
    const NEW_ID = "b0000000-0000-4000-8000-000000000002"
    const row = (page: Page, phone: string) =>
        page.getByRole("row").filter({has: page.getByRole("cell", {name: phone, exact: true})})

    /** A blocklist that follows the create, update and delete mutations. */
    function mockBlocklist(
        portal: AdminPortal,
        initial = [blocklistEntry(FIRST_ID, "+15550001234", "Abusive caller")]
    ) {
        let entries = initial
        mockIvrEvent(portal)
        portal.graphql.on(
            "sequent_backend_phone_blacklist",
            listOf("sequent_backend_phone_blacklist", () => entries)
        )
        portal.graphql.on("CreatePhoneBlacklistEntry", ({variables}) => {
            entries = [
                ...entries,
                blocklistEntry(
                    NEW_ID,
                    String(variables.phone_e164),
                    (variables.reason as string | null) ?? null
                ),
            ]
            return {data: {create_phone_blacklist_entry: {id: NEW_ID}}}
        })
        portal.graphql.on("UpdatePhoneBlacklistEntry", ({variables}) => {
            entries = entries.map((entry) =>
                entry.id === variables.id ? {...entry, reason: variables.reason} : entry
            )
            return {data: {update_sequent_backend_phone_blacklist_by_pk: {id: variables.id}}}
        })
        portal.graphql.on("DeletePhoneBlacklistEntry", ({variables}) => {
            entries = entries.filter((entry) => entry.id !== variables.id)
            return {data: {delete_phone_blacklist_entry: {id: variables.id}}}
        })
    }

    function wire(portal: AdminPortal, operation: string) {
        return portal.graphql
            .callsTo(operation)
            .map(({variables, headers}) => ({variables, role: headers["x-hasura-role"]}))
    }

    test("lists blocked numbers and blocks a new one", async ({page, portal}) => {
        mockBlocklist(portal)
        await openIvr(page, portal, "Blocklist")
        await expect(page.getByRole("row")).toHaveText(
            [
                "Phone number Reason Created by Created at Actions",
                "+15550001234 Abusive caller 80000000-0000-4000-8000-000000000001 2026-01-10T09:30:00Z",
            ],
            {useInnerText: true}
        )

        await page.getByRole("button", {name: "Add", exact: true}).click()
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page.getByText("Phone number is required", {exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("CreatePhoneBlacklistEntry")).toEqual([])

        await page.getByRole("textbox", {name: "Phone number"}).fill(" +15550005678 ")
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page.getByText("Saved successfully", {exact: true})).toBeVisible()
        expect(wire(portal, "CreatePhoneBlacklistEntry")).toEqual([
            {
                variables: {election_event_id: EVENT_ID, phone_e164: "+15550005678", reason: null},
                role: "phone-blacklist-create",
            },
        ])
        await expect(row(page, "+15550005678")).toBeVisible()
    })

    test("edits the reason of a blocked number", async ({page, portal}) => {
        mockBlocklist(portal)
        await openIvr(page, portal, "Blocklist")
        await edit(row(page, "+15550001234")).click()
        await expect(page.getByRole("textbox", {name: "Phone number"})).toBeDisabled()
        await expect(page.getByRole("textbox", {name: "Phone number"})).toHaveValue("+15550001234")
        await page.getByRole("textbox", {name: "Reason"}).fill("Repeated hoax calls")
        await page.getByRole("button", {name: "Save", exact: true}).click()

        await expect(page.getByText("Saved successfully", {exact: true})).toBeVisible()
        expect(wire(portal, "UpdatePhoneBlacklistEntry")).toEqual([
            {
                variables: {id: FIRST_ID, reason: "Repeated hoax calls"},
                role: "phone-blacklist-update",
            },
        ])
        await expect(row(page, "+15550001234")).toContainText("Repeated hoax calls")
    })

    test("unblocks a number after confirmation", async ({page, portal}) => {
        mockBlocklist(portal)
        await openIvr(page, portal, "Blocklist")
        await remove(row(page, "+15550001234")).click()
        await page.getByRole("dialog").getByRole("button", {name: "Cancel", exact: true}).click()
        expect(portal.graphql.callsTo("DeletePhoneBlacklistEntry")).toEqual([])

        await remove(row(page, "+15550001234")).click()
        await expect(page.getByRole("dialog")).toContainText(
            "Are you sure you want to delete this item?"
        )
        await page.getByRole("dialog").getByRole("button", {name: "Delete", exact: true}).click()
        await expect(page.getByText("Deleted successfully", {exact: true})).toBeVisible()
        expect(wire(portal, "DeletePhoneBlacklistEntry")).toEqual([
            {
                variables: {id: FIRST_ID, election_event_id: EVENT_ID},
                role: "phone-blacklist-delete",
            },
        ])

        await expect(
            page.getByText("There are no entries in the blocklist", {exact: true})
        ).toBeVisible()
        await page.getByRole("button", {name: "Add", exact: true}).click()
        await expect(page.getByRole("textbox", {name: "Phone number"})).toBeEditable()
    })

    test("failed blocks, edits and unblocks report the error", async ({page, portal}) => {
        mockBlocklist(portal)
        for (const operation of [
            "CreatePhoneBlacklistEntry",
            "UpdatePhoneBlacklistEntry",
            "DeletePhoneBlacklistEntry",
        ])
            portal.graphql.on(operation, () => ({errors: [{message: "permission denied"}]}))
        await openIvr(page, portal, "Blocklist")

        await page.getByRole("button", {name: "Add", exact: true}).click()
        await page.getByRole("textbox", {name: "Phone number"}).fill("+15550005678")
        await page.getByRole("textbox", {name: "Reason"}).fill("Robocalls")
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page.getByText("Failed to save", {exact: true})).toBeVisible()
        expect(wire(portal, "CreatePhoneBlacklistEntry")).toEqual([
            {
                variables: {
                    election_event_id: EVENT_ID,
                    phone_e164: "+15550005678",
                    reason: "Robocalls",
                },
                role: "phone-blacklist-create",
            },
        ])
        await expect(page.getByRole("textbox", {name: "Phone number"})).toHaveValue("+15550005678")
        await page.getByRole("button", {name: "Cancel", exact: true}).click()

        await edit(row(page, "+15550001234")).click()
        await page.getByRole("textbox", {name: "Reason"}).fill("")
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page.getByText("Failed to save", {exact: true})).toBeVisible()
        expect(wire(portal, "UpdatePhoneBlacklistEntry")).toEqual([
            {variables: {id: FIRST_ID, reason: null}, role: "phone-blacklist-update"},
        ])
        await page.getByRole("button", {name: "Cancel", exact: true}).click()

        await remove(row(page, "+15550001234")).click()
        await page.getByRole("dialog").getByRole("button", {name: "Delete", exact: true}).click()
        await expect(page.getByText("Failed to delete", {exact: true})).toBeVisible()
        await expect(row(page, "+15550001234")).toBeVisible()
    })

    test.describe("with read-only access", () => {
        test.use({roles: [...IVR_ROLES, "phone-blacklist-read"]})

        test("offers no add, edit or delete controls", async ({page, portal}) => {
            mockBlocklist(portal)
            await openIvr(page, portal, "Blocklist")
            await expect(row(page, "+15550001234")).toBeVisible()
            await expect(page.getByRole("button", {name: "Add", exact: true})).toHaveCount(0)
            await expect(row(page, "+15550001234").getByRole("button")).toHaveCount(0)
            await expect(page.getByRole("columnheader", {name: "Actions"})).toHaveCount(0)
        })

        // PhoneBlacklist's empty state renders its Add button without checking phone-blacklist-create.
        test("an empty blocklist offers no add button", async ({page, portal}) => {
            mockBlocklist(portal, [])
            await openIvr(page, portal, "Blocklist")
            await expect(
                page.getByText("There are no entries in the blocklist", {exact: true})
            ).toBeVisible()

            await expect(page.getByRole("button", {name: "Add", exact: true})).toHaveCount(0)
        })
    })
})

test.describe("emulator", () => {
    const COUNCIL_ID = "30000000-0000-4000-8000-000000000001"
    const MAYOR_ID = "30000000-0000-4000-8000-000000000002"
    const REFERENDUM_ID = "30000000-0000-4000-8000-000000000003"
    const MAYOR_STYLE =
        '{"election_id":"30000000-0000-4000-8000-000000000002","contests":["mayor"]}'
    const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/
    const say = (text: string, language = "en-US", voice_id = "Joanna"): PromptInfo => ({
        prompt_text: `<speak>${text}</speak>`,
        language,
        voice_id,
    })
    const askVoterId = (text: string) =>
        ({
            type: "ExpectInput",
            prompt: say(text),
            valid_inputs: "0123456789#",
            max_digits: 4,
            timeout: 10,
        }) as const
    const CALL: EmulatorScript = {
        connect: [
            {type: "Prompt", prompt: say("Welcome to the council election.")},
            {type: "Noop"},
            {type: "Prompt", prompt: say("Bienvenido.", "es-ES", "Lucia")},
            askVoterId("Enter your voter id followed by the hash key."),
        ],
        input: {
            "9": [askVoterId("That voter id is not valid. Please try again.")],
            "123#": [
                {type: "Prompt", prompt: say("Your vote has been recorded.")},
                {type: "Disconnect", prompt: say("Goodbye.")},
            ],
        },
        timeout: [askVoterId("We did not receive your voter id.")],
    }

    function mockBallotData(portal: AdminPortal) {
        mockIvrEvent(portal)
        portal.graphql.on("sequent_backend_area", listOf("sequent_backend_area", [NORTH_AREA]))
        portal.graphql.on(
            "sequent_backend_election",
            listOf("sequent_backend_election", [
                election(REFERENDUM_ID, "Referendum"),
                election(MAYOR_ID, "Mayor election"),
                election(COUNCIL_ID, "Council election"),
            ])
        )
        portal.graphql.on(
            "sequent_backend_ballot_style",
            listOf("sequent_backend_ballot_style", [
                ballotStyle(
                    "71000000-0000-4000-8000-000000000001",
                    COUNCIL_ID,
                    '{"election_id":"council"}'
                ),
                ballotStyle("71000000-0000-4000-8000-000000000002", MAYOR_ID, MAYOR_STYLE),
                // Not published yet, so the referendum is not offered.
                ballotStyle("71000000-0000-4000-8000-000000000003", REFERENDUM_ID, null),
            ])
        )
    }

    async function startCall(page: Page) {
        const start = page.getByRole("button", {name: "Start new session", exact: true})
        const noStyles = page.getByText(
            "No published ballot styles found matching your selections",
            {
                exact: true,
            }
        )
        await expect(noStyles).toBeVisible()
        await expect(start).toBeDisabled()
        await page.getByLabel("Area").fill("Nor")
        await page.getByRole("option", {name: "North district", exact: true}).click()
        await page.getByLabel("Elections").click()
        await expect(page.getByRole("option")).toHaveText(["Council election", "Mayor election"])
        await page.getByRole("option", {name: "Mayor election", exact: true}).click()
        await expect(noStyles).toHaveCount(0)
        await start.click()
    }

    const transcript = (page: Page) => page.getByTitle(/^[a-z]{2}-[A-Z]{2}, /).locator("..")

    test("says it is unavailable when the emulator module is not deployed", async ({
        page,
        portal,
    }) => {
        mockIvrEvent(portal)
        await openIvr(page, portal, "Emulator")
        await expect(
            page.getByText("The emulator system is not available in your environment", {
                exact: true,
            })
        ).toBeVisible()
        await expect(page.getByRole("button", {name: "Start new session"})).toHaveCount(0)
    })

    test("reports a module that fails to load", async ({page, portal}) => {
        mockIvrEvent(portal)
        let release = () => {}
        const ready = new Promise<void>((resolve) => (release = resolve))
        await serveEmulator(page, CALL, {ready, failInit: true})
        await openIvr(page, portal, "Emulator")
        await expect(page.getByText("Loading the emulator system", {exact: true})).toBeVisible()
        release()
        await expect(
            page.getByText("Error loading the emulator system", {exact: true})
        ).toBeVisible()
        await expect(page.getByText("Loading the emulator system", {exact: true})).toHaveCount(0)
    })

    test("runs a call from the area and election selection to the hang-up", async ({
        page,
        portal,
    }) => {
        mockBallotData(portal)
        await serveEmulator(page, CALL)
        await openIvr(page, portal, "Emulator")
        await expect(
            page.getByText('The valid voter id and pin are "123" and "123".')
        ).toBeVisible()
        await page.getByLabel("Blocklist the caller").check()
        await startCall(page)

        await expect(transcript(page)).toHaveText(
            [
                "EN Welcome to the council election.",
                "ES Bienvenido.",
                "EN Enter your voter id followed by the hash key.",
            ],
            {useInnerText: true}
        )
        // Only the selected area's published styles may reach the emulator.
        expect(
            portal.graphql.callsTo("sequent_backend_ballot_style").map((call) => call.variables)
        ).toEqual([
            {
                where: {
                    _and: [
                        {tenant_id: {_eq: TENANT_ID}},
                        {election_event_id: {_eq: EVENT_ID}},
                        {area_id: {_eq: NORTH_AREA.id}},
                        {deleted_at: {_is_null: true}},
                    ],
                },
                limit: 300,
                offset: 0,
                order_by: {created_at: "desc"},
            },
        ])
        const log = await emulatorLog(page)
        expect(log.binaries).toEqual(["/wasm/ivr_emulator_wasm_bg.wasm"])
        expect(log.configs).toEqual([
            {
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                caller_number: "+1234567890",
                contact_id: expect.stringMatching(UUID),
                blacklisted_numbers: ["+1234567890"],
                open_elections: [MAYOR_ID],
                election_event: expect.any(String),
                ballot_styles: [MAYOR_STYLE],
            },
        ])
        expect(JSON.parse(String(log.configs[0].election_event))).toMatchObject({
            id: EVENT_ID,
            tenant_id: TENANT_ID,
            annotations: IVR_ANNOTATIONS,
        })

        const input = page.getByPlaceholder(
            "Enter your input (max digits=4, valid inputs=0123456789#, timeout=10s)"
        )
        const sendDtmf = page.getByRole("button", {name: "Send DTMF input"})
        await expect(sendDtmf).toBeDisabled()
        await page.getByRole("button", {name: "Send timeout"}).click()
        await expect(transcript(page).last()).toHaveText("EN We did not receive your voter id.", {
            useInnerText: true,
        })
        await input.fill("9")
        await input.press("Enter")
        await expect(transcript(page).last()).toHaveText(
            "EN That voter id is not valid. Please try again.",
            {useInnerText: true}
        )
        await input.pressSequentially("1a23#")
        await expect(input).toHaveValue("123#")
        await sendDtmf.click()

        await expect(page.getByText("Disconnected", {exact: true})).toBeVisible()
        await expect(transcript(page)).toHaveText(
            [
                "EN Welcome to the council election.",
                "ES Bienvenido.",
                "EN Enter your voter id followed by the hash key.",
                "EN We did not receive your voter id.",
                "EN That voter id is not valid. Please try again.",
                "EN Your vote has been recorded.",
                "EN Goodbye.",
            ],
            {useInnerText: true}
        )
        await expect(input).toHaveCount(0)
        expect(await emulatorLog(page)).toMatchObject({
            inputs: ["9", "123#"],
            timeouts: 1,
            freed: 1,
        })

        await page.getByRole("button", {name: "End the session", exact: true}).click()
        await expect(
            page.getByRole("button", {name: "Start new session", exact: true})
        ).toBeVisible()
    })

    test("shows the emulator error when the call breaks and frees an abandoned call", async ({
        page,
        portal,
    }) => {
        mockBallotData(portal)
        await serveEmulator(page, {
            connect: [askVoterId("Enter your voter id followed by the hash key.")],
            input: {"1": [{type: "Fail", message: "ballot style could not be decoded"}]},
            timeout: [],
        })
        await openIvr(page, portal, "Emulator")
        await startCall(page)
        const input = page.getByPlaceholder(
            "Enter your input (max digits=4, valid inputs=0123456789#, timeout=10s)"
        )
        await input.fill("1")
        await page.getByRole("button", {name: "Send DTMF input"}).click()
        await expect(
            page.getByRole("alert").filter({hasText: "Error: ballot style could not be decoded"})
        ).toBeVisible()
        await expect.poll(async () => (await emulatorLog(page)).freed).toBe(1)
        expect((await emulatorLog(page)).configs[0]).toMatchObject({blacklisted_numbers: []})

        await page.getByRole("button", {name: "End the session", exact: true}).click()
        await startCall(page)
        await expect(transcript(page)).toHaveText(
            ["EN Enter your voter id followed by the hash key."],
            {
                useInnerText: true,
            }
        )
        await page.getByRole("button", {name: "End the session", exact: true}).click()
        await expect.poll(async () => (await emulatorLog(page)).freed).toBe(2)
    })
})
