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
import {ETasksExecution} from "@/types/tasksExecution"
import {IPermissions} from "@/types/keycloak"

const languages = {cat, en, es, eu, fr, gl, nl, tl}

describe("electoral-log audit labels", () => {
    it.each(Object.entries(languages))("are translated in %s", (_, language) => {
        const {logsScreen, tasksScreen, usersAndRolesScreen} = language.translations
        const labels = [
            logsScreen.actions.audit,
            logsScreen.auditDialog.title,
            logsScreen.auditDialog.confirm,
            logsScreen.auditDialog.description,
            tasksScreen.tasksExecution[ETasksExecution.AUDIT_ELECTORAL_LOG],
            usersAndRolesScreen.permissions[IPermissions.ELECTORAL_LOG_AUDIT],
        ]
        for (const label of labels) {
            expect(typeof label).toBe("string")
            expect(label.trim()).not.toHaveLength(0)
        }
    })
})
