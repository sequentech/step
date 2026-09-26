// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {expect, it} from "@jest/globals"
import {renderToStaticMarkup} from "react-dom/server"
import {stringToHtml} from "./stringToHtml"

const render = (html: string) => renderToStaticMarkup(<>{stringToHtml(html)}</>)

it("removes scripts, event handlers and executable URL schemes from authored content", () => {
    const output = render(
        '<script>alert(1)</script><p onclick="alert(2)">Read this</p><a href="javascript:alert(3)">Link</a><img src=x onerror="alert(4)">'
    )
    expect(output).toBe("<p>Read this</p><a>Link</a>")
})

it("preserves safe links, language changes and right-to-left passages", () => {
    expect(
        render(
            '<p lang="ar" dir="rtl">مرحبا <a href="https://example.org/help" title="Help">help</a></p>'
        )
    ).toBe(
        '<p lang="ar" dir="rtl">مرحبا <a href="https://example.org/help" title="Help">help</a></p>'
    )
})

it("prevents authored paragraphs from overriding application accessibility labels", () => {
    // The visible confirmation must remain visible to assistive technology.
    expect(
        render(
            '<p id="security-confirmation" role="presentation" aria-hidden="true" style="display:none">Confirm</p>'
        )
    ).toBe("<p>Confirm</p>")
})

it("retains explicit table header relationships for screen readers", () => {
    const output = render(
        '<table><tr><th id="name" scope="col" abbr="Name">Candidate</th><td headers="name" colspan="2" rowspan="1">Ada</td></tr></table>'
    )
    expect(output).toContain('<th id="name" scope="col" abbr="Name">Candidate</th>')
    expect(output).toContain('<td headers="name" colSpan="2" rowSpan="1">Ada</td>')
})
