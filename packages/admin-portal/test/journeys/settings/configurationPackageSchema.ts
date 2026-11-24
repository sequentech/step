// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {readFileSync} from "node:fs"
import {
    extendSchema,
    GraphQLObjectType,
    GraphQLSchema,
    isInputType,
    Kind,
    parse,
    typeFromAST,
    type ObjectTypeDefinitionNode,
} from "graphql"

/** The older introspection snapshot predates these actions. Extend only the
 * fixture schema from Hasura's checked-in definition rather than bypassing
 * validation. The upload argument is shared by all journeys; only package
 * journeys require the additional settings actions and types. */
export function withConfigurationPackageActions(
    baseline: GraphQLSchema,
    actionPath: string,
    includePackageActions: boolean
): GraphQLSchema {
    const {definitions} = parse(readFileSync(actionPath, "utf8"))
    const fields = (kind: string) =>
        definitions
            .filter(
                (node): node is ObjectTypeDefinitionNode =>
                    node.kind === Kind.OBJECT_TYPE_DEFINITION && node.name.value === kind
            )
            .flatMap((node) => node.fields ?? [])
    const only = (kind: string, names: readonly string[]) => {
        const selected = fields(kind).filter((field) => names.includes(field.name.value))
        if (selected.length !== names.length) throw new Error("Missing package action definition")
        return selected
    }
    const query = baseline.getQueryType()
    const mutation = baseline.getMutationType()
    if (!query || !mutation) throw new Error("Missing journey schema roots")
    const additional = [
        "ApplyConfigurationPackageOutput",
        "ArchitectApprovalPolicyOutput",
        "ArchitectSignerRosterMode",
        "ArchitectSignerRosterStatus",
    ]
    const types = definitions.filter(
        (node) =>
            (node.kind === Kind.OBJECT_TYPE_DEFINITION ||
                node.kind === Kind.ENUM_TYPE_DEFINITION) &&
            additional.includes(node.name.value) &&
            !baseline.getType(node.name.value)
    )
    const queries = only("Query", [
        "get_architect_signer_roster_status",
        "get_architect_approval_policy",
    ]).filter((field) => !query.getFields()[field.name.value])
    const mutations = only("Mutation", ["apply_configuration_package"]).filter(
        (field) => !mutation.getFields()[field.name.value]
    )
    // ExtendSchema returns a separate type graph; the baseline is cached by
    // the normal fixture, so never mutate a field on the original schema.
    const schema = includePackageActions
        ? extendSchema(baseline, {
              kind: Kind.DOCUMENT,
              definitions: [
                  ...types,
                  ...(queries.length
                      ? [
                            {
                                kind: Kind.OBJECT_TYPE_EXTENSION as const,
                                name: {kind: Kind.NAME as const, value: query.name},
                                fields: queries,
                            },
                        ]
                      : []),
                  ...(mutations.length
                      ? [
                            {
                                kind: Kind.OBJECT_TYPE_EXTENSION as const,
                                name: {kind: Kind.NAME as const, value: mutation.name},
                                fields: mutations,
                            },
                        ]
                      : []),
              ],
          })
        : baseline
    const mutationRoot = schema.getMutationType()
    const existing = mutationRoot?.getFields().get_upload_url
    const argument = only("Mutation", ["get_upload_url"])[0].arguments?.find(
        (arg) => arg.name.value === "configuration_package"
    )
    const type = argument && typeFromAST(schema, argument.type)
    if (!mutationRoot || !existing || !type || !isInputType(type)) {
        throw new Error("Missing package upload argument")
    }
    if (existing.args.some((arg) => arg.name === "configuration_package")) return schema
    const root = mutationRoot.toConfig()
    const newRoot = new GraphQLObjectType({
        ...root,
        fields: {
            ...root.fields,
            get_upload_url: {
                ...root.fields.get_upload_url,
                args: {
                    ...root.fields.get_upload_url.args,
                    configuration_package: {type, astNode: argument},
                },
            },
        },
    })
    const config = schema.toConfig()
    return new GraphQLSchema({
        ...config,
        mutation: newRoot,
        types: config.types.filter((type) => type.name !== mutationRoot.name),
    })
}
