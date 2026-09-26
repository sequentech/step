// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The tenant lookups of the tenant selection screen, answered as Hasura and Keycloak do.
import type {MockRequest} from "@sequentech/ui-test-kit/mocks/http"
import {json} from "@sequentech/ui-test-kit/mocks/http"
import {TENANT_ID, STORY_SETTINGS} from "@/__stories__/AdminStoryProvider"

export const TENANT_SLUG = "council"
export const HASURA_URL = String(STORY_SETTINGS.HASURA_URL)
export const REALM_CONFIGURATION_URL = `${String(STORY_SETTINGS.KEYCLOAK_URL)}realms/tenant-${TENANT_ID}/.well-known/openid-configuration`

/** The default tenant's look and feel. */
export const DEFAULT_TENANT_CSS = "h1 { letter-spacing: 3px; }"

interface TenantQuery {
    variables: {id?: string; slug?: string}
}

/** Hasura's answer to the screen's two tenant queries; `found` is whether the slug exists. */
export function tenantLookup(found: boolean) {
    return (request: MockRequest) => {
        const {variables} = JSON.parse(request.body ?? "{}") as TenantQuery
        if (variables.id) {
            return json(200, {
                data: {
                    sequent_backend_tenant: [
                        {id: variables.id, slug: "default", annotations: {css: DEFAULT_TENANT_CSS}},
                    ],
                },
            })
        }
        return json(200, {
            data: {
                sequent_backend_tenant:
                    found && variables.slug === TENANT_SLUG
                        ? [{id: TENANT_ID, slug: TENANT_SLUG}]
                        : [],
            },
        })
    }
}

/** Keycloak's discovery document, which only an existing realm serves. */
export const realmConfiguration = (exists: boolean) => () =>
    exists ? json(200, {issuer: `tenant-${TENANT_ID}`}) : json(404, {error: "Realm does not exist"})
