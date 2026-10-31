// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "node:fs"
import {dirname, resolve} from "node:path"
import {fileURLToPath} from "node:url"
import ts from "typescript"

export const APP_FILE = resolve(dirname(fileURLToPath(import.meta.url)), "../../../src/App.tsx")

/** React-admin's URL for each view a `<Resource>` declares. */
const RESOURCE_VIEWS = {
    list: (name: string) => `/${name}`,
    create: (name: string) => `/${name}/create`,
    edit: (name: string) => `/${name}/:id`,
    show: (name: string) => `/${name}/:id/show`,
} as const

export type RouteView = "custom" | keyof typeof RESOURCE_VIEWS

export interface AppRoute {
    /** The route pattern, as react-router matches it. */
    path: string
    view: RouteView
    /** The component the route renders, as written in the router. */
    component: string
}

function attributes(element: ts.JsxOpeningLikeElement) {
    const found = new Map<string, ts.JsxAttribute>()
    for (const property of element.attributes.properties)
        if (ts.isJsxAttribute(property)) found.set(property.name.getText(), property)
    return found
}

function stringValue(attribute: ts.JsxAttribute | undefined): string | undefined {
    const value = attribute?.initializer
    if (value && ts.isStringLiteral(value)) return value.text
}

function expressionText(attribute: ts.JsxAttribute | undefined): string {
    const value = attribute?.initializer
    return value && ts.isJsxExpression(value) && value.expression ? value.expression.getText() : ""
}

/**
 * Every route the admin router declares: the custom routes and the list,
 * create, edit and show views of each resource. Read from the router's source
 * so that adding a route there changes this list.
 */
export function appRoutes(file = APP_FILE): AppRoute[] {
    const source = ts.createSourceFile(
        file,
        readFileSync(file, "utf8"),
        ts.ScriptTarget.Latest,
        true,
        ts.ScriptKind.TSX
    )
    const routes: AppRoute[] = []
    const visit = (node: ts.Node) => {
        if (ts.isJsxSelfClosingElement(node) || ts.isJsxOpeningElement(node)) {
            const tag = node.tagName.getText()
            const found = attributes(node)
            if (tag === "Route") {
                const path = stringValue(found.get("path"))
                if (!path) throw new Error(`A <Route> in ${file} has no literal path`)
                routes.push({path, view: "custom", component: expressionText(found.get("element"))})
            } else if (tag === "Resource") {
                const name = stringValue(found.get("name"))
                if (!name) throw new Error(`A <Resource> in ${file} has no literal name`)
                for (const [view, url] of Object.entries(RESOURCE_VIEWS)) {
                    const component = expressionText(found.get(view))
                    if (component)
                        routes.push({path: url(name), view: view as RouteView, component})
                }
            }
        }
        ts.forEachChild(node, visit)
    }
    visit(source)
    return routes
}
