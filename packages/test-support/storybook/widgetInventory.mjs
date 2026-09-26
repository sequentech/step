// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// @ts-check

// Maps a package's authored React components to the Storybook sections and
// stories that render them. Plain JavaScript, like test-story.mjs, so that it
// runs on every Node version the workspace uses without a TypeScript loader.
import {readdirSync, readFileSync, existsSync, statSync} from "node:fs"
import {dirname, join, relative, resolve, sep} from "node:path"
import {createRequire} from "node:module"
import {loadCsf} from "storybook/internal/csf-tools"
import {sanitize} from "storybook/internal/csf"

/**
 * Package configuration, usually `.storybook/widgets.mjs`.
 * @typedef {object} WidgetConfig
 * @property {string} sources Source directory, relative to the package.
 * @property {string} title Title prefix of widget sections, e.g. `Admin`.
 * @property {string[]} screenTitles Title prefixes of screen stories that also count as sections.
 * @property {string} storybookUrl Storybook address used in direct links.
 * @property {(file: string) => string} feature Sidebar group of a source file.
 * @property {Record<string, string>} excluded `file#Component` keys that are not
 *     widgets, each with the reason.
 */

/**
 * An authored component: a top-level PascalCase binding that renders JSX.
 * @typedef {object} Component
 * @property {string} file Package-relative path with `/` separators.
 * @property {string} name Public name: the export alias when exported, otherwise the binding.
 * @property {string} local Name of the binding that renders.
 * @property {string[]} exportNames Names it is exported as, including `default`.
 * @property {"component" | "styled"} kind `styled` marks styled-component primitives.
 * @property {number} line
 */

/**
 * @typedef {object} Story
 * @property {string} id
 * @property {string} name
 * @property {string} exportName
 * @property {string[]} widgets Components of the section's module that the story shows.
 */

/**
 * A story file: one Storybook sidebar section.
 * @typedef {object} Section
 * @property {string} file
 * @property {string} title
 * @property {string} id
 * @property {{file: string, exportName: string} | undefined} component Resolved `meta.component`.
 * @property {string | undefined} componentName Name of `meta.component` as written.
 * @property {Story[]} stories
 */

/**
 * @typedef {object} WidgetEntry
 * @property {Component} component
 * @property {string} feature
 * @property {string} expectedTitle
 * @property {"covered" | "remaining" | "excluded"} status
 * @property {string | undefined} reason Exclusion reason.
 * @property {Section[]} sections Sections whose `meta.component` is this widget.
 * @property {Story[]} stories Stories of other sections of the same module that show it.
 */

/** @param {string} path */
const posix = (path) => path.split(sep).join("/")

/**
 * Humanizes a directory name: `ElectionEvent` and `election-event` become `Election event`.
 * @param {string} name
 */
export function humanize(name) {
    const words = name
        .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
        .split(/[\s_-]+/)
        .filter(Boolean)
        .map((word) => word.toLowerCase())
    const [first = "", ...rest] = words
    return [first.charAt(0).toUpperCase() + first.slice(1), ...rest].join(" ")
}

/**
 * @param {string} dir
 * @param {(file: string) => boolean} accept
 * @returns {string[]}
 */
function listFiles(dir, accept) {
    return readdirSync(dir, {recursive: true, encoding: "utf8"})
        .map((file) => join(dir, file))
        .filter((file) => !file.includes(`${sep}node_modules${sep}`))
        .filter((file) => statSync(file).isFile() && accept(file))
        .sort()
}

const isStoryFile = (/** @type {string} */ file) => /\.stories\.tsx?$/.test(file)
const isSourceFile = (/** @type {string} */ file) =>
    /\.tsx?$/.test(file) &&
    !file.endsWith(".d.ts") &&
    !/\.(test|stories)\.tsx?$/.test(file) &&
    !file.includes(`${sep}__stories__${sep}`)

/**
 * Finds the components of each source module with the TypeScript parser.
 * @param {string} packageDir
 * @param {string} sources
 * @returns {Component[]}
 */
