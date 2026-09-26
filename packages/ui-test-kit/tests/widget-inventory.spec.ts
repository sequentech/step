// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect} from "@playwright/test"
import {mkdirSync, writeFileSync} from "node:fs"
import {dirname, join} from "node:path"
import {
    buildInventory,
    humanize,
    scanComponents,
    scanSections,
    type WidgetConfig,
} from "../../test-support/storybook/widgetInventory.mjs"

const config: WidgetConfig = {
    sources: "src",
    title: "Admin",
    screenTitles: ["Screens/Admin/"],
    storybookUrl: "http://localhost:6008",
    feature: (file) => humanize(file.split("/")[2]),
    excluded: {"src/resources/Panel/Panel.tsx#PanelProvider": "Context provider"},
}

/** A package under the test's output directory, so that TypeScript resolves from the workspace. */
function writePackage(directory: string, files: Record<string, string>) {
    mkdirSync(directory, {recursive: true})
    writeFileSync(join(directory, "package.json"), '{"name": "inventory-fixture"}')
    for (const [path, content] of Object.entries(files)) {
        mkdirSync(dirname(join(directory, path)), {recursive: true})
        writeFileSync(join(directory, path), content)
    }
    return directory
}

const SOURCES = {
    "src/resources/Panel/Panel.tsx": `
        import React, {createContext, memo} from "react"
        import {styled} from "@mui/material/styles"
        const Frame = styled("div")\`padding: 4px;\`
        const Context = createContext(0)
        export const PanelProvider = ({children}) => <Context.Provider value={1}>{children}</Context.Provider>
        const ExportDialog = () => <div role="dialog" />
        const PanelMemo = memo(() => <Frame><ExportDialog /></Frame>)
        PanelMemo.displayName = "Panel"
        export const Panel = PanelMemo
        export function PanelToolbar() { return <div /> }
        export const formatTitle = (title: string) => title.toUpperCase()
    `,
    "src/resources/Panel/ListPanel.tsx": `
        import React from "react"
        export default function ListPanel() { return <ul /> }
        export class LegacyPanel extends React.Component { render() { return <div /> } }
    `,
    "src/resources/Panel/Panel.test.tsx": "export const Ignored = () => <div />",
    "src/resources/Panel/__stories__/PanelFixture.tsx": "export const Fixture = () => <div />",
}

const STORIES = {
    "src/resources/Panel/Panel.stories.tsx": `
        import {Panel} from "./Panel"
        const meta = {title: "Admin/Panel/Panel", component: Panel}
        export default meta
        export const Populated = {}
        export const ExportDialogOpen = {parameters: {widgets: ["ExportDialog"]}}
    `,
    "src/resources/Panel/ListPanel.stories.tsx": `
        import ListPanel from "@/resources/Panel/ListPanel"
        export default {title: "Admin/Panels/ListPanel", component: ListPanel}
        export const Empty = {parameters: {widgets: ["MissingDialog"]}}
    `,
}

test.describe("admin widget inventory", () => {
    test("humanizes directory names into feature names", () => {
        expect(humanize("ElectionEvent")).toBe("Election event")
        expect(humanize("keys-ceremony")).toBe("Keys ceremony")
        expect(humanize("TallySheetImport")).toBe("Tally sheet import")
    })

    test("finds rendering bindings, their export aliases and styled primitives", () => {
        const directory = writePackage(test.info().outputPath("package"), SOURCES)
        const components = scanComponents(directory, "src").map(
            ({file, name, local, exportNames, kind}) => ({file, name, local, exportNames, kind})
        )
        expect(components).toEqual([
            // A default export keeps the name of its declaration.
            {
                file: "src/resources/Panel/ListPanel.tsx",
                name: "ListPanel",
                local: "ListPanel",
                exportNames: ["default"],
                kind: "component",
            },
            {
                file: "src/resources/Panel/ListPanel.tsx",
                name: "LegacyPanel",
                local: "LegacyPanel",
                exportNames: ["LegacyPanel"],
                kind: "component",
            },
            {
                file: "src/resources/Panel/Panel.tsx",
                name: "Frame",
                local: "Frame",
                exportNames: [],
                kind: "styled",
            },
            {
                file: "src/resources/Panel/Panel.tsx",
                name: "PanelProvider",
                local: "PanelProvider",
                exportNames: ["PanelProvider"],
                kind: "component",
            },
            {
                file: "src/resources/Panel/Panel.tsx",
                name: "ExportDialog",
                local: "ExportDialog",
                exportNames: [],
                kind: "component",
            },
            // The memo binding is exported through its alias, which names it.
            {
                file: "src/resources/Panel/Panel.tsx",
                name: "Panel",
                local: "PanelMemo",
                exportNames: ["Panel"],
                kind: "component",
            },
            {
                file: "src/resources/Panel/Panel.tsx",
                name: "PanelToolbar",
                local: "PanelToolbar",
                exportNames: ["PanelToolbar"],
                kind: "component",
            },
        ])
    })

    test("maps sections and embedded stories and reports every contract problem", () => {
        const directory = writePackage(test.info().outputPath("package"), {...SOURCES, ...STORIES})
        const {widgets, problems} = buildInventory(
            {...config, excluded: {...config.excluded, "src/resources/Gone.tsx#Gone": "Stale"}},
            scanComponents(directory, "src"),
            scanSections(directory, "src")
        )
        const status = Object.fromEntries(
            widgets.map(({component, status, sections, stories}) => [
                `${component.file.split("/").pop()}#${component.name}`,
                {
                    status,
                    sections: sections.map(({id}) => id),
                    stories: stories.map(({id}) => id),
                },
            ])
        )
        expect(status).toEqual({
            "ListPanel.tsx#ListPanel": {
                status: "covered",
                sections: ["admin-panels-listpanel"],
                stories: [],
            },
            "ListPanel.tsx#LegacyPanel": {status: "remaining", sections: [], stories: []},
            "Panel.tsx#Frame": {status: "excluded", sections: [], stories: []},
            "Panel.tsx#PanelProvider": {status: "excluded", sections: [], stories: []},
            "Panel.tsx#ExportDialog": {
                status: "covered",
                sections: [],
                stories: ["admin-panel-panel--export-dialog-open"],
            },
            "Panel.tsx#Panel": {status: "covered", sections: ["admin-panel-panel"], stories: []},
            "Panel.tsx#PanelToolbar": {status: "remaining", sections: [], stories: []},
        })
        expect(problems).toEqual([
            "Exclusion src/resources/Gone.tsx#Gone names no component",
            'src/resources/Panel/ListPanel.stories.tsx: title "Admin/Panels/ListPanel" should be "Admin/Panel/ListPanel"',
            "src/resources/Panel/ListPanel.stories.tsx: story admin-panels-listpanel--empty shows MissingDialog, which src/resources/Panel/ListPanel.tsx does not define",
        ])
    })

    test("rejects a section whose component is a story fixture", () => {
        const directory = writePackage(test.info().outputPath("package"), {
            ...SOURCES,
            "src/resources/Panel/Panel.stories.tsx": `
                import {Fixture} from "./__stories__/PanelFixture"
                export default {title: "Admin/Panel/Panel", component: Fixture}
                export const Populated = {}
            `,
        })
        const {widgets, problems} = buildInventory(
            config,
            scanComponents(directory, "src"),
            scanSections(directory, "src")
        )
        expect(problems).toEqual([
            "src/resources/Panel/Panel.stories.tsx: meta.component Fixture is not an authored component of src",
        ])
        expect(widgets.find(({component}) => component.name === "Panel")?.status).toBe("remaining")
    })
})
