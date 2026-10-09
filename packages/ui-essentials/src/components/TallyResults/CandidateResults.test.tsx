/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, waitFor} from "@testing-library/react"

jest.mock(
    "@sequentech/ui-core",
    () => ({
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/types/VotingChannel"
        ),
        escapeHtml: jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/services/stringToHtml"
        ).escapeHtml,
        formatPercentOne: (value: number) => `${value}%`,
    }),
    {virtual: true}
)

jest.mock("@mui/x-data-grid", () => ({DataGrid: () => null}))

import {CandidateResultsChart} from "./CandidateResults"

const MARKUP_NAME = '<i class="candidate-name-markup">Ann</i> & Co'

const identityMatrix = () => ({
    a: 1,
    b: 0,
    c: 0,
    d: 1,
    e: 0,
    f: 0,
    inverse: identityMatrix,
    multiply: identityMatrix,
})

beforeAll(() => {
    // apexcharts observes its container and measures SVG elements, which jsdom does not implement.
    window.ResizeObserver = class {
        observe() {}
        unobserve() {}
        disconnect() {}
    }
    Object.assign(window.SVGElement.prototype, {
        getBBox: () => ({x: 0, y: 0, width: 10, height: 10}),
        getComputedTextLength: () => 10,
        getScreenCTM: identityMatrix,
        createSVGMatrix: identityMatrix,
    })
})

describe("CandidateResultsChart", () => {
    it("renders candidate names as text in the chart legend and tooltip", async () => {
        const {container} = render(
            <CandidateResultsChart
                chartName="Election - Contest"
                results={[
                    {id: "candidate-1", name: MARKUP_NAME, castVotes: 3},
                    {id: "candidate-2", name: "Plain name", castVotes: 1},
                ]}
            />
        )

        const legendTexts = await waitFor(() => {
            const nodes = container.querySelectorAll(".apexcharts-legend-text")
            expect(nodes).toHaveLength(2)
            return nodes
        })
        expect(legendTexts[0].textContent).toBe(MARKUP_NAME)
        expect(legendTexts[1].textContent).toBe("Plain name")
        expect(container.querySelector(".apexcharts-legend .candidate-name-markup")).toBeNull()

        fireEvent.mouseMove(container.querySelectorAll(".apexcharts-series")[0])

        await waitFor(() =>
            expect(
                container.querySelector(".apexcharts-tooltip-text-y-label")?.textContent
            ).toContain(MARKUP_NAME)
        )
        expect(container.querySelector(".apexcharts-tooltip .candidate-name-markup")).toBeNull()
    })
})
