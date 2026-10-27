// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    DocumentNode,
    Kind,
    SelectionSetNode,
    ValueNode,
    parseValue,
    valueFromASTUntyped,
    visit,
} from "graphql"

const settingsColumns = new Set([
    "id",
    "tenant_id",
    "election_event_id",
    "is_archived",
    "administration_settings",
    "presentation",
])
const settingsFields = new Set([
    ...Array.from(settingsColumns),
    "__typename",
    "aggregate",
    "count",
    "nodes",
    "returning",
    "affected_rows",
])
const settingsRoot =
    /^(?:(?:insert|update|delete)_)?sequent_backend_(?:election_event|election)(?:_by_pk|_aggregate)?$/
const noRows = parseValue("{id:{_is_null:true}}")
const defaultOrder = parseValue("{id:asc}")

function jsonValue(value: unknown): ValueNode {
    if (value === null || value === undefined) return {kind: Kind.NULL}
    if (Array.isArray(value)) return {kind: Kind.LIST, values: value.map(jsonValue)}
    if (typeof value === "object")
        return {
            kind: Kind.OBJECT,
            fields: Object.entries(value).map(([name, entry]) => ({
                kind: Kind.OBJECT_FIELD,
                name: {kind: Kind.NAME, value: name},
                value: jsonValue(entry),
            })),
        }
    if (typeof value === "boolean") return {kind: Kind.BOOLEAN, value}
    if (typeof value === "number")
        return {kind: Number.isInteger(value) ? Kind.INT : Kind.FLOAT, value: String(value)}
    return {kind: Kind.STRING, value: String(value)}
}

/** Keep filtering inside the public settings projection. An inaccessible
 * condition matches no rows instead of being dropped and widening a search. */
function projectInput(
    value: ValueNode,
    argument: string,
    variables: Record<string, unknown>
): ValueNode {
    if (value.kind === Kind.VARIABLE) {
        const name = value.name.value
        if (name in variables)
            variables[name] = valueFromASTUntyped(
                projectInput(jsonValue(variables[name]), argument, {})
            )
        return value
    }
    if (value.kind === Kind.LIST)
        return {
            ...value,
            values: value.values.map((entry) => projectInput(entry, argument, variables)),
        }
    if (value.kind !== Kind.OBJECT) return value
    const fields = []
    for (const field of value.fields) {
        const name = field.name.value
        if (argument === "where" && ["_and", "_or", "_not"].includes(name)) {
            fields.push({...field, value: projectInput(field.value, argument, variables)})
        } else if (settingsColumns.has(name)) {
            fields.push({
                ...field,
                name: {
                    ...field.name,
                    value: name === "presentation" ? "administration_settings" : name,
                },
            })
        } else if (argument === "where") {
            return noRows
        }
    }
    return fields.length || argument === "where" ? {...value, fields} : defaultOrder
}

function inlineFragments(document: DocumentNode): DocumentNode {
    const fragments = new Map(
        document.definitions.flatMap((definition) =>
            definition.kind === Kind.FRAGMENT_DEFINITION
                ? [[definition.name.value, definition] as const]
                : []
        )
    )
    const expand = (set: SelectionSetNode, stack = new Set<string>()): SelectionSetNode => ({
        ...set,
        selections: set.selections.map((selection) => {
            if (selection.kind === Kind.FRAGMENT_SPREAD) {
                const name = selection.name.value
                const fragment = fragments.get(name)
                if (!fragment || stack.has(name)) throw new Error("Invalid settings query fragment")
                return {
                    kind: Kind.INLINE_FRAGMENT,
                    typeCondition: fragment.typeCondition,
                    directives: selection.directives,
                    selectionSet: expand(
                        fragment.selectionSet,
                        new Set([...Array.from(stack), name])
                    ),
                }
            }
            return selection.selectionSet
                ? {...selection, selectionSet: expand(selection.selectionSet, stack)}
                : selection
        }),
    })
    return {
        ...document,
        definitions: document.definitions
            .filter((definition) => definition.kind !== Kind.FRAGMENT_DEFINITION)
            .map((definition) =>
                definition.kind === Kind.OPERATION_DEFINITION
                    ? {...definition, selectionSet: expand(definition.selectionSet)}
                    : definition
            ),
    }
}

/** Administrators receive a server-generated security-settings projection.
 * Aliasing it keeps existing settings forms usable without requesting election
 * data that their Hasura role cannot read. The database owns the access check.
 * Variables are updated together with their arguments before Apollo sends them. */
export function administratorSettingsQuery(
    document: DocumentNode,
    variables: Record<string, unknown> = {}
): DocumentNode {
    let hasSettings = false
    visit(document, {
        Field(node) {
            if (settingsRoot.test(node.name.value)) hasSettings = true
        },
    })
    if (!hasSettings) return document
    const projected = visit(inlineFragments(document), {
        Field(node, _key, _parent, _path, ancestors) {
            if (settingsRoot.test(node.name.value))
                return {
                    ...node,
                    arguments: node.arguments?.map((argument) =>
                        ["where", "order_by"].includes(argument.name.value)
                            ? {
                                  ...argument,
                                  value: projectInput(
                                      argument.value,
                                      argument.name.value,
                                      variables
                                  ),
                              }
                            : argument
                    ),
                }
            const inSettings = ancestors.some(
                (ancestor) =>
                    "kind" in ancestor &&
                    ancestor.kind === Kind.FIELD &&
                    settingsRoot.test(ancestor.name.value)
            )
            if (!inSettings) return
            if (!settingsFields.has(node.name.value)) return null
            if (node.name.value === "presentation" || node.name.value === "administration_settings")
                return {
                    ...node,
                    alias: node.alias ?? {kind: Kind.NAME, value: "presentation"},
                    name: {kind: Kind.NAME, value: "administration_settings"},
                }
        },
        InlineFragment: {
            leave(node) {
                if (!node.selectionSet.selections.length) return null
            },
        },
    })
    return visit(projected, {
        OperationDefinition(node) {
            const used = new Set<string>()
            visit(node.selectionSet, {
                Variable(variable) {
                    used.add(variable.name.value)
                },
            })
            node.directives?.forEach((directive) =>
                visit(directive, {
                    Variable(variable) {
                        used.add(variable.name.value)
                    },
                })
            )
            return {
                ...node,
                variableDefinitions: node.variableDefinitions?.filter((definition) =>
                    used.has(definition.variable.name.value)
                ),
            }
        },
    })
}
