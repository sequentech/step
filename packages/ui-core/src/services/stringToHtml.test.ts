// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createElement, Fragment, ReactNode} from "react"
import {renderToStaticMarkup} from "react-dom/server"
import i18next from "i18next"
import {
    escapeHtml,
    escapeTranslationValues,
    stringToHtml,
    stringToText,
    translateHtml,
} from "./stringToHtml"

const render = (node: ReactNode): string =>
    renderToStaticMarkup(createElement(Fragment, null, node))

describe("stringToHtml", () => {
    it("parses markup instead of escaping it", () => {
        expect(render(stringToHtml("<p>Cast your <strong>ballot</strong></p>"))).toBe(
            "<p>Cast your <strong>ballot</strong></p>"
        )
    })

    it("returns plain text unchanged", () => {
        expect(render(stringToHtml("Follow these steps:"))).toBe("Follow these steps:")
    })

    it("keeps characters that only look like markup", () => {
        // rendered entities; the browser displays "Tom & Jerry" and "Choose < 3"
        expect(render(stringToHtml("Tom & Jerry"))).toBe("Tom &amp; Jerry")
        expect(render(stringToHtml("Choose < 3 options"))).toBe("Choose &lt; 3 options")
        expect(render(stringToHtml("1 < 2"))).toBe("1 &lt; 2")
    })

    // a "<" that no ">" ever closes cannot open a tag, so the text is kept
    // rather than being swallowed by the parser looking for the tag's end
    it("keeps text after a '<' that never closes", () => {
        expect(render(stringToHtml("a <b and c"))).toBe("a &lt;b and c")
        expect(render(stringToHtml("<p>hello<b"))).toBe("<p>hello&lt;b</p>")
    })

    // a later ">" does close the tag, so the sanitizer consumes it as markup
    it("treats a '<' as a tag when a later '>' closes it", () => {
        expect(render(stringToHtml("a <b and c > d"))).toBe("a <b> d</b>")
    })

    // escaping an earlier "<" must not free a tag that the sanitizer was
    // discarding as another tag's attributes
    it("does not turn swallowed markup into a live element", () => {
        expect(render(stringToHtml('<x <a href="https://evil.example">click</a>'))).toBe("click")
        expect(render(stringToHtml("<zz <h1>BALLOT REJECTED</h1>"))).toBe("BALLOT REJECTED")
    })

    it("still strips a comment containing an angle bracket", () => {
        expect(render(stringToHtml("<!-- a < b -->"))).toBe("")
    })

    // the "<" inside the attribute must not make the opening "<" look unclosed
    it("keeps a tag whose quoted attribute contains an angle bracket", () => {
        expect(render(stringToHtml('<a title="a < b">link</a>'))).toBe(
            '<a title="a &lt; b">link</a>'
        )
        expect(render(stringToHtml("<a title='a < b'>link</a>"))).toBe(
            '<a title="a &lt; b">link</a>'
        )
    })

    // a "<" that does open a tag is still handled by the sanitizer, so a
    // disallowed tag is dropped. Translations needing a literal one write &lt;
    it("still drops a tag the sanitizer does not allow", () => {
        expect(render(stringToHtml("Use the <name> field"))).toBe("Use the  field")
    })

    it("leaves real markup untouched", () => {
        expect(render(stringToHtml('<a href="https://x.example">go</a> now'))).toBe(
            '<a href="https://x.example">go</a> now'
        )
    })

    it("renders an empty string for empty input", () => {
        expect(render(stringToHtml(""))).toBe("")
    })

    it("strips scripts, event handlers and javascript: urls", () => {
        expect(render(stringToHtml("<script>alert(1)</script>"))).not.toContain("alert")
        expect(render(stringToHtml('<b onclick="alert(1)">x</b>'))).toBe("<b>x</b>")
        expect(render(stringToHtml('<a href="javascript:alert(1)">x</a>'))).toBe("<a>x</a>")
    })
})

