/** @jest-environment node */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {readFileSync, readdirSync} from "fs"
import {join, relative} from "path"
import ts from "typescript"

const sourceFiles = (directory: string): string[] =>
    readdirSync(directory, {withFileTypes: true}).flatMap((entry) => {
        if (entry.name === "__mocks__") return []
        const path = join(directory, entry.name)
        return entry.isDirectory()
            ? sourceFiles(path)
            : entry.name.endsWith(".tsx") && !/\.(test|stories)\./.test(entry.name)
              ? [path]
              : []
    })

// These shared components forward a hook to a DOM element. Composite components
// (Question, Stepper, providers, etc.) are checked at their rendered elements.
const sharedDomComponents = [
    "Dialog",
    "BallotIdHelpDialog",
    "IconButton",
    "Icon",
    "InfoDataBox",
    "PageLimit",
    "PageBanner",
    "DecorativeIconBox",
    "VisuallyHidden",
    "WarnBox",
]
const sharedComponents = [
    "BallotHash",
    "BlankAnswer",
    "BreadCrumbSteps",
    "Candidate",
    "CandidatesList",
    "ConfirmationActions",
    "CountdownBar",
    "Dialog",
    "ExpandableText",
    "Footer",
    "Header",
    "IconButton",
    "LanguageMenu",
    "Loader",
    "ProfileMenu",
    "QRCode",
    "SelectElection",
    "Version",
    "WarnBox",
]
const files = [
    ...sourceFiles(__dirname),
    ...sharedComponents.flatMap((name) =>
        sourceFiles(join(__dirname, "../../ui-essentials/src/components", name))
    ),
]

it.each(files.map((file) => [relative(__dirname, file), file]))(
    "%s gives every authored DOM element a CSS hook",
    (_name, file) => {
        const source = ts.createSourceFile(
            file,
            readFileSync(file, "utf8"),
            ts.ScriptTarget.Latest,
            true,
            ts.ScriptKind.TSX
        )
        const domComponents = new Set(sharedDomComponents)
        const collect = (node: ts.Node) => {
            if (
                ts.isImportDeclaration(node) &&
                ts.isStringLiteral(node.moduleSpecifier) &&
                /^(?:@mui\/(?:material|icons-material)(?:\/|$)|@fortawesome\/react-fontawesome$|mui-image$)/.test(
                    node.moduleSpecifier.text
                )
            ) {
                const clause = node.importClause
                if (clause?.name) domComponents.add(clause.name.text)
                if (clause?.namedBindings && ts.isNamedImports(clause.namedBindings)) {
                    clause.namedBindings.elements.forEach((item) =>
                        domComponents.add(item.name.text)
                    )
                }
            }
            if (
                ts.isVariableDeclaration(node) &&
                ts.isIdentifier(node.name) &&
                node.initializer &&
                /^styled[<(]/.test(node.initializer.getText(source))
            )
                domComponents.add(node.name.text)
            ts.forEachChild(node, collect)
        }
        collect(source)
        const missing: string[] = []
        const visit = (node: ts.Node) => {
            if (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) {
                const tag = node.tagName.getText(source)
                const property = tag === "IconButton" ? "buttonClassName" : "className"
                const hasHook = node.attributes.properties.some(
                    (attribute) =>
                        ts.isJsxAttribute(attribute) &&
                        [property, "classes"].includes(attribute.name.getText(source))
                )
                // Candidate's icon compatibility wrappers forward all SVG props.
                const forwardsIconProps =
                    tag === "Icon" &&
                    node.attributes.properties.some((attribute) =>
                        ts.isJsxSpreadAttribute(attribute)
                    )
                if (
                    tag !== "ThemeProvider" &&
                    (/^[a-z]/.test(tag) || domComponents.has(tag)) &&
                    !hasHook &&
                    !forwardsIconProps
                ) {
                    missing.push(
                        `${source.getLineAndCharacterOfPosition(node.getStart()).line + 1}: ${tag}`
                    )
                }
            }
            ts.forEachChild(node, visit)
        }
        visit(source)
        expect(missing).toEqual([])
    }
)
