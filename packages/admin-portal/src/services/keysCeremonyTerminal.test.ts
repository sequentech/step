// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IKeysCeremonyExecutionStatus as EStatus, isKeysCeremonyTerminal} from "./KeyCeremony"

describe("isKeysCeremonyTerminal", () => {
    it.each([EStatus.SUCCESS, EStatus.CANCELLED])("recognizes terminal status %s", (status) => {
        expect(isKeysCeremonyTerminal(status)).toBe(true)
    })

    it.each([EStatus.USER_CONFIGURATION, EStatus.STARTED, EStatus.IN_PROGRESS])(
        "rejects active status %s",
        (status) => {
            expect(isKeysCeremonyTerminal(status)).toBe(false)
        }
    )
})
