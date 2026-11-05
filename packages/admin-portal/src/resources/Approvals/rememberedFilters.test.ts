// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IQueueFilter, restorableListParams} from "./rememberedFilters"

// The queue's filter inputs: where each writes its value and, for a choice, what it offers.
const FILTERS: IQueueFilter[] = [
    {source: "q"},
    {source: "status", choices: ["pending", "accepted", "rejected"]},
    {source: "verification_type", choices: ["MANUAL", "AUTOMATIC"]},
    {source: "applicant_id"},
    {source: "id"},
    {source: "applicant_data.firstName._ilike"},
    {source: "applicant_data.dateOfBirth"},
]

// What react-admin stores after an administrator chose Approved and searched for "bob".
const SAVED = {
    filter: {status: "accepted", q: "bob"},
    displayedFilters: {status: true},
    sort: "created_at",
    order: "DESC",
    page: 2,
    perPage: 25,
}

describe("restorableListParams", () => {
    it("restores the filters, page and sorting an administrator chose", () => {
        expect(restorableListParams(SAVED, FILTERS)).toEqual(SAVED)
    })

    it("restores the profile fields' filters, which react-admin nests", () => {
        const saved = {
            ...SAVED,
            filter: {applicant_data: {firstName: {_ilike: "Bo"}, dateOfBirth: "1990-01-02"}},
        }
        expect(restorableListParams(saved, FILTERS)).toEqual(saved)
    })

    it("keeps a cleared filter cleared", () => {
        // No status at all: the administrator removed it, which is not the same as no saved list.
        const cleared = {...SAVED, filter: {}, displayedFilters: {}}
        expect(restorableListParams(cleared, FILTERS)).toEqual(cleared)
    })

    it("forgets the applications a support reference was resolved to", () => {
        // They are looked up again, in the queue's current scope.
        const saved = {
            ...SAVED,
            filter: {
                q: "Q5KW-8RFX",
                support_reference: {reference: "Q5KW-8RFX", ids: ["stale-application"]},
            },
        }
        expect(restorableListParams(saved, FILTERS)).toEqual({...SAVED, filter: {q: "Q5KW-8RFX"}})
    })

    it.each([
        ["the event", {election_event_id: "another-event"}],
        ["the election's label", {permission_label: "south"}],
        ["the tenant", {tenant_id: "another-tenant"}],
    ])("discards saved filters that would choose %s", (_, scope) => {
        expect(
            restorableListParams({...SAVED, filter: {...SAVED.filter, ...scope}}, FILTERS)
        ).toBeUndefined()
    })

    it.each([
        ["nothing", undefined],
        ["unreadable JSON, which the store returns as text", "{not json"],
        ["a list", [SAVED]],
        ["filters that aren't an object", {...SAVED, filter: "garbage"}],
        ["a status the queue no longer offers", {...SAVED, filter: {status: "archived"}}],
        ["a value that isn't text", {...SAVED, filter: {q: 42}}],
        [
            "a profile field that no longer exists",
            {...SAVED, filter: {applicant_data: {embassy: "Paris"}}},
        ],
        [
            "a profile field compared another way",
            {...SAVED, filter: {applicant_data: {firstName: "Bob"}}},
        ],
        ["shown filters that aren't an object", {...SAVED, displayedFilters: ["status"]}],
        ["a shown filter the queue doesn't have", {...SAVED, displayedFilters: {retired: true}}],
        ["a shown filter that isn't true or false", {...SAVED, displayedFilters: {status: "yes"}}],
        ["parameters react-admin doesn't store", {...SAVED, filterValues: {}}],
    ])("discards %s", (_, saved) => {
        expect(restorableListParams(saved, FILTERS)).toBeUndefined()
    })

    it("restores a sort by any column the administrator chose to show", () => {
        const saved = {...SAVED, sort: "applicant_id", order: "ASC"}
        expect(restorableListParams(saved, FILTERS)).toEqual(saved)
    })

    const {sort: _sort, order: _order, ...UNSORTED} = SAVED
    const {page: _page, ...FIRST_PAGE} = SAVED
    const {perPage: _perPage, ...DEFAULT_PAGE_SIZE} = SAVED
    it.each([
        ["a sort that isn't a column", {...SAVED, sort: 42}, UNSORTED],
        ["a sort by several columns", {...SAVED, sort: "created_at,id"}, UNSORTED],
        ["a sort by a nested path", {...SAVED, sort: "created_at.x"}, UNSORTED],
        ["an unknown order", {...SAVED, order: "SIDEWAYS"}, UNSORTED],
        ["a page size the queue doesn't offer", {...SAVED, perPage: 100000}, DEFAULT_PAGE_SIZE],
        ["a page that doesn't exist", {...SAVED, page: 0}, FIRST_PAGE],
    ])("keeps the filters but forgets %s", (_, saved, restored) => {
        // React-admin uses the queue's own sorting and pagination for what is forgotten.
        expect(restorableListParams(saved, FILTERS)).toEqual(restored)
    })

    it("restores a saved list without sorting or pagination", () => {
        expect(restorableListParams({filter: {status: "rejected"}}, FILTERS)).toEqual({
            filter: {status: "rejected"},
        })
    })
})
