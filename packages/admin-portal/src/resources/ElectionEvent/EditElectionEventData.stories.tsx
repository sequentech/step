// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider, type DataProvider, type UpdateParams} from "react-admin"
import {Route, Routes} from "react-router-dom"
import {i18n, initCore} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, eventPresentation} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {EditElectionEventData} from "./EditElectionEventData"
import {
    SECOND_ELECTION_ID,
    eventDataArgs,
    eventDataBoundaries,
    type EventDataScenario,
} from "./__stories__/EventDataFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const RESOURCE = "sequent_backend_election_event"

let boundaries: ReturnType<typeof eventDataBoundaries>
let dataProvider: DataProvider

const PRIVACY_REFUSAL = "Pseudonyms require signed receipts and voter signatures"

type Presentation = Record<string, unknown> & {receipts?: Record<string, unknown>}

/**
 * The database's check on the event's presentation (trigger
 * check_ballot_privacy_identity, migration 1792400000000): the update writes the
 * whole presentation, and a pseudonymous event refuses one without signed
 * receipts and voter signatures. A refused update is recorded too.
 */
function privacyChecked(provider: DataProvider): DataProvider {
    return new Proxy(provider, {
        get(target, key, receiver) {
            const original: unknown = Reflect.get(target, key, receiver)
            if (key !== "update" || typeof original !== "function") return original
            return async (resource: string, params: UpdateParams) => {
                const presentation = (params.data?.presentation ?? {}) as Presentation
                if (
                    resource === RESOURCE &&
                    presentation.ballot_identity === "pseudonym" &&
                    (presentation.voter_signing_policy !== "with-signature" ||
                        presentation.receipts?.policy !== "signed-by-ballot-box")
                ) {
                    boundaries.data.writes.push({method: "update", resource, params: {...params}})
                    throw new Error(PRIVACY_REFUSAL)
                }
                return Reflect.apply(original, target, [resource, params])
            }
        },
    })
}

// The event tabs render the section on the event's route.
function Fixture() {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={boundaries.graphql}
            dataProvider={dataProvider}
            role={permissions}
            tenant={tenant}
        >
            <WidgetsContextProvider>
                <ResourceContextProvider value={RESOURCE}>
                    <Routes>
                        <Route path={`/${RESOURCE}/:id`} element={<EditElectionEventData />} />
                        <Route path={`/${RESOURCE}`} element={null} />
                    </Routes>
                </ResourceContextProvider>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/EditElectionEventData",
    component: EditElectionEventData,
    args: eventDataArgs,
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {router: {initialEntries: [`/${RESOURCE}/${EVENT_ID}`]}},
    beforeEach: async ({args}) => {
        boundaries = eventDataBoundaries(args)
        dataProvider = privacyChecked(boundaries.data.provider)
        await Promise.all([boundaries.graphql.ready, initCore()])
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies WidgetMeta<EventDataScenario>
export default meta
type Story = StoryObj<EventDataScenario>

const field = (key: string) => i18n.t(`electionEventScreen.field.${key}`)
const location = (canvasElement: HTMLElement) =>
    within(canvasElement).getByLabelText("Current location")

async function loaded(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await expect(await canvas.findByRole("textbox", {name: field("name")})).toHaveValue(
        "Council event"
    )
    return canvas
}

const writes = (resource: string) =>
    boundaries.data.writes.filter((write) => write.resource === resource)

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        expect(
            boundaries.data.calls.find(
                ({method, args: [resource]}) => method === "getOne" && resource === RESOURCE
            )?.args[1]
        ).toMatchObject({id: EVENT_ID})
        expect(boundaries.data.writes).toEqual([])
    },
}

