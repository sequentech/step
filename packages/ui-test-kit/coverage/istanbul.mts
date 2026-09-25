// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Istanbul coverage of a portal's TypeScript from V8 coverage of its production
// bundle. Statements, functions and branches come from the TypeScript source,
// as Istanbul's instrumenter would count them; each takes the V8 count of the
// innermost block around its code in the bundle, found through the source maps.

import {existsSync, readFileSync} from "node:fs"
import {readFile, readdir} from "node:fs/promises"
import {join, resolve, sep} from "node:path"
import {fileURLToPath} from "node:url"
import {mergeProcessCovs, type FunctionCov, type ProcessCov} from "@bcoe/v8-coverage"
import remapping, {type EncodedSourceMap} from "@jridgewell/remapping"
import libCoverage, {type CoverageMap} from "istanbul-lib-coverage"
import type {BranchMapping, FileCoverageData, FunctionMapping, Range} from "istanbul-lib-coverage"
import ts from "typescript"

function readJson<T>(file: string): T {
    return JSON.parse(readFileSync(file, "utf8")) as T
}

// Webpack names sources `webpack://<package name>/<path from the package>`,
// with a query suffix on some assets.
function absoluteSources(map: EncodedSourceMap, packageDir: string): EncodedSourceMap {
    const prefix = `webpack://${readJson<{name: string}>(join(packageDir, "package.json")).name}/`
    const sources = map.sources.map((source) =>
        source?.startsWith(prefix)
            ? resolve(packageDir, source.slice(prefix.length).replace(/\?[^/]*$/, ""))
            : source
    )
    return {...map, sourceRoot: undefined, sources}
}

