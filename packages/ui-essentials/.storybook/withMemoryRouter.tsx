// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"
import type {Decorator} from "@storybook/react-vite"
import {
    createMemoryRouter,
    Outlet,
    useLocation,
    type ActionFunction,
    type LoaderFunction,
} from "react-router"
import {RouterProvider} from "react-router/dom"

/** `parameters.router` of a story. */
export interface RouterParameters {
    /** Route pattern that renders the story, e.g. "/tenant/:tenantId/event/:eventId". */
    path?: string
    /** Initial history stack; the last entry is the current location. */
    initialEntries?: string[]
    /** Optional parent route for screens whose action redirects relative to a sibling. */
    parentPath?: string
    /** Real route action used by screens submitting through the data router. */
    action?: ActionFunction
    /** Real route loader and boundary for failure-state stories. */
    loader?: LoaderFunction
    errorElement?: React.ReactNode
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

const MemoryRouterHost: React.FC<
    RouterParameters & {path: string; initialEntries: string[]; story: React.ReactNode}
> = ({path, initialEntries, parentPath, action, loader, errorElement, story}) => {
    const [router] = useState(() =>
        createMemoryRouter(
            [
                {
                    path: parentPath,
                    element: (
                        <>
                            <Outlet />
                            <CurrentLocation />
                        </>
                    ),
                    children:
                        path === "*"
                            ? [{path, action, loader, errorElement, element: <StoryRoute />}]
                            : [
                                  {path, action, loader, errorElement, element: <StoryRoute />},
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
    const {
        path = "*",
        initialEntries = ["/"],
        parentPath,
        action,
        loader,
        errorElement,
    }: RouterParameters = parameters.router ?? {}

    return (
        <MemoryRouterHost
            key={JSON.stringify([path, initialEntries, parentPath])}
            path={path}
            initialEntries={initialEntries}
            parentPath={parentPath}
            action={action}
            loader={loader}
            errorElement={errorElement}
            story={<Story />}
        />
    )
}
