// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"
import {SigningAction} from "@/lib/signing/types"
import {actionDescription} from "./format"

const t = ((key: string) => key) as unknown as TFunction

describe("actionDescription", () => {
    it("says Close voting seals the ballot boxes at an event with Seal at close", () => {
        expect(actionDescription(t, SigningAction.CloseVoting, true)).toBe(
            "signing.actions.close-voting.descriptionSealed"
        )
    })

    it("keeps the plain description at an event that doesn't seal", () => {
        expect(actionDescription(t, SigningAction.CloseVoting, false)).toBe(
            "signing.actions.close-voting.description"
        )
        expect(actionDescription(t, SigningAction.CloseVoting)).toBe(
            "signing.actions.close-voting.description"
        )
    })

    it("never claims a seal for another action", () => {
        expect(actionDescription(t, SigningAction.OpenVoting, true)).toBe(
            "signing.actions.open-voting.description"
        )
    })
})
