// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {TFunction} from "i18next"
import i18next from "i18next"
import en from "@/translations/en"
import {worded} from "./SigningSubject"

let t: TFunction

beforeAll(async () => {
    const instance = i18next.createInstance()
    await instance.init({lng: "en", resources: {en}, interpolation: {escapeValue: false}})
    t = instance.getFixedT("en", "translations")
})

describe("a signed value in the organization's words", () => {
    it("names each channel", () => {
        expect(worded(t, "channels", ["ONLINE", "TELEPHONE"], "ONLINE, TELEPHONE")).toBe(
            "Online, Telephone"
        )
    })

    it("names each channel's status before, not CHANNEL=STATUS", () => {
        expect(worded(t, "from", ["ONLINE=OPEN"], "ONLINE=OPEN")).toBe("Online: Open")
        expect(
            worded(t, "from", ["ONLINE=PAUSED", "TELEPHONE=NOT_STARTED"], "ONLINE=PAUSED, …")
        ).toBe("Online: Paused, Telephone: Not started")
    })

    it("keeps a code it has no words for as signed", () => {
        expect(worded(t, "from", ["SMOKE=SIGNAL"], "SMOKE=SIGNAL")).toBe("SMOKE: SIGNAL")
        expect(worded(t, "channels", ["SMOKE"], "SMOKE")).toBe("SMOKE")
        expect(worded(t, "count", 3, "3")).toBe("3")
    })
})
