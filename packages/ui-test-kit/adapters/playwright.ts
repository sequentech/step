// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {BrowserContext, Route} from "@playwright/test"
import {GraphQLMock} from "../mocks/graphql"
import {OidcMock} from "../mocks/oidc"
import {S3Mock} from "../mocks/s3"
import {ViolationLog} from "../mocks/violations"
import {isAbort, json, MOCK_PATHS, type MockResponse} from "../mocks/http"

export interface PortalServices {
    origin: string
    settings: Record<string, unknown>
    graphql: GraphQLMock
    oidc: OidcMock
    s3: S3Mock
    violations: ViolationLog
}

async function fulfill(route: Route, response: MockResponse) {
    if (isAbort(response)) await route.abort(response.abort)
    else
        await route.fulfill({
            ...response,
            body: response.body instanceof Uint8Array ? Buffer.from(response.body) : response.body,
        })
}

/** A strict boundary: only registered services, SPA navigations and static assets leave the page. */
export async function routePortal(context: BrowserContext, services: PortalServices) {
    const handler = async (route: Route) => {
        const request = route.request()
        const url = new URL(request.url())
        const mockRequest = {
            method: request.method(),
            url,
            headers: await request.allHeaders(),
            body: request.postData() ?? undefined,
        }
        try {
            if (url.origin !== services.origin) {
                services.violations.add(`Unexpected external request: ${request.method()} ${url}`)
                await route.abort("blockedbyclient")
            } else if (url.pathname === "/global-settings.json" && request.method() === "GET") {
                await fulfill(route, json(200, services.settings))
            } else if (url.pathname === MOCK_PATHS.graphql) {
                await fulfill(route, await services.graphql.handle(mockRequest))
            } else if (services.oidc.handles(url)) {
                await fulfill(route, services.oidc.handle(mockRequest))
            } else if (services.s3.handles(url)) {
                await fulfill(route, services.s3.handle(mockRequest))
            } else if (
                request.method() === "GET" &&
                (request.isNavigationRequest() ||
                    ["script", "stylesheet", "image", "font"].includes(request.resourceType()) ||
                    /\.wasm$/.test(url.pathname))
            ) {
                await route.continue()
            } else {
                services.violations.add(`Unexpected request: ${request.method()} ${url}`)
                await route.abort("blockedbyclient")
            }
        } catch (error) {
            services.violations.add(`Mock failed for ${url}: ${String(error)}`)
            await route.abort("failed").catch(() => {})
        }
    }
    await context.route("**/*", handler)
    return async () => context.unroute("**/*", handler)
}