export const SaveTheEvent: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        const description = canvas.getByRole("textbox", {name: field("description")})
        await userEvent.clear(description)
        await userEvent.type(description, "Council members for 2026")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(writes(RESOURCE)).toHaveLength(1))
        const [write] = writes(RESOURCE)
        expect(write.method).toBe("update")
        // The description follows the English presentation; since migration
        // 1772358027729 the name and alias live only in the presentation.
        expect(write.params.data).toMatchObject({
            id: EVENT_ID,
            description: "Council members for 2026",
            presentation: {
                language_conf: {enabled_language_codes: ["en", "es"], default_language_code: "en"},
            },
        })
        for (const key of [
            "enabled_languages",
            "electionsOrder",
            "resultsWebsitePolicy",
            "name",
            "alias",
        ])
            expect(write.params.data).not.toHaveProperty(key)
        expect(writes("sequent_backend_election")).toEqual([])
        // The saved event is read again.
        await waitFor(() =>
            expect(
                boundaries.data.calls.filter(
                    ({method, args: [resource]}) => method === "getOne" && resource === RESOURCE
                ).length
            ).toBeGreaterThan(1)
        )
    },
}

export const SaveACustomElectionOrder: Story = {
    args: {customOrder: true},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        const description = canvas.getByRole("textbox", {name: field("description")})
        await userEvent.type(description, ".")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(writes(RESOURCE)).toHaveLength(1))
        // The elections, sorted by their order, are numbered from zero.
        expect(
            writes("sequent_backend_election").map(({params}) => [
                params.id,
                (params.data as {presentation: {sort_order: number}}).presentation.sort_order,
            ])
        ).toEqual([
            [SECOND_ELECTION_ID, 0],
            [STORY_IDS.election, 1],
        ])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(boundaries.data.calls.map(({method}) => method)).toContain("getOne")
        )
        expect(within(canvasElement).queryByRole("textbox", {name: field("name")})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        const notice = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(notice).toBeVisible())
        await waitFor(() => expect(location(canvasElement).textContent).toBe(`/${RESOURCE}`))
    },
}

// meta#15035: a save writes the whole presentation, so it keeps every setting the
// administrator did not change, whether or not the form shows it or lets it change.

const edit = (key: string) => i18n.t(`electionEventScreen.edit.${key}`)
const receiptsPolicy = () => field("receiptsPolicy.policyLabel")
const voterSigningPolicy = () => field("voterSigningPolicy.policyLabel")
const sealPolicy = () => field("ballotBoxSealPolicy.policyLabel")

/** A pseudonymous event as the database requires it, with settings the form doesn't know. */
const PSEUDONYMOUS: Presentation = {
    ballot_identity: "pseudonym",
    network_data_policy: "do-not-store",
    receipts: {
        policy: "signed-by-ballot-box",
        checks_period_policy: "unlimited",
        unknown_receipts_setting: "kept",
    },
    voter_signing_policy: "with-signature",
    ballot_box_seal_policy: "do-not-seal",
    unknown_setting: {nested: {value: 7}},
}

const VOTER_ID: Presentation = {
    ballot_identity: "voter-id",
    receipts: {policy: "disabled", checks_period_policy: "unlimited"},
    voter_signing_policy: "no-signature",
    logo_url: "https://logo.example.test/council.png",
    unknown_setting: {nested: {value: 7}},
}

/** The advanced section shows JSON editors, whose item counts are below the contrast minimum. */
const advancedSection = {
    expectedFailure: {
        reason: "The JSON editors' item counts are light grey below the contrast minimum.",
        a11y: ["color-contrast"],
    },
}

/** Opens a section once the others have finished collapsing: one section is open at a time. */
async function openSection(canvasElement: HTMLElement, key: string) {
    const summary = within(canvasElement).getByRole("button", {name: edit(key)})
    await userEvent.click(summary)
    await waitFor(() => expect(summary).toHaveAttribute("aria-expanded", "true"))
    const opened = summary.closest(".MuiAccordion-root")
    await waitFor(() => {
        for (const section of Array.from(canvasElement.querySelectorAll(".MuiAccordion-root"))) {
            const collapse = section.querySelector(":scope > .MuiCollapse-root")
            if (section !== opened && collapse) expect(collapse).toHaveClass("MuiCollapse-hidden")
        }
    })
}

async function choose(canvasElement: HTMLElement, label: string, option: string) {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: label}))
    await userEvent.click(await within(document.body).findByRole("option", {name: option}))
    await waitFor(() => expect(within(document.body).queryByRole("listbox")).toBeNull())
}

