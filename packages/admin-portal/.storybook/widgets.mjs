// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// @ts-check
import {humanize} from "../../test-support/storybook/widgetInventory.mjs"

const PROVIDER =
    "Context provider: holds shared state for the widgets below it; stories supply its " +
    "value directly, and the production journeys exercise the provider itself."
const SESSION =
    "Owns a real service session; AdminStoryProvider supplies the signed-in user, Apollo " +
    "client or settings instead, and the production journeys exercise this provider."
const POLLER =
    "Renders nothing: polls a created record and reports it to its parent, whose stories " +
    "cover the outcome."

/**
 * The admin widget catalog: `yarn stories:inventory` maps every authored
 * component under `src` to its section, `Admin/<Feature>/<Component>`.
 * @type {import("../../test-support/storybook/widgetInventory.mjs").WidgetConfig}
 */
export default {
    sources: "src",
    title: "Admin",
    // Complete screen stories that are also the section of their component.
    screenTitles: ["Screens/Admin/"],
    storybookUrl: "http://localhost:6008",
    // The feature is the directory path under src/resources or src/components,
    // e.g. "Dashboard/Election event"; shared components directly in
    // src/components form "Components" and src/screens "Screens".
    feature: (file) => {
        const [, area, ...rest] = file.split("/")
        const directories = rest.slice(0, -1)
        if ((area === "resources" || area === "components") && directories.length) {
            return directories.map(humanize).join("/")
        }
        return humanize(area.replace(/\.tsx?$/, ""))
    },
    excluded: {
        "src/App.tsx#App":
            "Application root: builds the Hasura data provider and registers the routes; " +
            "the production journeys' route matrix renders every route.",
        "src/components/CustomLayout.tsx#CustomCssReader":
            "Renders nothing: copies the tenant's custom CSS and languages into shared state; " +
            "the tenant global applies the same CSS in every story.",
        "src/components/election-event/create/CreateScreen.tsx#PullChecker": POLLER,
        "src/resources/ElectionEvent/CreateElectionEvent.tsx#PullChecker": POLLER,
        "src/resources/Approvals/ListApprovals.tsx#CustomFilters":
            "Not a component: ListApprovals calls it as a function to build its filter " +
            "inputs, which the ListApprovals stories render.",
        "src/resources/Tally/ResultsDataLoader.tsx#ResultsDataLoader":
            "Renders nothing: loads the results of a tally into the tally context; the tally " +
            "result stories provide that context.",
        "src/providers/ApolloContextProvider.tsx#ApolloContextProvider": SESSION,
        "src/providers/ApolloContextProvider.tsx#ApolloWrapper": SESSION,
        "src/providers/ApolloContextProvider.tsx#CustomApolloContextProvider": SESSION,
        "src/providers/AuthContextProvider.tsx#AuthContextProvider": SESSION,
        "src/providers/SettingsContextProvider.tsx#SettingsContextProvider": SESSION,
        "src/providers/SettingsContextProvider.tsx#SettingsGate": SESSION,
        "src/providers/SettingsContextProvider.tsx#SettingsWrapper": SESSION,
        "src/providers/CandidateContextProvider.tsx#CandidateContextProvider": PROVIDER,
        "src/providers/ContestContextProvider.tsx#ContestContextProvider": PROVIDER,
        "src/providers/CreateElectionEventContextProvider.tsx#CreateElectionEventProvider":
            PROVIDER,
        "src/providers/CreateElectionEventContextProvider.tsx#PullChecker": POLLER,
        "src/providers/DatabaseProvider.tsx#DatabaseProvider": PROVIDER,
        "src/providers/ElectionContextProvider.tsx#ElectionContextProvider": PROVIDER,
        "src/providers/ElectionEventContextProvider.tsx#ElectionEventContextProvider": PROVIDER,
        "src/providers/ElectionEventTallyProvider.tsx#ElectionEventTallyContextProvider": PROVIDER,
        "src/providers/IvrEmulatorContextProvider.tsx#IvrEmulatorContextProvider": PROVIDER,
        "src/providers/NewResourceProvider.tsx#NewResourceContextProvider": PROVIDER,
        "src/providers/PublishContextProvider.tsx#PublishContextProvider": PROVIDER,
        "src/providers/TenantContextProvider.tsx#TenantContextProvider": PROVIDER,
        "src/providers/WidgetsContextProvider.tsx#WidgetsContextProvider":
            PROVIDER + " Its task widgets have their own section, Admin/Components/WidgetsStack.",
    },
}
