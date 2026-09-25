// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"
import type {Decorator} from "@storybook/react-vite"
import {createMemoryRouter, Outlet, useLocation} from "react-router"
import {RouterProvider} from "react-router/dom"

/** `parameters.router` of a story. */
export interface RouterParameters {
    /** Route pattern that renders the story, e.g. "/tenant/:tenantId/event/:eventId". */
    path?: string
    /** Initial history stack; the last entry is the current location. */
    initialEntries?: string[]
}

// The router is created once per story, so the story element reaches its route
// through context and still picks up new args on every render.
const StoryElement = createContext<React.ReactNode>(null)

const StoryRoute: React.FC = () => <>{useContext(StoryElement)}</>

// Play functions read where a navigation went from this status element.
const CurrentLocation: React.FC = () => {
    const {pathname, search} = useLocation()
    return (
        <output
            aria-label="Current location"
            style={{
                position: "absolute",
                width: 1,
                height: 1,
                overflow: "hidden",
                clip: "rect(0 0 0 0)",
            }}
        >
            {pathname + search}
        </output>
    )
}

const MemoryRouterHost: React.FC<Required<RouterParameters> & {story: React.ReactNode}> = ({
    path,
    initialEntries,
    story,
}) => {
    const [router] = useState(() =>
        createMemoryRouter(
            [
                {
                    element: (
                        <>
                            <Outlet />
                            <CurrentLocation />
                        </>
                    ),
                    children:
                        path === "*"
                            ? [{path, element: <StoryRoute />}]
                            : [
                                  {path, element: <StoryRoute />},
                                  {path: "*", element: null},
                              ],
                },
            ],
            {initialEntries}
        )
    )

    return (
        <StoryElement.Provider value={story}>
            <RouterProvider router={router} />
        </StoryElement.Provider>
    )
}

/** Renders every story inside an in-memory data router (`parameters.router`). */
export const withMemoryRouter: Decorator = (Story, {parameters}) => {
    const {path = "*", initialEntries = ["/"]}: RouterParameters = parameters.router ?? {}

    return (
        <MemoryRouterHost
            key={JSON.stringify([path, initialEntries])}
            path={path}
            initialEntries={initialEntries}
            story={<Story />}
        />
    )
}
