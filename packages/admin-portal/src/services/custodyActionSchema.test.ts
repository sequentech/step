// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {readFileSync} from "node:fs"
import {resolve} from "node:path"
import {parse, visit} from "graphql"

const sdlPath = resolve(__dirname, "../../../../hasura/metadata/actions.graphql")
const schemaPath = resolve(__dirname, "../../graphql.schema.json")

function missingTypes(sdl: string): string[] {
    const document = parse(sdl)
    const schema = JSON.parse(readFileSync(schemaPath, "utf8")) as {
        __schema: {types: Array<{kind: string; name: string}>}
    }
    const defined = new Set(["String", "Boolean", "Int", "Float", "ID"])
    for (const type of schema.__schema.types) {
        if (type.kind === "SCALAR") defined.add(type.name)
    }
    for (const definition of document.definitions) {
        if ("name" in definition && definition.name) defined.add(definition.name.value)
    }
    const missing = new Set<string>()
    visit(document, {
        NamedType(node) {
            if (!defined.has(node.name.value)) missing.add(node.name.value)
        },
    })
    return Array.from(missing).sort()
}

test("every Hasura action references a declared type or a database scalar", () => {
    expect(missingTypes(readFileSync(sdlPath, "utf8"))).toEqual([])
})

test("a dangling legacy outcome enum is rejected before fresh deployment", () => {
    const sdl = readFileSync(sdlPath, "utf8")
    const broken = sdl.replace(
        "type AuthorizeTallyCustodyOutput {",
        "type AuthorizeTallyCustodyOutput {\n    outcome: UndefinedLegacyCustodyOutcome!"
    )
    expect(missingTypes(broken)).toEqual(["UndefinedLegacyCustodyOutcome"])
})
