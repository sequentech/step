// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {CHART_MAX_HEIGHT_FACTOR, chartSize, frameHeight} from "./chartSize"

const tall =
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 607.7" width="400"><g/></svg>'

describe("chartSize", () => {
    it("reads the root's viewBox and pixel width", () => {
        expect(chartSize(tall)).toEqual({aspect: 607.7 / 400, width: 400})
    })

    it("takes commas and a width in px, and leaves out a relative width", () => {
        expect(chartSize('<svg viewBox="0,0,200,100" width="150px"></svg>')).toEqual({
            aspect: 0.5,
            width: 150,
        })
        expect(chartSize('<svg viewBox="0 0 200 100" width="100%"></svg>')).toEqual({aspect: 0.5})
    })

    it("reads the root only, not a nested svg", () => {
        expect(chartSize('<svg width="300"><svg viewBox="0 0 10 50"></svg></svg>')).toBeNull()
    })

    it("has no size without a usable viewBox", () => {
        expect(chartSize("<svg></svg>")).toBeNull()
        expect(chartSize('<svg viewBox="0 0 0 100"></svg>')).toBeNull()
        expect(chartSize('<svg viewBox="0 0 400 -3"></svg>')).toBeNull()
        expect(chartSize('<svg viewBox="0 0 400"></svg>')).toBeNull()
        expect(chartSize("<g></g>")).toBeNull()
    })
})

describe("frameHeight", () => {
    const size = chartSize(tall)

    it("is the chart's height at the width it is drawn", () => {
        // Drawn at its own 400 px in a wider frame.
        expect(frameHeight(size, 520, 400)).toBe(608)
        // Narrowed to a 300 px frame.
        expect(frameHeight(size, 300, 400)).toBe(Math.ceil(300 * (607.7 / 400)))
        // No width of its own: as wide as the frame.
        expect(frameHeight({aspect: 0.5}, 600, 280)).toBe(300)
    })

    it("stops at twice the widget height; the frame scrolls past that", () => {
        expect(CHART_MAX_HEIGHT_FACTOR).toBe(2)
        expect(frameHeight(size, 520, 280)).toBe(560)
    })

    it("is the widget height with no viewBox or before the frame is measured", () => {
        expect(frameHeight(null, 520, 280)).toBe(280)
        expect(frameHeight(size, null, 280)).toBe(280)
        expect(frameHeight(size, 0, 280)).toBe(280)
    })
})
