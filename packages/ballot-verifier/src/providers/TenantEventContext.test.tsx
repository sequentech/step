// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {TenantEventContext, TenantEventProvider} from "./TenantEventContext"

const RouteIdentity = () => {
    const {tenantId, eventId} = useContext(TenantEventContext)
    return (
        <span>
            {tenantId ?? "no-tenant"}/{eventId ?? "no-event"}
        </span>
    )
}

it("can read the default context without mounting the application", () => {
    expect(renderToStaticMarkup(<RouteIdentity />)).toBe("<span>no-tenant/no-event</span>")
})
it("provides the tenant and election event from the route", () => {
    const markup = renderToStaticMarkup(
        <TenantEventProvider tenantId="tenant-a" eventId="event-a">
            <RouteIdentity />
        </TenantEventProvider>
    )
    expect(markup).toBe("<span>tenant-a/event-a</span>")
})
it("keeps separate provider instances isolated", () => {
    const markup = renderToStaticMarkup(
        <>
            <TenantEventProvider tenantId="a" eventId={null}>
                <RouteIdentity />
            </TenantEventProvider>
            <TenantEventProvider tenantId="b" eventId="event-b">
                <RouteIdentity />
            </TenantEventProvider>
        </>
    )
    expect(markup).toBe("<span>a/no-event</span><span>b/event-b</span>")
})