// ts-loader emits modules without source maps, so the bundle maps end at its
// ES5 output. Emitting the checked-out source again must reproduce that output
// exactly; the new map then continues the trace to the TypeScript.
function typescriptMaps(packageDir: string) {
    const config = ts.readConfigFile(join(packageDir, "tsconfig.json"), ts.sys.readFile)
    const {options: project, fileNames} = ts.parseJsonConfigFileContent(
        config.config,
        ts.sys,
        packageDir
    )
    const options: ts.CompilerOptions = {
        ...project,
        noEmit: false,
        sourceMap: true,
        declaration: false,
        declarationMap: false,
        emitDeclarationOnly: false,
        skipLibCheck: true,
        suppressOutputPathCheck: true,
    }
    const code = (output?: string) => output?.replace(/\/\/# sourceMappingURL=\S+\s*$/, "")
    let program: ts.Program | undefined
    return (path: string, emitted: string): EncodedSourceMap | undefined => {
        const text = readFileSync(path, "utf8")
        let {outputText: js, sourceMapText: map} = ts.transpileModule(text, {
            compilerOptions: options,
            fileName: path,
        })
        // Without transpileOnly, ts-loader emits through a program, which also
        // drops imports that are only used as types.
        if (code(js) !== emitted) {
            program ??= ts.createProgram({rootNames: fileNames, options})
            const source = program.getSourceFile(path)
            if (source)
                program.emit(source, (file, output) => {
                    if (file.endsWith(".map")) map = output
                    else js = output
                })
        }
        if (code(js) !== emitted || !map) return
        return {...JSON.parse(map), sources: [path], sourcesContent: [text], sourceRoot: undefined}
    }
}

/** Positions in one module's text, each with a bundle offset that maps to it, in order. */
type Positions = [position: number, offset: number][]
interface BundleModule {
    text: string
    positions: Positions
}

function lineStarts(text: string) {
    const starts = [0]
    for (let index = text.indexOf("\n"); index !== -1; index = text.indexOf("\n", index + 1))
        starts.push(index + 1)
    return starts
}

// Terser hoists an inlined function's locals into `var a,b,c` at the top of
// the caller. Those names map back to the inlined code but never run it.
function hoistedNames(bundle: string) {
    const source = ts.createSourceFile(
        "bundle.js",
        bundle,
        ts.ScriptTarget.Latest,
        false,
        ts.ScriptKind.JS
    )
    const offsets = new Set<number>()
    const visit = (node: ts.Node): void => {
        // A for-in or for-of variable is assigned on every iteration.
        if (ts.isForInStatement(node) || ts.isForOfStatement(node)) {
            visit(node.expression)
            visit(node.statement)
            return
        }
        if (ts.isVariableDeclaration(node) && !node.initializer && ts.isIdentifier(node.name))
            offsets.add(node.name.getStart(source))
        ts.forEachChild(node, visit)
    }
    visit(source)
    return offsets
}

function bundleModules(
    bundle: string,
    map: EncodedSourceMap,
    counted: (path: string) => boolean,
    typescript: ReturnType<typeof typescriptMaps>
) {
    const emitted = new Map(
        map.sources.map((source, index) => [source, map.sourcesContent?.[index]])
    )
    let unverified = 0
    const composed = remapping(
        map,
        (source, context) => {
            const content = emitted.get(source)
            if (context.depth > 1 || !counted(source) || !content) return
            const typescriptMap = typescript(source, content)
            if (!typescriptMap) unverified++
            return typescriptMap
        },
        {decodedMappings: true}
    )
    const modules = new Map<string, BundleModule & {starts: number[]}>()
    const bySource = composed.sources.map((source, index) => {
        if (!source || !counted(source)) return
        const text = composed.sourcesContent?.[index] ?? ""
        const module = modules.get(source) ?? {text, positions: [], starts: lineStarts(text)}
        modules.set(source, module)
        return module
    })
    const generated = lineStarts(bundle)
    const hoisted = hoistedNames(bundle)
    const mappings = composed.mappings as [number, number?, number?, number?, number?][][]
    mappings.forEach((segments, line) => {
        for (const [column, source, originalLine, originalColumn] of segments) {
            const module = source === undefined ? undefined : bySource[source]
            const start = originalLine === undefined ? undefined : module?.starts[originalLine]
            const offset = generated[line] + column
            if (!module || start === undefined || originalColumn === undefined) continue
            if (!hoisted.has(offset)) module.positions.push([start + originalColumn, offset])
        }
    })
    for (const module of modules.values())
        module.positions.sort((a, b) => a[0] - b[0] || a[1] - b[1])
    return {modules, unverified}
}

interface Block {
    start: number
    end: number
    count: number
    function: boolean
    parent: number
}

// V8 counts nest: the innermost block around an offset says how often the code
// there ran. A function's own range starts at its literal, but the code at that
// offset creates the function, so it belongs to the enclosing block.
function blockCounts(functions: FunctionCov[]) {
    const blocks: Block[] = functions
        .flatMap(({ranges}) =>
            ranges.map((range, index) => ({
                start: range.startOffset,
                end: range.endOffset,
                count: range.count,
                function: index === 0,
                parent: -1,
            }))
        )
        .sort((a, b) => a.start - b.start || b.end - a.end)
    const open: number[] = []
    blocks.forEach((block, index) => {
        while (open.length && blocks[open[open.length - 1]].end < block.end) open.pop()
        block.parent = open.length ? open[open.length - 1] : -1
        open.push(index)
    })
    const calls = new Map(
        blocks.filter((block) => block.function).map((block) => [block.start, block.count])
    )
    const at = (offset: number) => {
        let low = 0
        let high = blocks.length
        while (low < high) {
            const middle = (low + high) >> 1
            if (blocks[middle].start <= offset) low = middle + 1
            else high = middle
        }
        let index = low - 1
        while (index >= 0 && blocks[index].end <= offset) index = blocks[index].parent
        if (index >= 0 && blocks[index].function && blocks[index].start === offset)
            index = blocks[index].parent
        return index >= 0 ? blocks[index].count : 0
    }
    return {at, calls: (offset: number) => calls.get(offset)}
}

interface Counts {
    at(offset: number): number
    calls(offset: number): number | undefined
}
const never: Counts = {at: () => 0, calls: () => undefined}

/** Bundle offsets of the first position in [start, end) that emits bundle code. */
function firstCode(positions: Positions, start: number, end: number) {
    let low = 0
    let high = positions.length
    while (low < high) {
        const middle = (low + high) >> 1
        if (positions[middle][0] < start) low = middle + 1
        else high = middle
    }
    const offsets: number[] = []
    const position = positions[low]?.[0]
    if (position === undefined || position >= end) return offsets
    for (let index = low; positions[index]?.[0] === position; index++)
        offsets.push(positions[index][1])
    return offsets
}

const statementKinds = new Set([
    ts.SyntaxKind.BreakStatement,
    ts.SyntaxKind.ContinueStatement,
    ts.SyntaxKind.DebuggerStatement,
    ts.SyntaxKind.DoStatement,
    ts.SyntaxKind.EnumDeclaration,
    ts.SyntaxKind.EnumMember,
    ts.SyntaxKind.ExpressionStatement,
    ts.SyntaxKind.ForInStatement,
    ts.SyntaxKind.ForOfStatement,
    ts.SyntaxKind.ForStatement,
    ts.SyntaxKind.IfStatement,
    ts.SyntaxKind.LabeledStatement,
    ts.SyntaxKind.ReturnStatement,
    ts.SyntaxKind.SwitchStatement,
    ts.SyntaxKind.ThrowStatement,
    ts.SyntaxKind.TryStatement,
    ts.SyntaxKind.WhileStatement,
    ts.SyntaxKind.WithStatement,
])
const logical = new Set([
    ts.SyntaxKind.AmpersandAmpersandToken,
    ts.SyntaxKind.BarBarToken,
    ts.SyntaxKind.QuestionQuestionToken,
])
const isLogical = (node: ts.Node): node is ts.BinaryExpression =>
    ts.isBinaryExpression(node) && logical.has(node.operatorToken.kind)
const unwrap = (node: ts.Node): ts.Node =>
    ts.isParenthesizedExpression(node) ? unwrap(node.expression) : node
const leaves = (node: ts.Node): ts.Node[] =>
    isLogical(unwrap(node))
        ? [
              ...leaves((unwrap(node) as ts.BinaryExpression).left),
              ...leaves((unwrap(node) as ts.BinaryExpression).right),
          ]
        : [node]
function parentExpression(node: ts.Node) {
    let parent = node.parent
    while (parent && ts.isParenthesizedExpression(parent)) parent = parent.parent
    return parent
}

// The statements, functions and branches that Istanbul's instrumenter counts:
// executable statements, initializers and arrow expression bodies; functions,
// methods and accessors with bodies; if, conditional, logical, switch and
// default-value branches. Only code that the bundle kept is reported.
function fileCoverage(path: string, {text, positions}: BundleModule, counts: Counts) {
    const file = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true)
    const location = (node: ts.Node): Range => {
        const start = file.getLineAndCharacterOfPosition(node.getStart(file))
        const end = file.getLineAndCharacterOfPosition(node.end)
        return {
            start: {line: start.line + 1, column: start.character},
            end: {line: end.line + 1, column: end.character},
        }
    }
    const code = (node: ts.Node) => firstCode(positions, node.getStart(file), node.end)
    const hits = (node: ts.Node) => Math.max(0, ...code(node).map(counts.at))
    const data: FileCoverageData = {
        path,
        statementMap: {},
        fnMap: {},
        branchMap: {},
        s: {},
        f: {},
        b: {},
    }
    let statements = 0
    let functions = 0
    let branches = 0
    const branch = (type: string, node: ts.Node, arms: ts.Node[], hitCounts: number[]) => {
        if (!code(node).length) return
        const loc = location(node)
        const mapping: BranchMapping = {
            type,
            loc,
            locations: arms.map(location),
            line: loc.start.line,
        }
        data.branchMap[branches] = mapping
        data.b[branches++] = hitCounts
    }
    const visit = (node: ts.Node) => {
        let statement: ts.Node | undefined
        if (statementKinds.has(node.kind)) statement = node
        else if (ts.isVariableDeclaration(node) || ts.isPropertyDeclaration(node))
            statement = node.initializer
        else if (ts.isArrowFunction(node) && !ts.isBlock(node.body)) statement = node.body
        if (statement && code(statement).length) {
            data.statementMap[statements] = location(statement)
            data.s[statements++] = hits(statement)
        }
        if (
            (ts.isFunctionDeclaration(node) ||
                ts.isFunctionExpression(node) ||
                ts.isArrowFunction(node) ||
                ts.isMethodDeclaration(node) ||
                ts.isConstructorDeclaration(node) ||
                ts.isGetAccessorDeclaration(node) ||
                ts.isSetAccessorDeclaration(node)) &&
            node.body &&
            code(node).length
        ) {
            // A literal kept as a function has a V8 range starting at its code;
            // an inlined body only has the count of the code it became.
            const starts = code(node)
                .map(counts.calls)
                .filter((calls): calls is number => calls !== undefined)
            const loc = location(node)
            const name = ts.getNameOfDeclaration(node)
            const mapping: FunctionMapping = {
                name: name?.getText(file) ?? `(anonymous_${functions})`,
                decl: name ? location(name) : loc,
                loc,
                line: loc.start.line,
            }
            data.fnMap[functions] = mapping
            data.f[functions++] = starts.length ? Math.max(...starts) : hits(node.body)
        }
        if (ts.isIfStatement(node)) {
            const then = hits(node.thenStatement)
            const otherwise = node.elseStatement
                ? hits(node.elseStatement)
                : Math.max(0, hits(node) - then)
            branch("if", node, [node.thenStatement, node.elseStatement ?? node], [then, otherwise])
        } else if (ts.isConditionalExpression(node))
            branch(
                "cond-expr",
                node,
                [node.whenTrue, node.whenFalse],
                [hits(node.whenTrue), hits(node.whenFalse)]
            )
        else if (isLogical(node) && !isLogical(parentExpression(node))) {
            const arms = leaves(node)
            branch("binary-expr", node, arms, arms.map(hits))
        } else if (ts.isSwitchStatement(node)) {
            const clauses = node.caseBlock.clauses
            branch(
                "switch",
                node,
                [...clauses],
                clauses.map((clause) => (clause.statements.length ? hits(clause.statements[0]) : 0))
            )
        } else if ((ts.isParameter(node) || ts.isBindingElement(node)) && node.initializer)
            branch("default-arg", node.initializer, [node.initializer], [hits(node.initializer)])
        ts.forEachChild(node, visit)
    }
    visit(file)
    return data
}

