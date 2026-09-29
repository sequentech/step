// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    countThemeWidgets,
    filterCatalog,
    groupBySource,
    loadWidgetCatalog,
    type IWidgetCatalogEntry,
} from "./catalog"
import type {IMonitoringEditorApi} from "./api"
import {EMonitoringConfigKind} from "./types"

const ENTRIES: IWidgetCatalogEntry[] = [
    {id: "turnout", title: "Voter turnout", source: "voter_turnout", requirements: ["SW-F-0259"]},
    {id: "by-post", title: "Turnout by Post", source: "voter_turnout", requirements: ["SW-F-0260"]},
    {id: "attacks", title: "Attacks", source: "attack_detections", requirements: ["SW-F-0301"]},
]

describe("filterCatalog", () => {
    it("matches title, id, source or requirement, ignoring case", () => {
        expect(filterCatalog(ENTRIES, "sw-f-0260").map((entry) => entry.id)).toEqual(["by-post"])
        expect(filterCatalog(ENTRIES, "attack").map((entry) => entry.id)).toEqual(["attacks"])
        expect(filterCatalog(ENTRIES, "  ")).toHaveLength(3)
    })
})

describe("groupBySource", () => {
    it("keeps the first-seen order of sources and of widgets in each", () => {
        expect(groupBySource(ENTRIES)).toEqual([
            {source: "voter_turnout", entries: [ENTRIES[0], ENTRIES[1]]},
            {source: "attack_detections", entries: [ENTRIES[2]]},
        ])
    })
})

describe("loadWidgetCatalog", () => {
    it("reads every widget document and skips one that does not parse", async () => {
        const yaml: Record<string, string> = {
            turnout: "title: Voter turnout\nsource: voter_turnout\nrequirements: [SW-F-0259]\n",
            broken: "title: [unclosed\n",
        }
        const api = {
            listConfig: async () => [
                {kind: EMonitoringConfigKind.WIDGET, key: "turnout", revision: 1},
                {kind: EMonitoringConfigKind.THEME, key: "default", revision: 1},
                {kind: EMonitoringConfigKind.WIDGET, key: "broken", revision: 1},
            ],
            getConfig: async ({kind, key}: {kind: EMonitoringConfigKind; key: string}) => ({
                kind,
                key,
                yaml: yaml[key],
                revision: 1,
            }),
        } as unknown as IMonitoringEditorApi
        expect(await loadWidgetCatalog(api)).toEqual([
            {
                id: "turnout",
                title: "Voter turnout",
                source: "voter_turnout",
                requirements: ["SW-F-0259"],
            },
        ])
    })
})

describe("countThemeWidgets", () => {
    it("counts the widgets of every dashboard using the theme, this one as drafted", async () => {
        const yaml: Record<string, string> = {
            turnout: "id: turnout\ntitle: T\nlayout: [{widget: a, width: 6}]\n",
            security: "id: security\ntitle: S\ntheme: dark\nlayout: [{widget: b, width: 6}]\n",
            helpdesk:
                "id: helpdesk\ntitle: H\ntheme: default\nlayout: [{widget: c, width: 6}, {widget: d, width: 6}]\n",
        }
        const api = {
            listConfig: async () =>
                Object.keys(yaml).map((key) => ({
                    kind: EMonitoringConfigKind.DASHBOARD,
                    key,
                    revision: 1,
                })),
            getConfig: async ({kind, key}: {kind: EMonitoringConfigKind; key: string}) => ({
                kind,
                key,
                yaml: yaml[key],
                revision: 1,
            }),
        } as unknown as IMonitoringEditorApi
        const draft = {
            id: "turnout",
            theme: "default",
            layout: [{widget: "a"}, {widget: "e"}, {widget: "f"}],
        }
        expect(await countThemeWidgets(api, "default", "turnout", draft)).toBe(5)
        expect(await countThemeWidgets(api, "dark", "turnout", draft)).toBe(1)
    })
})
