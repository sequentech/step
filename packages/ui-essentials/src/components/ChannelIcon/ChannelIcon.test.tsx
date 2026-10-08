// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import ChannelIcon, {CHANNEL_ICON_NAMES, ChannelLabel} from "./ChannelIcon"

describe("ChannelIcon", () => {
    it("draws every channel as an outline", () => {
        for (const channel of CHANNEL_ICON_NAMES) {
            const {container, unmount} = render(<ChannelIcon channel={channel} />)
            const svg = container.querySelector("svg")
            expect(svg).not.toBeNull()
            expect(svg).toHaveAttribute("data-channel", channel)
            expect(svg?.getAttribute("fill")).toBe("none")
            expect(svg?.getAttribute("stroke")).toBe("currentColor")
            expect(svg?.querySelectorAll("path").length).toBeGreaterThan(0)
            unmount()
        }
    })

    it("gives each channel its own drawing", () => {
        const drawings = CHANNEL_ICON_NAMES.map((channel) => {
            const {container, unmount} = render(<ChannelIcon channel={channel} />)
            const paths = Array.from(container.querySelectorAll("path"))
                .map((path) => path.getAttribute("d"))
                .join(" ")
            unmount()
            return paths
        })
        expect(new Set(drawings).size).toBe(CHANNEL_ICON_NAMES.length)
    })

    it("is decorative unless titled", () => {
        const {container, rerender} = render(<ChannelIcon channel="VIBER" />)
        expect(container.querySelector("svg")).toHaveAttribute("aria-hidden", "true")
        rerender(<ChannelIcon channel="VIBER" titleAccess="Viber" />)
        expect(screen.getByRole("img", {name: "Viber"})).toBeInTheDocument()
    })

    it("keeps the outline with one style or several", () => {
        for (const sx of [{color: "red"}, [{color: "red"}, {opacity: 0.5}]]) {
            const {container, unmount} = render(<ChannelIcon channel="SMS" sx={sx} />)
            const svg = container.querySelector("svg") as SVGSVGElement
            expect(getComputedStyle(svg).fill).toBe("none")
            expect(getComputedStyle(svg).color).toBe("rgb(255, 0, 0)")
            unmount()
        }
    })

    it("labels a channel next to its icon", () => {
        render(<ChannelLabel channel="MESSENGER" label="Facebook Messenger" />)
        expect(screen.getByText("Facebook Messenger")).toBeInTheDocument()
    })
})