/** Istanbul coverage of `<package>/src/**` TypeScript from the journeys' merged V8 results. */
export async function journeyCoverage(packageDir: string, workers: ProcessCov[]) {
    const executed = new Map(
        mergeProcessCovs(workers).result.map((script) => [
            fileURLToPath(script.url),
            script.functions,
        ])
    )
    const src = join(packageDir, "src") + sep
    const counted = (path: string) => path.startsWith(src) && /\.tsx?$/.test(path)
    const coverage: CoverageMap = libCoverage.createCoverageMap({})
    const typescript = typescriptMaps(packageDir)
    const dist = join(packageDir, "dist")
    let unloaded = 0
    let unverified = 0
    for (const chunk of await readdir(dist, {recursive: true})) {
        const file = join(dist, chunk)
        if (!file.endsWith(".js") || !existsSync(`${file}.map`)) continue
        const map = absoluteSources(readJson<EncodedSourceMap>(`${file}.map`), packageDir)
        if (!map.sources.some((source) => source && counted(source))) continue
        const bundle = bundleModules(await readFile(file, "utf8"), map, counted, typescript)
        unverified += bundle.unverified
        const functions = executed.get(file)
        if (!functions) unloaded++
        const counts = functions ? blockCounts(functions) : never
        for (const [path, module] of bundle.modules)
            coverage.addFileCoverage(fileCoverage(path, module, counts))
    }
    return {coverage, unloaded, unverified}
}
