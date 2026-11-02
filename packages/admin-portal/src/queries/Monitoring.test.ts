// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "fs"
import {resolve} from "path"
import {
    buildClientSchema,
    validate,
    type DocumentNode,
    type FieldNode,
    type GraphQLSchema,
    type IntrospectionQuery,
    type OperationDefinitionNode,
} from "graphql"
import {MONITORING_EXPORT} from "./MonitoringExport"
import {MONITORING_GET_CONFIG} from "./MonitoringGetConfig"
import {MONITORING_GET_DASHBOARD} from "./MonitoringGetDashboard"
import {MONITORING_LIST_CONFIG} from "./MonitoringListConfig"
import {MONITORING_LIST_DASHBOARDS} from "./MonitoringListDashboards"
import {MONITORING_LIST_PRESETS} from "./MonitoringListPresets"
import {MONITORING_RENDER_WIDGET} from "./MonitoringRenderWidget"
import {MONITORING_RESET_TO_PRESET} from "./MonitoringResetToPreset"
import {MONITORING_SAVE_CONFIG} from "./MonitoringSaveConfig"
import {MONITORING_SET_MODE} from "./MonitoringSetMode"
import {MONITORING_VALIDATE_CONFIG} from "./MonitoringValidateConfig"

const operation = (document: DocumentNode) =>
    document.definitions.find(
        (definition): definition is OperationDefinitionNode =>
            definition.kind === "OperationDefinition"
    )!

// Each action takes the Harvest route's snake_case body fields as arguments.
const CONTRACT: Array<[DocumentNode, string, "query" | "mutation", string[]]> = [
    [
        MONITORING_LIST_DASHBOARDS,
        "monitoringListDashboards",
        "query",
        ["election_event_id", "election_id"],
    ],
    [
        MONITORING_GET_DASHBOARD,
        "monitoringGetDashboard",
        "query",
        ["election_event_id", "election_id", "dashboard_id"],
    ],
    [
        MONITORING_RENDER_WIDGET,
        "monitoringRenderWidget",
        "query",
        [
            "election_event_id",
            "election_id",
            "dashboard_id",
            "widget_id",
            "scope",
            "selector_values",
            "snapshot_revision",
            "width",
            "color_scheme",
            "locale",
            "draft",
        ],
    ],
    [
        MONITORING_EXPORT,
        "monitoringExport",
        "mutation",
        [
            "election_event_id",
            "election_id",
            "dashboard_id",
            "widget_id",
            "scope",
            "selector_values",
            "widget_selector_values",
            "snapshot_revision",
            "format",
            "from",
            "to",
        ],
    ],
    [
        MONITORING_VALIDATE_CONFIG,
        "monitoringValidateConfig",
        "query",
        ["election_event_id", "kind", "key", "yaml"],
    ],
    [
        MONITORING_SAVE_CONFIG,
        "monitoringSaveConfig",
        "mutation",
        ["election_event_id", "kind", "key", "yaml", "expected_revision", "change"],
    ],
    [
        MONITORING_RESET_TO_PRESET,
        "monitoringResetToPreset",
        "mutation",
        ["election_event_id", "preset_id"],
    ],
    [MONITORING_LIST_PRESETS, "monitoringListPresets", "query", ["election_event_id"]],
    [MONITORING_SET_MODE, "monitoringSetMode", "mutation", ["election_event_id", "mode"]],
    [MONITORING_LIST_CONFIG, "monitoringListConfig", "query", ["election_event_id"]],
    [
        MONITORING_GET_CONFIG,
        "monitoringGetConfig",
        "query",
        ["election_event_id", "kind", "key", "revision", "before_revision", "limit"],
    ],
]

describe("monitoring operations", () => {
    it.each(CONTRACT)(
        "calls %#: the action with the route's fields",
        (document, action, kind, args) => {
            const definition = operation(document)
            expect(definition.operation).toBe(kind)
            expect(definition.name?.value).toBe(action.charAt(0).toUpperCase() + action.slice(1))
            const field = definition.selectionSet.selections[0] as FieldNode
            expect(field.name.value).toBe(action)
            expect(field.arguments?.map((argument) => argument.name.value)).toEqual(args)
            const declared = definition.variableDefinitions?.map(
                (variable) => variable.variable.name.value
            )
            expect(declared).toHaveLength(args.length)
        }
    )
})

describe("monitoring operations against the admin schema", () => {
    let schema: GraphQLSchema
    beforeAll(() => {
        // The introspection file codegen writes from Hasura, which holds Harvest's actions.
        const json = JSON.parse(
            readFileSync(resolve(__dirname, "../../graphql.schema.json"), "utf8")
        ) as IntrospectionQuery | {data: IntrospectionQuery}
        schema = buildClientSchema("data" in json ? json.data : json)
    })

    it.each(CONTRACT)(
        "validates %#: fields exist and variable types fit the arguments",
        (document) => {
            expect(validate(schema, document).map(({message}) => message)).toEqual([])
        }
    )

    const selected = (document: DocumentNode) =>
        (operation(document).selectionSet.selections[0] as FieldNode).selectionSet!.selections.map(
            (selection) => (selection as FieldNode).name.value
        )

    it("reads what the view needs of Harvest's additions", () => {
        expect(selected(MONITORING_GET_DASHBOARD)).toContain("event_days")
        expect(selected(MONITORING_SET_MODE)).toEqual(["mode", "generation"])
        // Every governed query's rows, in widget order, next to the first one's `table`.
        const render = operation(MONITORING_RENDER_WIDGET).selectionSet.selections[0] as FieldNode
        const tables = render.selectionSet!.selections.find(
            (selection) => (selection as FieldNode).name.value === "tables"
        ) as FieldNode | undefined
        expect(
            tables?.selectionSet?.selections.map((field) => (field as FieldNode).name.value)
        ).toEqual(["query", "table"])
        expect(selected(MONITORING_RENDER_WIDGET)).toContain("table")
    })
})
