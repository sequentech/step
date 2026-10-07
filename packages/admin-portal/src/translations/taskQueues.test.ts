// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import cat from "./cat"
import en from "./en"
import es from "./es"
import eu from "./eu"
import fr from "./fr"
import gl from "./gl"
import nl from "./nl"
import tl from "./tl"
import {IPermissions} from "@/types/keycloak"
import {GRAPH_PERIODS, MESSAGE_STATES, OUTCOMES} from "@/services/TaskQueues"

const languages = {cat, en, es, eu, fr, gl, nl, tl}

const leaves = (value: unknown, path = ""): Array<[string, unknown]> =>
    typeof value === "object" && value !== null
        ? Object.entries(value).flatMap(([key, child]) =>
              leaves(child, path ? `${path}.${key}` : key)
          )
        : [[path, value]]

describe("task-queue labels", () => {
    const english = leaves(en.translations.taskQueues).map(([path]) => path)

    it("cover every outcome, graph period and message state", () => {
        for (const outcome of OUTCOMES) {
            expect(english).toContain(`outcomes.${outcome}`)
        }
        for (const period of GRAPH_PERIODS) {
            expect(english).toContain(`graphs.periods.${period}`)
        }
        for (const state of MESSAGE_STATES) {
            expect(english).toContain(`messages.states.${state}`)
        }
    })

    it.each(Object.entries(languages))("are complete in %s", (_, language) => {
        const {taskQueues, sideMenu, usersAndRolesScreen} = language.translations
        const labels = leaves(taskQueues)
        expect(labels.map(([path]) => path)).toEqual(english)
        for (const label of [
            ...labels.map(([, label]) => label),
            sideMenu.taskQueues,
            usersAndRolesScreen.permissions[IPermissions.TASK_QUEUES_READ],
            usersAndRolesScreen.permissions[IPermissions.TASK_QUEUES_WRITE],
        ]) {
            expect(typeof label).toBe("string")
            expect((label as string).trim()).not.toHaveLength(0)
        }
    })

    it.each(Object.entries(languages))("keep their placeholders in %s", (_, language) => {
        const placeholders = (text: unknown) =>
            Array.from(String(text).matchAll(/{{(\w+)}}/g), (match) => match[1]).sort()
        const translated = new Map(leaves(language.translations.taskQueues))
        for (const [path, label] of leaves(en.translations.taskQueues)) {
            expect([path, placeholders(translated.get(path))]).toEqual([path, placeholders(label)])
        }
    })
})
