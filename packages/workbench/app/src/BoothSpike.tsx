// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Booth lift: mounts voting-portal route components from production
// source, unmodified, at the same paths the portal itself uses, with
// route-level `action`s wired exactly as in `voting-portal/src/index.tsx`.
// Data is supplied by `fixtures/boothFixtures.ts` via the portal's own
// action creators. See `packages/workbench/LIFTING.md` for the procedure.

import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {ApolloClient, ApolloLink, InMemoryCache} from "@apollo/client"
import {ApolloProvider} from "@apollo/client/react"
import type {RouteObject} from "react-router-dom"
import {Outlet} from "react-router-dom"

// Side-effectful imports: register translations.
import "voting-portal/src/services/i18n"

import {store} from "voting-portal/src/store/store"
import {WasmWrapper} from "voting-portal/src/providers/WasmWrapper"
import {SettingsWrapper} from "voting-portal/src/providers/SettingsContextProvider"
import ElectionSelectionScreen from "voting-portal/src/routes/ElectionSelectionScreen"
import StartScreen from "voting-portal/src/routes/StartScreen"
import VotingScreen, {
    action as votingAction,
} from "voting-portal/src/routes/VotingScreen"
import ReviewScreen, {
    action as castBallotAction,
} from "voting-portal/src/routes/ReviewScreen"
import ConfirmationScreen from "voting-portal/src/routes/ConfirmationScreen"
import {loadBundledSnapshot} from "./fixtures/bundledSnapshots"
import {
    bundledId,
    clearPersistedSnapshot,
    hydrateFromSnapshot,
    installPersistence,
    loadPersistedSnapshot,
} from "./persistence"
import {getPolicyOverrides} from "./policyOverridesStore"

// On boot, the workbench's state ALWAYS comes from a snapshot. The
// auto-resume slot (`workbench:state:v1`) wins if present \u2014 that is
// what gives us "close the tab, reopen, ballot is still cast". On a
// fresh checkout we fall back to the bundled `default` snapshot, which
// was captured from a previous run of the in-code fixture and now
// ships as JSON alongside the source. There is no longer a live
// "seed" code path: the snapshot is the unit of state, and a fresh
// boot is hydration-from-bundled, not regeneration.
//
// Bundled snapshots already carry the snapshot's keypair in
// `workbench.keypair`, so warm and cold boots both do zero wasm
// work on the boot path. Keypair generation now happens only when
// an operator explicitly creates a new scenario (out of scope for
// this boot).
//
// Boot remains async because hydrate-time decrypts of any persisted
// cast votes go through velvet-wasm. Top-level await is enabled by
// the Vite top-level-await plugin (see vite.config.ts).
const persisted = loadPersistedSnapshot()
if (persisted) {
    // Warm boot: recover provenance from the snapshot's own parentId.
    hydrateFromSnapshot(store, persisted)
} else {
    const bundled = loadBundledSnapshot("default")
    if (bundled) {
        // First boot: the working copy is a fork of `bundled:default`.
        hydrateFromSnapshot(store, bundled, bundledId("default"))
    } else {
        // Should not happen: the build pipeline guarantees a default
        // snapshot is bundled. Surface loudly rather than silently
        // booting into an empty store.
        console.error(
            "[workbench/boot] no persisted snapshot and no bundled default; " +
                "the workbench will boot into an empty store"
        )
    }
}
// Subscribe AFTER any boot dispatches so we never persist a partial
// in-progress hydration.
installPersistence(store)

// Workbench-only debug: expose the production store on `window.__store`
// so we can inspect Redux state from the browser console / Playwright
// while lifting screens. This is a workbench affordance only and lives
// outside `voting-portal/src/`.
if (typeof window !== "undefined") {
    ;(window as unknown as {__store: typeof store}).__store = store
    const w = window as unknown as {
        __dispatchLog: unknown[]
        __resetWorkbench: () => void
    }
    w.__dispatchLog = []
    // Patch dispatch to log every action; helps spot whether a click
    // actually reached the reducer or not.
    const origDispatch = store.dispatch.bind(store)
    ;(store as unknown as {dispatch: typeof origDispatch}).dispatch = ((
        action: Parameters<typeof origDispatch>[0]
    ) => {
        w.__dispatchLog.push({type: (action as {type?: string}).type, action})
        return origDispatch(action)
    }) as typeof origDispatch
    // Convenience: from the browser console, `__resetWorkbench()` wipes
    // the persisted snapshot and reloads. Same as deleting the
    // `workbench:state:v1` key in DevTools → Application → Local Storage.
    w.__resetWorkbench = () => {
        clearPersistedSnapshot()
        location.reload()
    }
    ;(w as unknown as {__getPolicyOverrides: typeof getPolicyOverrides}).__getPolicyOverrides =
        getPolicyOverrides
}

