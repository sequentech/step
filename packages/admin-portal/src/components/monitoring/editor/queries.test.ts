// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {print, type DocumentNode, type OperationDefinitionNode} from "graphql"
import * as view from "@/queries/MonitoringGetConfig"
import * as viewSave from "@/queries/MonitoringSaveConfig"
import * as viewRender from "@/queries/MonitoringRenderWidget"
import * as viewValidate from "@/queries/MonitoringValidateConfig"
import * as viewListConfig from "@/queries/MonitoringListConfig"
import * as viewPresets from "@/queries/MonitoringListPresets"
import * as viewReset from "@/queries/MonitoringResetToPreset"
import * as editor from "./queries"

const operation = (document: DocumentNode) =>
    document.definitions.find(
        (definition): definition is OperationDefinitionNode =>
            definition.kind === "OperationDefinition"
    )!

/** Harvest's actions.graphql: reads are queries, writes mutations. */
const KINDS: Record<string, "query" | "mutation"> = {
    MONITORING_EDITOR_VALIDATE_CONFIG: "query",
    MONITORING_EDITOR_RENDER_WIDGET: "query",
    MONITORING_EDITOR_GET_CONFIG: "query",
    MONITORING_EDITOR_LIST_CONFIG: "query",
    MONITORING_EDITOR_LIST_PRESETS: "query",
    MONITORING_EDITOR_SAVE_CONFIG: "mutation",
    MONITORING_EDITOR_RESET_TO_PRESET: "mutation",
}

describe("the editor's monitoring operations", () => {
    it.each(Object.entries(editor))("%s is a %s as Harvest declares it", (name, document) => {
        expect(operation(document).operation).toBe(KINDS[name])
    })

    it.each(Object.entries(editor))("%s takes the event id as uuid", (_, document) => {
        const types = Object.fromEntries(
            (operation(document).variableDefinitions ?? []).map((variable) => [
                variable.variable.name.value,
                print(variable.type),
            ])
        )
        expect(types.election_event_id).toBe("uuid!")
        if ("election_id" in types) expect(types.election_id).toBe("uuid")
    })

    it("names each operation apart from the view's, as codegen requires", () => {
        const names = (documents: object) =>
            Object.values(documents).map((document) => operation(document).name?.value)
        const viewNames = new Set(
            [
                view,
                viewSave,
                viewRender,
                viewValidate,
                viewListConfig,
                viewPresets,
                viewReset,
            ].flatMap(names)
        )
        const editorNames = names(editor)
        expect(new Set(editorNames).size).toBe(editorNames.length)
        expect(editorNames.filter((name) => viewNames.has(name))).toEqual([])
    })
})
