// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {createLocalValidator} from "./localValidator"
import {EMonitoringConfigKind, EMonitoringProblemSeverity} from "./types"

describe("createLocalValidator", () => {
    it("is unavailable when sequent-core has no monitoring export", () => {
        expect(createLocalValidator({}, EMonitoringConfigKind.WIDGET)("id: a")).toBeNull()
    })

    it("passes the kind, the key, the text and the event's documents to the export", () => {
        const validateMonitoringConfig = jest.fn(() => ({
            problems: [{severity: "warning", code: "unused_selector", path: "", message: "m"}],
        }))
        const validate = createLocalValidator(
            {validateMonitoringConfig},
            EMonitoringConfigKind.DASHBOARD,
            {key: "overview", configSet: {widgets: {}}}
        )
        expect(validate("id: a")).toEqual([
            expect.objectContaining({severity: EMonitoringProblemSeverity.WARNING}),
        ])
        expect(validateMonitoringConfig).toHaveBeenCalledWith(
            "dashboard",
            "overview",
            "id: a",
            '{"widgets":{}}'
        )
    })

    it("sends an empty set when there are no other documents", () => {
        const validateMonitoringConfig = jest.fn(() => ({problems: []}))
        createLocalValidator({validateMonitoringConfig}, EMonitoringConfigKind.THEME)("id: t")
        expect(validateMonitoringConfig).toHaveBeenCalledWith("theme", "", "id: t", "")
    })
})
