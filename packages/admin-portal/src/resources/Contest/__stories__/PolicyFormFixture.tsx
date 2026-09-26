// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {initCore} from "@sequentech/ui-core"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {
    RecordContextProvider,
    ResourceContextProvider,
    SaveContextProvider,
    type RaRecord,
} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {AuthContext} from "@/providers/AuthContextProvider"
import {ContestDataForm} from "../EditContestDataForm"
import {ElectionDataForm} from "../../Election/ElectionDataForm"

export const ELECTION_ID = "33333333-3333-4333-8333-333333333333"
export const CONTEST_ID = "44444444-4444-4444-8444-444444444444"
const language = {enabled_language_codes: ["en", "es"], default_language_code: "en"}
const event = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    presentation: {language_conf: language, contest_encryption_policy: "multiple-contests"},
}
const election = {
    id: ELECTION_ID,
    election_event_id: EVENT_ID,
    tenant_id: TENANT_ID,
    status: {allow_tally: "allowed"},
    voting_channels: ["ONLINE"],
    presentation: {
        language_conf: language,
        i18n: {en: {name: "Council election"}, es: {name: "Elección municipal"}},
        contests_order: "alphabetical",
    },
}
const contest = {
    id: CONTEST_ID,
    election_id: ELECTION_ID,
    election_event_id: EVENT_ID,
    tenant_id: TENANT_ID,
    min_votes: 0,
    max_votes: 2,
    winning_candidates_num: 1,
    counting_algorithm: "plurality-at-large",
    presentation: {
        columns: 1,
        i18n: {en: {name: "Council members"}, es: {name: "Miembros del consejo"}},
    },
}
/** Which form renders and what the signed-in administrator may do. */
export interface PolicyScenario {
    kind: "contest" | "election"
    canEdit: boolean
    preferential: boolean
}
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>
/** The form's save handler; each story starts with no calls. */
export const save = fn(async (_values: Record<string, unknown>) => undefined)
export function PolicyFormFixture({kind, canEdit, preferential}: PolicyScenario) {
    const auth = useContext(AuthContext)
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <AuthContext.Provider
                value={{
                    ...auth,
                    tenantId: TENANT_ID,
                    isAuthorized: (_super, _tenant, permission) =>
                        canEdit && ["election-write", "contest-write"].includes(String(permission)),
                }}
            >
                <ResourceContextProvider value={`sequent_backend_${kind}`}>
                    <SaveContextProvider value={{save, saving: false, mutationMode: "pessimistic"}}>
                        <RecordContextProvider
                            value={
                                kind === "contest"
                                    ? {
                                          ...contest,
                                          counting_algorithm: preferential
                                              ? "instant-runoff"
                                              : "plurality-at-large",
                                      }
                                    : election
                            }
                        >
                            {kind === "contest" ? <ContestDataForm /> : <ElectionDataForm />}
                        </RecordContextProvider>
                    </SaveContextProvider>
                </ResourceContextProvider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}

/** Starts each story with fresh boundaries; returns the check of unexpected requests. */
export async function setUpPolicyForm() {
    await initCore()
    save.mockClear()
    boundary = graphqlBoundary({})
    data = dataBoundary({
        getOne: async <RecordType extends RaRecord>(
            resource: string,
            {id}: {id: string | number}
        ) => {
            const records: Record<string, RaRecord> = {
                sequent_backend_election_event: event,
                sequent_backend_election: election,
                sequent_backend_tenant: {id: TENANT_ID, settings: {languages: ["en", "es"]}},
                sequent_backend_document: {id: TENANT_ID, name: null},
            }
            const record = records[resource]
            if (!record || record.id !== id) {
                data.unexpected.push(`${resource}/${id}`)
                throw new Error("Unexpected record")
            }
            return {data: record as RecordType}
        },
        getList: async <RecordType extends RaRecord>(resource: string) => {
            if (!["sequent_backend_contest", "sequent_backend_candidate"].includes(resource)) {
                data.unexpected.push(resource)
                throw new Error("Unexpected list")
            }
            return {data: [] as RecordType[], total: 0}
        },
    })
    const services = boundary
    const records = data
    return () => {
        expect(services.unexpected).toEqual([])
        expect(records.unexpected).toEqual([])
    }
}

export async function choose(canvasElement: HTMLElement, label: string | RegExp, value: string) {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: label}))
    await userEvent.click(await within(document.body).findByRole("option", {name: value}))
}

export async function openPolicies(canvasElement: HTMLElement, kind: "contest" | "election") {
    const canvas = within(canvasElement)
    const name = await canvas.findByDisplayValue(
        kind === "contest" ? "Council members" : "Council election"
    )
    // Keep a real edited field so saving a policy's original/default value also
    // exercises submission instead of clicking a pristine disabled Save button.
    await userEvent.type(name, " (policy check)")
    await userEvent.click(canvas.getByRole("button", {name: "Ballot Design"}))
    await waitFor(() => expect(name).not.toBeVisible())
    return canvas
}

/** Saves each choice of a contest policy and checks the saved wire value. */
export function contestPolicyChoices(
    label: string | RegExp,
    field: string,
    choices: ReadonlyArray<readonly [string, string]>
) {
    return {
        play: async ({canvasElement}: {canvasElement: HTMLElement}) => {
            const canvas = await openPolicies(canvasElement, "contest")
            for (const [option, wireValue] of choices) {
                save.mockClear()
                await choose(canvasElement, label, option)
                await userEvent.click(canvas.getByRole("button", {name: "Save"}))
                await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
                expect(save.mock.calls[0][0]).toMatchObject({
                    id: CONTEST_ID,
                    tenant_id: TENANT_ID,
                    election_id: ELECTION_ID,
                    presentation: {[field]: wireValue},
                })
            }
        },
    }
}

/** GraphQL operations of the story. */
export const policyOperations = () => boundary.calls
