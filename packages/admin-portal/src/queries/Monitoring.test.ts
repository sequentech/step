// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    buildClientSchema,
    print,
    validate,
    type DocumentNode,
    type FieldNode,
    type IntrospectionQuery,
    type OperationDefinitionNode,
} from "graphql"
import introspection from "../../graphql.schema.json"
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
const schema = buildClientSchema(introspection as unknown as IntrospectionQuery)

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
        ["election_event_id", "preset_id", "mode"],
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

    // Harvest's actions take ids as String: a uuid variable is refused by Hasura.
    it.each(CONTRACT)("declares no uuid variable in %#", (document) => {
        const types = operation(document).variableDefinitions?.map((variable) =>
            print(variable.type)
        )
        expect(types?.filter((type) => type.startsWith("uuid"))).toEqual([])
    })

    // graphql.schema.json carries Harvest's action types (actions.graphql), so a
    // field the action does not declare, or a scalar where it has an object, fails here.
    it.each(CONTRACT)("is valid against the action schema: %#", (document) => {
        expect(validate(schema, document).map((error) => error.message)).toEqual([])
    })
})