/** Saves and returns the presentation the event's update wrote (or tried to). */
async function savedPresentation(canvasElement: HTMLElement) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
    await waitFor(() => expect(writes(RESOURCE)).toHaveLength(1))
    return (writes(RESOURCE)[0].params.data as {presentation: Presentation}).presentation
}

const notices = () =>
    within(document.body)
        .queryAllByText(/./, {selector: ".MuiSnackbarContent-message"})
        .map((notice) => notice.textContent)

/** Every stored setting but `changed` is written as it was. */
function expectKept(presentation: Presentation, stored: Presentation, changed: string[] = []) {
    for (const [key, value] of Object.entries(stored)) {
        if (!changed.includes(key)) expect({[key]: presentation[key]}).toEqual({[key]: value})
    }
}

/** Before voting opens, changing the seal policy keeps the pseudonymous settings. */
export const PseudonymousEventKeepsItsSettings: Story = {
    args: {neverOpened: true, presentation: PSEUDONYMOUS},
    parameters: advancedSection,
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        await openSection(canvasElement, "advancedConfigurations")
        // The ballot box receives only signed ballots: the voter signing is fixed.
        await expect(
            within(canvasElement).getByRole("combobox", {name: voterSigningPolicy()})
        ).toHaveAttribute("aria-disabled", "true")
        await choose(canvasElement, sealPolicy(), "Seal at close")
        const presentation = await savedPresentation(canvasElement)
        expect(presentation.ballot_box_seal_policy).toBe("seal-at-close")
        expectKept(presentation, PSEUDONYMOUS, ["ballot_box_seal_policy"])
        expect(presentation.receipts).toEqual(PSEUDONYMOUS.receipts)
        // The whole presentation, exactly: what was stored, the seal change, and only
        // the values the form adds to an event that lacks them. Any other added or
        // changed key fails here. (`lifecycle_policies: undefined` is not written.)
        expect(JSON.parse(JSON.stringify(presentation))).toEqual({
            ...eventPresentation,
            ...PSEUDONYMOUS,
            // The change the administrator made.
            ballot_box_seal_policy: "seal-at-close",
            // The edit view's transform writes the language settings it shows.
            language_conf: {
                ...eventPresentation.language_conf,
                language_detection_policy: "browser-detect",
            },
            // parseValues fills these in for every event (EditElectionEventDataForm.tsx).
            elections_order: "alphabetical",
            custom_urls: {},
            voting_portal_countdown_policy: {
                policy: "NO_COUNTDOWN",
                // Disabled under NO_COUNTDOWN: their defaults (60 s, 180 s), which
                // react-hook-form used to leave out, are now kept from the form.
                countdown_anticipation_secs: 60,
                countdown_alert_anticipation_secs: 180,
            },
            // onSave writes the results website policy the section shows (its defaults).
            results_website: JSON.stringify({
                status: "disabled",
                access: "public",
                visibility_scope: "full_event",
            }),
            // The defaultValue of each enabled input the stored event has no value for.
            voting_portal_datetime_format: "legacy-gb-24h",
            skip_election_list: false,
            show_user_profile: false,
            show_cast_vote_logs: "hide-logs-tab",
            voter_accessibility_settings_policy: "disabled",
            audio_instructions_policy: "disabled",
            automatic_recount_policy: "disabled",
            materials: {policy: "off"},
            contest_encryption_policy: "single-contest",
            locked_down: "not-locked-down",
            decoded_ballot_inclusion_policy: "not-included",
            ceremonies_policy: "manual-ceremonies",
            weighted_voting_policy: "disabled-weighted-voting",
            batch_anonymity_policy: "warn",
            voter_certificate_policy: "disabled",
        })
        await waitFor(() =>
            expect(boundaries.data.records[RESOURCE][0].presentation).toMatchObject({
                ballot_box_seal_policy: "seal-at-close",
            })
        )
        expect(notices()).not.toContain(PRIVACY_REFUSAL)
    },
}

/**
 * After voting opened, the receipts and the seal policy are locked; an ordinary
 * save writes them, and the rest of the presentation, as they were.
 */