export function scanComponents(packageDir, sources) {
    const require = createRequire(join(packageDir, "package.json"))
    /** @type {typeof import("typescript")} */
    const ts = require("typescript")
    /** @type {Component[]} */
    const components = []
    for (const path of listFiles(resolve(packageDir, sources), isSourceFile)) {
        const file = posix(relative(packageDir, path))
        const source = ts.createSourceFile(
            path,
            readFileSync(path, "utf8"),
            ts.ScriptTarget.Latest,
            true,
            path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS
        )
        /** @param {import("typescript").Node} node */
        const rendersJsx = (node) => {
            let found = false
            /** @param {import("typescript").Node} child */
            const visit = (child) => {
                if (found) return
                if (
                    ts.isJsxElement(child) ||
                    ts.isJsxSelfClosingElement(child) ||
                    ts.isJsxFragment(child) ||
                    (ts.isCallExpression(child) &&
                        /(^|\.)createElement$/.test(child.expression.getText(source)))
                ) {
                    found = true
                    return
                }
                ts.forEachChild(child, visit)
            }
            visit(node)
            return found
        }
        /** @param {import("typescript").Expression} node */
        const isStyled = (node) => {
            /** @type {import("typescript").Node | undefined} */
            let current = node
            while (current) {
                if (ts.isIdentifier(current)) return current.text === "styled"
                if (ts.isCallExpression(current)) current = current.expression
                else if (ts.isTaggedTemplateExpression(current)) current = current.tag
                else if (ts.isPropertyAccessExpression(current)) current = current.expression
                else return false
            }
            return false
        }
        /** @param {import("typescript").Statement} statement */
        const exportedDirectly = (statement) =>
            ts.canHaveModifiers(statement) &&
            (ts.getModifiers(statement) ?? []).some(
                (modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword
            )
        /** @param {import("typescript").Statement} statement */
        const exportedAsDefault = (statement) =>
            ts.canHaveModifiers(statement) &&
            (ts.getModifiers(statement) ?? []).some(
                (modifier) => modifier.kind === ts.SyntaxKind.DefaultKeyword
            )

        /** @type {Map<string, {kind: "component" | "styled", line: number}>} */
        const bindings = new Map()
        /** @type {Map<string, string>} alias binding -> aliased binding */
        const aliases = new Map()
        /** @type {Map<string, Set<string>>} binding -> export names */
        const exports = new Map()
        /** @param {string} local @param {string} name */
        const addExport = (local, name) => {
            const names = exports.get(local) ?? new Set()
            names.add(name)
            exports.set(local, names)
        }
        const line = (/** @type {import("typescript").Node} */ node) =>
            source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1

        for (const statement of source.statements) {
            if (ts.isFunctionDeclaration(statement) && statement.body) {
                const name = statement.name?.text ?? (exportedAsDefault(statement) ? "default" : "")
                if ((/^[A-Z]/.test(name) || name === "default") && rendersJsx(statement.body)) {
                    bindings.set(name, {kind: "component", line: line(statement)})
                    if (exportedDirectly(statement)) {
                        addExport(name, exportedAsDefault(statement) ? "default" : name)
                    }
                }
            } else if (ts.isClassDeclaration(statement) && statement.name) {
                if (/^[A-Z]/.test(statement.name.text) && rendersJsx(statement)) {
                    bindings.set(statement.name.text, {kind: "component", line: line(statement)})
                    if (exportedDirectly(statement)) {
                        const exportName = exportedAsDefault(statement)
                            ? "default"
                            : statement.name.text
                        addExport(statement.name.text, exportName)
                    }
                }
            } else if (ts.isVariableStatement(statement)) {
                for (const declaration of statement.declarationList.declarations) {
                    if (!ts.isIdentifier(declaration.name) || !declaration.initializer) continue
                    const name = declaration.name.text
                    if (!/^[A-Z]/.test(name)) continue
                    const value = declaration.initializer
                    /** @type {"component" | "styled" | undefined} */
                    let kind
                    if (isStyled(value)) kind = "styled"
                    else if (
                        (ts.isArrowFunction(value) ||
                            ts.isFunctionExpression(value) ||
                            ts.isCallExpression(value)) &&
                        rendersJsx(value)
                    )
                        kind = "component"
                    else if (ts.isIdentifier(value)) aliases.set(name, value.text)
                    else if (
                        ts.isCallExpression(value) &&
                        value.arguments.some((argument) => ts.isIdentifier(argument))
                    ) {
                        // memo(Component) and similar wrappers of a local component.
                        const wrapped = value.arguments.find((argument) =>
                            ts.isIdentifier(argument)
                        )
                        if (wrapped && ts.isIdentifier(wrapped)) aliases.set(name, wrapped.text)
                    }
                    if (kind) bindings.set(name, {kind, line: line(declaration)})
                    if (exportedDirectly(statement)) addExport(name, name)
                }
            } else if (ts.isExportDeclaration(statement) && !statement.moduleSpecifier) {
                const clause = statement.exportClause
                if (clause && ts.isNamedExports(clause)) {
                    for (const element of clause.elements) {
                        addExport((element.propertyName ?? element.name).text, element.name.text)
                    }
                }
            } else if (ts.isExportAssignment(statement) && !statement.isExportEquals) {
                if (ts.isIdentifier(statement.expression)) {
                    addExport(statement.expression.text, "default")
                } else if (rendersJsx(statement.expression)) {
                    bindings.set("default", {kind: "component", line: line(statement)})
                    addExport("default", "default")
                }
            }
        }
        /** @param {string} name @returns {string | undefined} */
        const target = (name) => {
            const seen = new Set()
            let current = name
            while (!bindings.has(current) && aliases.has(current) && !seen.has(current)) {
                seen.add(current)
                current = /** @type {string} */ aliases.get(current)
            }
            return bindings.has(current) ? current : undefined
        }
        /** @type {Map<string, Set<string>>} rendering binding -> export names */
        const exportNames = new Map()
        for (const [local, names] of exports) {
            const binding = target(local)
            if (!binding) continue
            const all = exportNames.get(binding) ?? new Set()
            for (const name of names) all.add(name)
            exportNames.set(binding, all)
        }
        for (const [local, {kind, line}] of bindings) {
            const names = [...(exportNames.get(local) ?? [])].sort()
            // An exported alias such as `export const DiffView = DiffViewMemo`
            // names the component; a default export keeps the binding's name.
            const alias = names.find((name) => name !== "default" && name !== local)
            components.push({
                file,
                name: alias ?? (local === "default" ? "Default" : local),
                local,
                exportNames: names,
                kind,
                line,
            })
        }
    }
    return components
}

/**
 * Resolves a story's import specifier to a package-relative source file.
 * @param {string} packageDir
 * @param {string} storyFile Package-relative story file.
 * @param {string} specifier
 * @param {string} sources
 */
function resolveImport(packageDir, storyFile, specifier, sources) {
    let base
    if (specifier.startsWith("@/")) base = resolve(packageDir, sources, specifier.slice(2))
    else if (specifier.startsWith(".")) base = resolve(packageDir, dirname(storyFile), specifier)
    else return undefined
    for (const candidate of [
        base,
        `${base}.tsx`,
        `${base}.ts`,
        join(base, "index.tsx"),
        join(base, "index.ts"),
    ]) {
        if (existsSync(candidate) && statSync(candidate).isFile()) {
            return posix(relative(packageDir, candidate))
        }
    }
    return undefined
}

/**
 * Reads a string array from a story's `parameters.widgets` Babel AST node.
 * @param {any} annotations
 * @returns {string[]}
 */
function widgetParameter(annotations) {
    const parameters = annotations?.parameters
    if (!parameters || parameters.type !== "ObjectExpression") return []
    const property = parameters.properties.find(
        (/** @type {any} */ entry) =>
            entry.type === "ObjectProperty" &&
            (entry.key.name === "widgets" || entry.key.value === "widgets")
    )
    if (!property || property.value.type !== "ArrayExpression") return []
    return property.value.elements
        .filter((/** @type {any} */ element) => element?.type === "StringLiteral")
        .map((/** @type {any} */ element) => element.value)
}

/**
 * Parses the package's story files.
 * @param {string} packageDir
 * @param {string} sources
 * @returns {Section[]}
 */
export function scanSections(packageDir, sources) {
    return listFiles(resolve(packageDir, sources), isStoryFile).map((path) => {
        const file = posix(relative(packageDir, path))
        const csf = loadCsf(readFileSync(path, "utf8"), {
            fileName: path,
            makeTitle: (title) => title,
        }).parse()
        const title = csf._meta?.title ?? ""
        const specifier = /** @type {any} */ csf._componentImportSpecifier
        const componentPath = csf._rawComponentPath
        const resolved = componentPath
            ? resolveImport(packageDir, file, componentPath, sources)
            : undefined
        const exportName =
            specifier?.type === "ImportDefaultSpecifier"
                ? "default"
                : (specifier?.imported?.name ?? specifier?.imported?.value)
        const annotations = /** @type {Record<string, any>} */ csf._storyAnnotations
        return {
            file,
            title,
            id: sanitize(title),
            component: resolved && exportName ? {file: resolved, exportName} : undefined,
            componentName: csf._meta?.component,
            stories: csf.indexInputs.flatMap((input) =>
                input.type === "story" && input.__id
                    ? [
                          {
                              id: input.__id,
                              name: input.name ?? input.exportName,
                              exportName: input.exportName,
                              widgets: widgetParameter(annotations[input.exportName]),
                          },
                      ]
                    : []
            ),
        }
    })
}

/**
 * Joins components and sections.
 * @param {WidgetConfig} config
 * @param {Component[]} components
 * @param {Section[]} sections
 */
export function buildInventory(config, components, sections) {
    /** @type {WidgetEntry[]} */
    const widgets = components.map((component) => {
        const key = `${component.file}#${component.name}`
        const feature = config.feature(component.file)
        const own = sections.filter(
            (section) =>
                section.component?.file === component.file &&
                component.exportNames.includes(section.component.exportName)
        )
        const shown = sections
            .filter((section) => section.component?.file === component.file)
            .flatMap((section) =>
                section.stories.filter(
                    (story) =>
                        story.widgets.includes(component.name) ||
                        story.widgets.includes(component.local)
                )
            )
        const reason =
            config.excluded[key] ??
            (component.kind === "styled"
                ? "Styled primitive: no behavior of its own; its widget's stories render it."
                : undefined)
        return {
            component,
            feature,
            expectedTitle: `${config.title}/${feature}/${component.name}`,
            status: reason ? "excluded" : own.length || shown.length ? "covered" : "remaining",
            reason,
            sections: own,
            stories: shown,
        }
    })
    /** @type {string[]} */
    const problems = []
    const keys = new Set(widgets.map(({component}) => `${component.file}#${component.name}`))
    for (const key of Object.keys(config.excluded)) {
        if (!keys.has(key)) problems.push(`Exclusion ${key} names no component`)
    }
    for (const section of sections) {
        const isScreen = config.screenTitles.some((prefix) => section.title.startsWith(prefix))
        const widget = widgets.find(
            (entry) =>
                section.component?.file === entry.component.file &&
                entry.component.exportNames.includes(section.component.exportName)
        )
        if (!widget) {
            problems.push(
                `${section.file}: meta.component ${section.componentName ?? "(none)"} is not ` +
                    `an authored component of ${config.sources}`
            )
            continue
        }
        if (!isScreen && section.title !== widget.expectedTitle) {
            problems.push(
                `${section.file}: title "${section.title}" should be "${widget.expectedTitle}"`
            )
        }
        const names = new Set(
            widgets
                .filter((entry) => entry.component.file === widget.component.file)
                .flatMap((entry) => [entry.component.name, entry.component.local])
        )
        for (const story of section.stories) {
            for (const name of story.widgets) {
                if (!names.has(name)) {
                    problems.push(
                        `${section.file}: story ${story.id} shows ${name}, which ` +
                            `${widget.component.file} does not define`
                    )
                }
            }
        }
    }
    const ids = new Map()
    for (const section of sections) {
        const other = ids.get(section.id)
        if (other) problems.push(`${section.file} and ${other} share the title ${section.title}`)
        ids.set(section.id, section.file)
    }
    return {widgets, problems}
}

/**
 * @param {string} packageDir
 * @param {WidgetConfig} config
 */
export function inventory(packageDir, config) {
    return buildInventory(
        config,
        scanComponents(packageDir, config.sources),
        scanSections(packageDir, config.sources)
    )
}
