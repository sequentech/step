// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type ComponentProps} from "react"
import {ResourceContextProvider, ResourceDefinitionContextProvider} from "react-admin"
import {AdminStoryProvider} from "./AdminStoryProvider"

type ResourceScreenProps = ComponentProps<typeof AdminStoryProvider> & {
    /** The react-admin resource whose route renders the screen. */
    resource: string
    /** The resource's `options.label` in App.tsx. */
    label: string
}

/**
 * A react-admin resource screen as its route renders it: the resource is
 * registered with list, create, edit and show views, as in App.tsx, so that
 * redirects and links resolve to the resource's own paths.
 */
export function ResourceScreen({resource, label, children, ...provider}: ResourceScreenProps) {
    return (
        <AdminStoryProvider {...provider}>
            <ResourceDefinitionContextProvider
                definitions={{
                    [resource]: {
                        name: resource,
                        hasList: true,
                        hasEdit: true,
                        hasCreate: true,
                        hasShow: true,
                        options: {label},
                    },
                }}
            >
                <ResourceContextProvider value={resource}>{children}</ResourceContextProvider>
            </ResourceDefinitionContextProvider>
        </AdminStoryProvider>
    )
}