describe("stringToText", () => {
    it("removes markup and decodes entities", () => {
        expect(stringToText("Enter your <b>Ballot ID</b>")).toBe("Enter your Ballot ID")
        expect(stringToText("Smith & Sons")).toBe("Smith & Sons")
    })

    it("returns an empty string for empty input", () => {
        expect(stringToText("")).toBe("")
    })

    it("keeps text after a '<' that never closes", () => {
        expect(stringToText("a <b and c")).toBe("a <b and c")
    })
})

describe("escapeHtml", () => {
    it("escapes every character that could open a tag or attribute", () => {
        expect(escapeHtml("<a href=\"x\" title='y'>&</a>")).toBe(
            "&lt;a href=&quot;x&quot; title=&#39;y&#39;&gt;&amp;&lt;/a&gt;"
        )
    })

    it("leaves ordinary text alone", () => {
        expect(escapeHtml("ballot-123")).toBe("ballot-123")
    })
})

describe("escapeTranslationValues", () => {
    it("escapes strings and passes numbers and booleans through", () => {
        expect(escapeTranslationValues({id: "<b>x</b>", count: 3, flag: true})).toEqual({
            id: "&lt;b&gt;x&lt;/b&gt;",
            count: 3,
            flag: true,
        })
    })

    it("preserves null and undefined", () => {
        expect(escapeTranslationValues({a: null, b: undefined})).toEqual({a: null, b: undefined})
    })

    it("escapes values that are not strings but interpolate as one", () => {
        // reaches i18next as String(value), so it must be escaped too
        const values = {list: ['<a href="https://evil.example">x</a>'] as unknown as string}
        expect(escapeTranslationValues(values).list).toBe(
            "&lt;a href=&quot;https://evil.example&quot;&gt;x&lt;/a&gt;"
        )
    })
})

describe("translateHtml", () => {
    const injected = '<a href="https://evil.example">Re-cast your vote</a>'

    beforeAll(async () => {
        await i18next.init({
            lng: "en",
            // matches the global configuration in services/i18n.ts
            interpolation: {escapeValue: false},
            resources: {
                en: {
                    translation: {
                        notFound: "Ballot <b>{{ballotId}}</b> was not found",
                        // an override may opt out of i18next escaping with {{- }}
                        notFoundUnescaped: "Ballot {{- ballotId}} was not found",
                        nested: "Voter {{voter.name}} was not found",
                        plural_one: "{{count}} ballot",
                        plural_other: "{{count}} ballots",
                    },
                },
            },
        })
    })

    const translate = (key: string, values?: Record<string, unknown>) =>
        values ? i18next.t(key, values) : i18next.t(key)

    it("renders markup that comes from the translation", () => {
        expect(render(translateHtml(translate, "notFound", {ballotId: "abc123"}))).toBe(
            "Ballot <b>abc123</b> was not found"
        )
    })

    it("keeps an interpolated value inert and visible", () => {
        const output = render(translateHtml(translate, "notFound", {ballotId: injected}))
        expect(output).not.toContain("<a ")
        expect(output).toContain("&lt;a href=")
    })

    // regression: relying on i18next's escapeValue left this open, because the
    // translation itself decides whether {{- value}} skips escaping
    it("keeps an interpolated value inert even when the translation uses {{- }}", () => {
        const output = render(translateHtml(translate, "notFoundUnescaped", {ballotId: injected}))
        expect(output).not.toContain("<a ")
        expect(output).toContain("&lt;a href=")
    })

    it("does not let a nested value bypass escaping", () => {
        const values = {voter: {name: injected} as unknown as string}
        expect(render(translateHtml(translate, "nested", values))).not.toContain("<a ")
    })

    it("keeps plurals working", () => {
        expect(render(translateHtml(translate, "plural", {count: 1}))).toBe("1 ballot")
        expect(render(translateHtml(translate, "plural", {count: 5}))).toBe("5 ballots")
    })

    it("renders a translation with no values", () => {
        expect(render(translateHtml(translate, "missing.key"))).toBe("missing.key")
    })
})
