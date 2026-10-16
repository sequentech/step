// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {sanitizePresentationCss} from "./sanitizePresentationCss"

const scope = {
    baseUrl: "https://vote.example.org/tenant/t1/event/e1/election-chooser",
    publicBucketUrl: "https://files.example.org/public/",
}

const sanitize = (css: string): string => sanitizePresentationCss(css, scope)

describe("sanitizePresentationCss", () => {
    it("keeps ordinary styling, nested rules and media queries", () => {
        const css = [
            "color: rgb(1, 2, 3);",
            ".candidate-item { border: 1px solid red; &:hover { color: blue; } }",
            "@media (max-width: 600px) { .header { display: none; } }",
            "@keyframes pulse { from { opacity: 0; } to { opacity: 1; } }",
        ].join("\n")

        expect(sanitize(css)).toBe(css)
    })

    it("returns an empty string for empty input", () => {
        expect(sanitize("")).toBe("")
    })

    it.each([
        ["a path on the portal origin", "url(/assets/bg.png)", "/assets/bg.png"],
        ["a relative path", "url('images/bg.png')", "images/bg.png"],
        [
            "the public bucket",
            'url("https://files.example.org/public/tenant-t1/bg.png")',
            "https://files.example.org/public/tenant-t1/bg.png",
        ],
        ["a data URI", "url(data:image/png;base64,AAAA)", "data:image/png;base64,AAAA"],
    ])("keeps background images from %s", (_name, value, url) => {
        const out = sanitize(`.banner { background-image: ${value}; color: red; }`)

        expect(out).toContain(`background-image: url("${url}")`)
        expect(out).toContain("color: red")
    })

    it.each([
        ["an absolute URL", "url(https://elsewhere.example/a.png)"],
        ["a protocol-relative URL", 'url("//elsewhere.example/a.png")'],
        ["an upper-case function name", "URL('https://elsewhere.example/a.png')"],
        ["a slash-backslash path", "url(/\\elsewhere.example/a.png)"],
        ["an escaped function name", "u\\72l(https://elsewhere.example/a.png)"],
        ["a function name split by a comment", "ur/**/l(https://elsewhere.example/a.png)"],
        ["a function name split by a line break", "ur//\nl(https://elsewhere.example/a.png)"],
        ["user info before the host", "url(https://vote.example.org@elsewhere.example/a.png)"],
        ["another port on the portal host", "url(https://vote.example.org:8443/a.png)"],
        ["a sibling of the public bucket", "url(https://files.example.org/public-x/a.png)"],
        ["another path on the bucket host", "url(https://files.example.org/other/a.png)"],
        ["an image set", 'image-set("https://elsewhere.example/a.png" 1x)'],
        ["a prefixed image set", '-webkit-image-set("https://elsewhere.example/a.png" 1x)'],
        ["a src function", 'src("https://elsewhere.example/a.png")'],
        ["a form feed before the parenthesis", 'url\f("https://elsewhere.example/a.png") &'],
        [
            "a null character inside a string",
            '"\0" "{} .d{background:url(https://elsewhere.example/a.png)} .e{"',
        ],
        [
            "text that grows in lower case",
            `${"\u0130".repeat(40)} url(https://elsewhere.example/a.png)`,
        ],
    ])("drops background images loaded through %s", (_name, value) => {
        const out = sanitize(
            `.candidate-item:has(.candidate-input:checked) { color: red; background: ${value}; }`
        )

        expect(out).not.toContain("a.png")
        expect(out).toContain(".candidate-item:has(.candidate-input:checked)")
        expect(out).toContain("color: red")
    })

    it("drops imported stylesheets and font faces", () => {
        const out = sanitize(
            [
                "@import url(https://elsewhere.example/a.css);",
                '@import "https://elsewhere.example/b.css";',
                "@IMPORT 'https://elsewhere.example/c.css';",
                "@font-face { font-family: Custom; src: url(/fonts/custom.woff2); }",
                ".title { color: red; }",
            ].join("\n")
        )

        expect(out).not.toMatch(/@import|@font-face|elsewhere|custom\.woff2/i)
        expect(out).toContain(".title { color: red; }")
    })

    it("drops generated text and keeps empty decorative content", () => {
        const out = sanitize(
            [
                '.candidate-item:nth-of-type(2) .candidate-title::after { content: "Other"; color: red; }',
                ".candidate-item:nth-of-type(3) .candidate-title::after { content: 'Other'; }",
                '.candidate-item:nth-of-type(4) { --label: "Other"; }',
                ".candidate-item:nth-of-type(4)::before { content: var(--label); }",
                '.candidate-item:nth-of-type(5) { display: list-item; list-style-type: "Other"; }',
                '.candidate-title { text-overflow: "Other"; }',
                '.divider::before { content: ""; display: block; }',
            ].join("\n")
        )

        expect(out).not.toMatch(/content: ["']Other|content: var|list-style-type|text-overflow/)
        expect(out).toContain('content: ""; display: block;')
        expect(out).toContain("color: red")
        expect(out).toContain("display: list-item")
    })

    it("drops declarations with escapes or comments inside strings", () => {
        const out = sanitize(
            '.a { color: red; font-family: "x\\" y"; grid-template-areas: "/* a */"; }'
        )

        expect(out).toBe(".a { color: red; }")
    })

    it("drops text that would change the generated class name", () => {
        expect(sanitize('.a { color: red; font-family: "label:x}y;"; }')).toBe(".a { color: red; }")
        expect(sanitize(".a { --tab-label: 12px; }")).toBe(".a { --tab-label: 12px; }")
        expect(sanitize(".a{x:label:y}@layer;")).toBe("")
    })

    it("returns an empty string when the stylesheet cannot be parsed", () => {
        expect(sanitize("} body { background: url(https://elsewhere.example/a.png) } .a {")).toBe(
            ""
        )
        expect(sanitize('.a { font-family: "unterminated; }')).toBe("")
    })
})
