// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EExportLogsFormat,
    EExportLogsScope,
    exportLogsScopes,
    exportScopeReady,
    eventExportVariables,
    tenantExportScope,
    tenantExportVariables,
    type IExportLogsChoice,
} from "./exportLogsRequest"
import {ELogScope} from "./logScope"

const RANGE = {
    createdFrom: "2028-02-01T00:00:00Z",
    createdTo: "2028-02-03T23:59:00Z",
    timeZone: "Europe/Madrid",
}
const EVENT = "20000000-0000-4000-8000-000000000001"
const choice = (overrides: Partial<IExportLogsChoice> = {}): IExportLogsChoice => ({
    scope: EExportLogsScope.TENANT_LOG,
    username: "",
    electionId: null,
    areaId: null,
    ...overrides,
})

describe("the scopes each log offers", () => {
    it("offers the whole tenant log or one account for the tenant log", () => {
        expect(exportLogsScopes(ELogScope.TENANT)).toEqual([
            EExportLogsScope.TENANT_LOG,
            EExportLogsScope.ACCOUNT,
        ])
    })

    it("offers the event, one election, one area or one account for an event's log", () => {
        expect(exportLogsScopes(ELogScope.EVENT)).toEqual([
            EExportLogsScope.ALL,
            EExportLogsScope.ELECTION,
            EExportLogsScope.AREA,
            EExportLogsScope.ACCOUNT,
        ])
    })
})

describe("what a scope needs before it can be exported", () => {
    it.each([
        [EExportLogsScope.TENANT_LOG, {}, true],
        [EExportLogsScope.ALL, {}, true],
        [EExportLogsScope.ACCOUNT, {username: ""}, false],
        [EExportLogsScope.ACCOUNT, {username: "   "}, false],
        [EExportLogsScope.ACCOUNT, {username: "rosa.marin"}, true],
        [EExportLogsScope.ACCOUNT_WITH_EVENTS, {username: ""}, false],
        [EExportLogsScope.ACCOUNT_WITH_EVENTS, {username: "rosa.marin"}, true],
        [EExportLogsScope.ELECTION, {}, false],
        [EExportLogsScope.ELECTION, {electionId: "e"}, true],
        [EExportLogsScope.AREA, {electionId: "e"}, false],
        [EExportLogsScope.AREA, {areaId: "a"}, true],
    ])("%s with %j: %s", (scope, overrides, ready) => {
        expect(exportScopeReady(choice({scope, ...overrides}))).toBe(ready)
    })
})

describe("the tenant log's export", () => {
    it("sends the whole log without an account", () => {
        expect(
            tenantExportVariables(choice({username: "left.over"}), EExportLogsFormat.SQL, RANGE)
        ).toEqual({format: "SQL", scope: "TENANT_LOG", username: null, ...RANGE})
    })

    it("sends one account, trimmed", () => {
        expect(
            tenantExportVariables(
                choice({scope: EExportLogsScope.ACCOUNT, username: " rosa.marin "}),
                EExportLogsFormat.CSV,
                {createdFrom: null, createdTo: null, timeZone: "UTC"}
            )
        ).toEqual({
            format: "CSV",
            scope: "ACCOUNT",
            username: "rosa.marin",
            createdFrom: null,
            createdTo: null,
            timeZone: "UTC",
        })
    })

    it("includes the account's election event entries when asked", () => {
        expect(tenantExportScope(EExportLogsScope.ACCOUNT, true)).toBe(
            EExportLogsScope.ACCOUNT_WITH_EVENTS
        )
        expect(tenantExportScope(EExportLogsScope.ACCOUNT, false)).toBe(EExportLogsScope.ACCOUNT)
        expect(tenantExportScope(EExportLogsScope.TENANT_LOG, true)).toBe(
            EExportLogsScope.TENANT_LOG
        )
        expect(
            tenantExportVariables(
                choice({scope: EExportLogsScope.ACCOUNT_WITH_EVENTS, username: "rosa.marin"}),
                EExportLogsFormat.PDF,
                RANGE
            )
        ).toMatchObject({format: "PDF", scope: "ACCOUNT_WITH_EVENTS", username: "rosa.marin"})
    })
})

describe("an election event's export", () => {
    it("sends the whole event by default", () => {
        expect(
            eventExportVariables(
                EVENT,
                choice({scope: EExportLogsScope.ALL, username: "x", electionId: "e", areaId: "a"}),
                EExportLogsFormat.CSV,
                RANGE
            )
        ).toEqual({
            electionEventId: EVENT,
            format: "CSV",
            scope: "ALL",
            electionId: null,
            areaId: null,
            username: null,
            ...RANGE,
        })
    })

    it.each([
        [EExportLogsScope.ELECTION, {electionId: "e", areaId: null, username: null}],
        [EExportLogsScope.AREA, {electionId: null, areaId: "a", username: null}],
        [EExportLogsScope.ACCOUNT, {electionId: null, areaId: null, username: "rosa.marin"}],
    ])("sends only what %s needs", (scope, sent) => {
        expect(
            eventExportVariables(
                EVENT,
                choice({scope, username: "rosa.marin", electionId: "e", areaId: "a"}),
                EExportLogsFormat.SQL,
                RANGE
            )
        ).toEqual({electionEventId: EVENT, format: "SQL", scope, ...sent, ...RANGE})
    })
})
