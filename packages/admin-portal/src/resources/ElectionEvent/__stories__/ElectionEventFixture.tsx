// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Boundaries of the election event sections whose widgets hand over to the
// widgets of other features, which have sections of their own: the reads of
// those widgets stay loading, and the stories assert which reads they issued.
import type {FetchResult, Operation} from "@apollo/client"
import type {DataProvider, RaRecord} from "react-admin"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {pending} from "../../../../../ui-essentials/.storybook/screens"

type Handler = (operation: Operation) => FetchResult | Promise<FetchResult>

/** Answers the named operations; every other operation stays loading. */
export function answerOrPending(handlers: Record<string, Handler> = {}): Record<string, Handler> {
    return new Proxy(handlers, {
        get: (target, name) =>
            typeof name === "string" && name in target
                ? target[name]
                : () => pending<FetchResult>(),
    })
}

const READS = ["getList", "getOne", "getMany", "getManyReference"] as const

/**
 * A data provider that answers the listed resources from their records, with
 * the filters and writes of `resourceBoundary`; reads of any other resource
 * never settle. `calls` records every call.
 */
export function recordsOrPending(records: Record<string, RaRecord[]> = {}) {
    const answered = resourceBoundary(records)
    const handlers: Partial<DataProvider> = Object.fromEntries(
        READS.map((method) => [
            method,
            (resource: string, params: unknown) =>
                resource in records
                    ? Reflect.apply(answered.provider[method], answered.provider, [
                          resource,
                          params,
                      ])
                    : pending(),
        ])
    )
    for (const method of ["create", "update", "updateMany", "delete", "deleteMany"] as const) {
        handlers[method] = (resource: string, params: never) =>
            Reflect.apply(answered.provider[method], answered.provider, [resource, params])
    }
    return {...dataBoundary(handlers), writes: answered.writes}
}

/** "method resource" of each data provider call, e.g. "getList sequent_backend_area". */
export const readsOf = (boundary: ReturnType<typeof dataBoundary>) =>
    boundary.calls.map(({method, args}) => `${method} ${String(args[0])}`)

/** The parameters of the first call of a method on a resource. */
export const paramsOf = (
    boundary: ReturnType<typeof dataBoundary>,
    method: string,
    resource: string
) => boundary.calls.find((call) => call.method === method && call.args[0] === resource)?.args[1]
