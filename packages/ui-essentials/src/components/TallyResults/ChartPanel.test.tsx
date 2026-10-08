// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {Chart} from "./ChartPanel"

jest.mock("react-apexcharts", () => {
    const react = jest.requireActual<typeof import("react")>("react")
    return {
        __esModule: true,
        default: ({options}: import("react-apexcharts").Props) => {
            const legend = globalThis.document.createElement("div")
            const el = globalThis.document.createElement("div")
            legend.className = "apexcharts-legend"
            el.append(legend)
            options?.chart?.events?.mounted?.({el})
            const mountedIndex = legend.tabIndex
            legend.tabIndex = -1
            options?.chart?.events?.updated?.({el})
            options?.chart?.events?.mounted?.({el: globalThis.document.createElement("div")})
            return react.createElement("div", {
                "data-mounted-index": mountedIndex,
                "data-updated-index": legend.tabIndex,
            })
        },
    }
})

it("makes the rendered legend keyboard reachable on initial and updated charts", () => {
    const mounted = jest.fn()
    const updated = jest.fn()
    const markup = renderToStaticMarkup(<Chart options={{chart: {events: {mounted, updated}}}} />)
    expect(markup).toContain('data-mounted-index="0"')
    expect(markup).toContain('data-updated-index="0"')
    expect(mounted).toHaveBeenCalledTimes(2)
    expect(updated).toHaveBeenCalledTimes(1)
})

it("keeps legends keyboard reachable when the caller supplies no options or callbacks", () => {
    expect(renderToStaticMarkup(<Chart />)).toContain('data-updated-index="0"')
    expect(renderToStaticMarkup(<Chart options={{chart: {}}} />)).toContain(
        'data-mounted-index="0"'
    )
})
