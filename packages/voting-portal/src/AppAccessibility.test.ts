// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {readFileSync} from "fs"
import {join} from "path"
import ts from "typescript"

it("places the skip link inside a navigation landmark", () => {
    const source = ts.createSourceFile(
        "App.tsx",
        readFileSync(join(__dirname, "App.tsx"), "utf8"),
        ts.ScriptTarget.Latest,
        true,
        ts.ScriptKind.TSX
    )
    let skipLink: ts.JsxElement | undefined
    const visit = (node: ts.Node) => {
        if (
            ts.isJsxElement(node) &&
            node.openingElement.tagName.getText(source) === "a" &&
            node.openingElement.attributes.properties.some(
                (attribute) =>
                    ts.isJsxAttribute(attribute) &&
                    attribute.name.getText(source) === "href" &&
                    attribute.initializer &&
                    ts.isStringLiteral(attribute.initializer) &&
                    attribute.initializer.text === "#main-content"
            )
        ) {
            skipLink = node
        }
        ts.forEachChild(node, visit)
    }
    visit(source)

    expect(skipLink).toBeDefined()
    const parent = skipLink!.parent
    expect(ts.isJsxElement(parent) && parent.openingElement.tagName.getText(source)).toBe("nav")
})

it("hides only SVG.js's off-screen measurement SVG without removing its layout", () => {
    const style = document.createElement("style")
    style.textContent = readFileSync(join(__dirname, "index.css"), "utf8")
    document.head.appendChild(style)

    const createSvg = (id: string, width: string, height: string) => {
        const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg")
        svg.id = id
        svg.setAttribute("width", width)
        svg.setAttribute("height", height)
        document.body.appendChild(svg)
        return svg
    }
    const helper = createSvg("SvgjsSvg1001", "2", "0")
    const chart = createSvg("SvgjsSvg1002", "300", "200")
    const otherSvg = createSvg("meaningful-svg", "2", "0")

    try {
        expect(getComputedStyle(helper).visibility).toBe("hidden")
        expect(getComputedStyle(helper).display).not.toBe("none")
        expect(getComputedStyle(chart).visibility).toBe("visible")
        expect(getComputedStyle(otherSvg).visibility).toBe("visible")
    } finally {
        helper.remove()
        chart.remove()
        otherSvg.remove()
        style.remove()
    }
})