export const PseudonymousEventSavedAfterVotingOpened: Story = {
    args: {presentation: PSEUDONYMOUS},
    parameters: advancedSection,
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        const description = canvas.getByRole("textbox", {name: field("description")})
        await userEvent.clear(description)
        await userEvent.type(description, "Council members for 2026")
        await openSection(canvasElement, "ballotReceipts")
        await expect(canvas.getByRole("combobox", {name: receiptsPolicy()})).toHaveAttribute(
            "aria-disabled",
            "true"
        )
        await openSection(canvasElement, "advancedConfigurations")
        await expect(await canvas.findByText(/Locked: voting has opened in /)).toBeVisible()
        const presentation = await savedPresentation(canvasElement)
        expectKept(presentation, PSEUDONYMOUS)
        expect(presentation.receipts).toEqual(PSEUDONYMOUS.receipts)
        expect(presentation).toMatchObject({
            i18n: {en: {description: "Council members for 2026"}},
        })
        expect(notices()).not.toContain(PRIVACY_REFUSAL)
    },
}

/** Turning signed receipts on also has voters sign, and the save writes both. */
export const VoterIdEventTurnsReceiptsOn: Story = {
    args: {neverOpened: true, presentation: VOTER_ID},
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        await openSection(canvasElement, "ballotReceipts")
        await choose(canvasElement, receiptsPolicy(), "Signed by the ballot box")
        const presentation = await savedPresentation(canvasElement)
        expect(presentation.receipts).toEqual({
            policy: "signed-by-ballot-box",
            checks_period_policy: "unlimited",
        })
        expect(presentation.voter_signing_policy).toBe("with-signature")
        expectKept(presentation, VOTER_ID, ["receipts", "voter_signing_policy"])
    },
}

/** Settings the administrator turns off or empties are saved so, not restored. */
export const VoterIdEventTurnsReceiptsOffAndClearsTheLogo: Story = {
    args: {
        neverOpened: true,
        presentation: {
            ...VOTER_ID,
            receipts: {policy: "signed-by-ballot-box", checks_period_policy: "unlimited"},
            voter_signing_policy: "with-signature",
        },
    },
    parameters: advancedSection,
    play: async ({canvasElement, args}) => {
        const canvas = await loaded(canvasElement)
        await openSection(canvasElement, "ballotDesign")
        await userEvent.clear(canvas.getByRole("textbox", {name: field("logoUrl")}))
        await openSection(canvasElement, "ballotReceipts")
        await choose(canvasElement, receiptsPolicy(), "Disabled")
        await openSection(canvasElement, "advancedConfigurations")
        await choose(canvasElement, voterSigningPolicy(), "No signature")
        const presentation = await savedPresentation(canvasElement)
        expect(presentation.receipts).toEqual({
            policy: "disabled",
            checks_period_policy: "unlimited",
        })
        expect(presentation.voter_signing_policy).toBe("no-signature")
        expect(presentation.logo_url ?? "").toBe("")
        expectKept(presentation, args.presentation as Presentation, [
            "receipts",
            "voter_signing_policy",
            "logo_url",
        ])
    },
}

/** A refused update says why, and nothing says the event changed. */
export const DatabaseRefusesTheSave: Story = {
    args: {neverOpened: true, presentation: PSEUDONYMOUS, writeError: PRIVACY_REFUSAL},
    parameters: advancedSection,
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        await openSection(canvasElement, "advancedConfigurations")
        await choose(canvasElement, sealPolicy(), "Seal at close")
        await savedPresentation(canvasElement)
        await waitFor(() => expect(notices()).toContain(PRIVACY_REFUSAL))
        await new Promise((resolve) => setTimeout(resolve, 500))
        expect(notices().filter((notice) => notice !== PRIVACY_REFUSAL)).toEqual([])
        expect(boundaries.data.records[RESOURCE][0].presentation).toMatchObject({
            ballot_box_seal_policy: "do-not-seal",
        })
        // The lifecycle policies, saved only after the event, are not.
        expect(boundaries.graphql.calls.map(({name}) => name)).not.toContain(
            "SaveLifecyclePolicies"
        )
        // The edit stays on screen, to save again.
        await expect(
            within(canvasElement).getByRole("combobox", {name: sealPolicy()})
        ).toHaveTextContent("Seal at close")
    },
}
