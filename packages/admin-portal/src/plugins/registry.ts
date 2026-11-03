// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {lazy} from "react"
import type {PluginContribution} from "./contracts"
/** Only release-bundled code can contribute a route; a server manifest is never an import URL. */
export const contributions: PluginContribution[] = [
    {
        id: "projects",
        apiMajor: 1,
        labelKey: "projects.title",
        route: "/plugins/projects",
        component: lazy(() => import("./projects/Projects")),
    },
]
