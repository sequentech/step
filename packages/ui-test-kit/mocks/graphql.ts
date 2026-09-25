// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {readFileSync} from "node:fs"
import {
    buildClientSchema,
    execute,
    getOperationAST,
    getVariableValues,
    parse,
    validate,
    type DocumentNode,
    type GraphQLFormattedError,
    type GraphQLSchema,
    type IntrospectionQuery,
    type OperationDefinitionNode,
} from "graphql"
import {json, type MockAbortReason, type MockRequest, type MockResponse} from "./http"
import {ViolationLog} from "./violations"

export interface GraphQLCall {
    operationName: string
    query: string
    variables: Record<string, unknown>
    /** Header names are lower case. */
    headers: Record<string, string>
}

export type GraphQLReply =
    /** Executed against the schema, so it is trimmed to the selection set and type checked. */
    | {data: Record<string, unknown>}
    /** Sent unchanged, as Hasura sends action and permission errors. */
    | {errors: GraphQLFormattedError[]; data?: Record<string, unknown> | null}
    /** A transport-level response, for example a gateway error. */
    | {status: number; body?: string; contentType?: string}
    | {abort: MockAbortReason}

export type GraphQLHandler = (call: GraphQLCall) => GraphQLReply | Promise<GraphQLReply>

const schemas = new Map<string, GraphQLSchema>()

/** Builds (once per process) the schema from a portal's `graphql.schema.json`. */
export function loadClientSchema(introspectionPath: string): GraphQLSchema {
    const cached = schemas.get(introspectionPath)
    if (cached) {
        return cached
    }
    const introspection = JSON.parse(readFileSync(introspectionPath, "utf8")) as IntrospectionQuery
    const schema = buildClientSchema(introspection)
    schemas.set(introspectionPath, schema)
    return schema
}

export interface GraphQLMockOptions {
    schema: GraphQLSchema
    violations: ViolationLog
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

/**
 * A GraphQL endpoint that accepts only operations valid against the portal's
 * schema, dispatches them by operation name and records every call.
 */
export class GraphQLMock {
    readonly calls: GraphQLCall[] = []
    private readonly schema: GraphQLSchema
    private readonly violations: ViolationLog
    private readonly handlers = new Map<string, GraphQLHandler>()
    private readonly queued = new Map<string, GraphQLHandler[]>()

    constructor({schema, violations}: GraphQLMockOptions) {
        this.schema = schema
        this.violations = violations
    }

    /** Handles every later call to `operationName`, replacing an earlier handler. */
    on(operationName: string, handler: GraphQLHandler): this {
        this.handlers.set(operationName, handler)
        return this
    }

    /** Handles only the next call to `operationName`, ahead of `on` handlers. */
    once(operationName: string, handler: GraphQLHandler): this {
        const queue = this.queued.get(operationName) ?? []
        queue.push(handler)
        this.queued.set(operationName, queue)
        return this
    }

    callsTo(operationName: string): GraphQLCall[] {
        return this.calls.filter((call) => call.operationName === operationName)
    }

    async handle(request: MockRequest): Promise<MockResponse> {
        if (request.method !== "POST") {
            this.violations.add(`GraphQL ${request.method} request: ${request.url}`)
            return json(405, {errors: [{message: "Only POST is supported"}]})
        }
        let body: unknown
        try {
            body = JSON.parse(request.body ?? "")
        } catch {
            this.violations.add("GraphQL request body is not JSON")
            return json(400, {errors: [{message: "Invalid JSON body"}]})
        }
        if (!isRecord(body) || typeof body.query !== "string") {
            this.violations.add("GraphQL request without a query")
            return json(400, {errors: [{message: "Missing query"}]})
        }
        let operationName = typeof body.operationName === "string" ? body.operationName : ""
        const variables = isRecord(body.variables) ? body.variables : {}
        let document: DocumentNode
        try {
            document = parse(body.query)
        } catch (error) {
            this.violations.add(`GraphQL syntax error in ${operationName}: ${String(error)}`)
            return json(200, {errors: [{message: String(error)}]})
        }
        const operation = getOperationAST(document, operationName || undefined)
        // GraphQL permits omitting operationName when the document has one operation.
        operationName ||= operation?.name?.value ?? ""
        const problems = [
            ...validate(this.schema, document).map((error) => error.message),
            ...this.checkOperation(operation, operationName, variables),
        ]
        if (problems.length > 0) {
            this.violations.add(
                `Invalid GraphQL operation ${operationName}: ${problems.join("; ")}`
            )
            return json(200, {errors: problems.map((message) => ({message}))})
        }

        const call: GraphQLCall = {
            operationName,
            query: body.query,
            variables,
            headers: request.headers,
        }
        this.calls.push(call)
        const handler = this.queued.get(operationName)?.shift() ?? this.handlers.get(operationName)
        if (!handler) {
            this.violations.add(`Unexpected GraphQL operation ${operationName}`)
            return json(200, {errors: [{message: `No mock for ${operationName}`}]})
        }
        const reply = await handler(call)
        if ("abort" in reply) {
            return {abort: reply.abort}
        }
        if ("status" in reply) {
            return {
                status: reply.status,
                headers: {"content-type": reply.contentType ?? "text/plain"},
                body: reply.body ?? "",
            }
        }
        if ("errors" in reply) {
            return json(200, reply)
        }
        const result = await execute({
            schema: this.schema,
            document,
            operationName: operationName || undefined,
            rootValue: reply.data,
            variableValues: variables,
        })
        if (result.errors?.length) {
            this.violations.add(
                `Mock data for ${operationName} does not match the schema: ${result.errors
                    .map((error) => error.message)
                    .join("; ")}`
            )
        }
        return json(200, result)
    }

    private checkOperation(
        operation: OperationDefinitionNode | null | undefined,
        operationName: string,
        variables: Record<string, unknown>
    ): string[] {
        if (!operation) {
            return [`the document has no operation named "${operationName}"`]
        }
        if ((operation.name?.value ?? "") !== operationName) {
            return [`operationName "${operationName}" does not name the operation`]
        }
        const coerced = getVariableValues(
            this.schema,
            operation.variableDefinitions ?? [],
            variables
        )
        return coerced.errors?.map((error) => error.message) ?? []
    }
}