/**
 * Workbench Apollo client. We never want a live GraphQL endpoint here, so
 * the link is `ApolloLink.empty()` — observables complete with no data,
 * which yields `{ data: undefined, loading: true }` for any `useQuery`.
 * Production `ReviewScreen.GET_ELECTIONS` runs with `skip: DISABLE_AUTH`
 * so it never actually fires; `INSERT_CAST_VOTE` only fires when the user
 * clicks Cast — at which point we'll either replace this with a mocked
 * link or let it fail visibly at the network boundary.
 *
 * Why not `MockedProvider` from `@apollo/client/testing`: it requires
 * pre-declared mocks for every operation and throws on misses, which is
 * the wrong default for an exploratory workbench. The empty link defers
 * the decision to per-operation mocks added when each screen demands them.
 */
const apolloClient = new ApolloClient({
    link: ApolloLink.empty(),
    cache: new InMemoryCache(),
})

/**
 * Layout for every booth screen. Provides, in the exact order the portal
 * itself wires them in `voting-portal/src/index.tsx`:
 *   1. MUI ThemeProvider (theme from ui-essentials)
 *   2. SettingsContextProvider — fetches `/global-settings.json` (served
 *      from `public/`) so `globalSettings.DISABLE_AUTH` is `true` by the
 *      time screens read it. Without this, the default DISABLE_AUTH=false
 *      makes ReviewScreen's GET_ELECTIONS query fire against
 *      `http://localhost:8080/v1/graphql` and the auth-gated paths trip.
 *   3. ApolloProvider with the workbench's empty-link client. Required
 *      by any screen using `useQuery` / `useMutation` (e.g. ReviewScreen).
 *   4. WasmWrapper — initializes the sequent-core wasm module before
 *      rendering children. Required by every screen that calls into
 *      ui-core wasm helpers.
 *
 * The Redux Provider used to live inside this layout, but it has been
 * hoisted to `Shell` (in `main.tsx`) so the workbench's own pages (e.g.
 * the tally view) share the same store and see the cast votes the booth
 * produced. The portal's production index.tsx wraps Redux outside its
 * routes too, so this matches its layering.
 *
 * Designed to be mounted as a layout route under a data router so the
 * portal's `useSubmit` / `useActionData` calls work.
 */
export function BoothLayout() {
    return (
        <ThemeProvider theme={theme}>
            <SettingsWrapper>
                <ApolloProvider client={apolloClient}>
                    <WasmWrapper>
                        <Outlet />
                    </WasmWrapper>
                </ApolloProvider>
            </SettingsWrapper>
        </ThemeProvider>
    )
}

/**
 * Route children mirroring the portal's own route tree (see
 * `voting-portal/src/index.tsx`) at the same paths so its absolute
 * `<Link to="/tenant/.../vote">` navigations and `useSubmit` calls
 * resolve without any rewrite.
 *
 * As we lift more screens, add them here paired with their `action`
 * (when the portal wires one). Keep this list aligned with the portal's
 * `election/:electionId` subtree.
 */
export const boothChildren: RouteObject[] = [
    {
        // Mirrors the portal's `tenant/:tenantId/event/:eventId` subtree
        // (see voting-portal/src/index.tsx). Keeping `election-chooser`
        // and `election/:electionId/*` as siblings under the same parent
        // makes every `<Link to="/tenant/.../election-chooser">` and the
        // chooser's `navigate(\`/tenant/.../election/${id}/start\`)` call
        // resolve at the same paths the portal uses in production.
        path: "tenant/:tenantId/event/:eventId",
        children: [
            {path: "election-chooser", element: <ElectionSelectionScreen />},
            {
                path: "election/:electionId",
                children: [
                    {path: "start", element: <StartScreen />},
                    {path: "vote", element: <VotingScreen />, action: votingAction},
                    {
                        path: "review",
                        element: <ReviewScreen />,
                        action: castBallotAction,
                    },
                    {path: "confirmation", element: <ConfirmationScreen />},
                ],
            },
        ],
    },
]
