/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderHook} from "@testing-library/react"
import i18next, {type i18n as I18n} from "i18next"
import uiCoreEnglish from "../../../ui-core/src/translations/en"
import {zoneLabel} from "../../../ui-core/src/services/timeZones"
import {useZonedTime} from "./useZonedTime"
import {EventTimeZoneProvider, MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../ui-core/src/services/timeZones"),
    ...jest.requireActual("../../../ui-core/src/services/eventTimeZones"),
    ...jest.requireActual("../../../ui-core/src/types/ElectionEventPresentation"),
}))

let mockI18n: I18n
jest.mock("react-i18next", () => ({useTranslation: () => ({t: mockI18n.t, i18n: mockI18n})}))

beforeAll(async () => {
    mockI18n = i18next.createInstance()
    await mockI18n.init({
        lng: "en",
        ns: ["translations"],
        defaultNS: "translations",
        interpolation: {escapeValue: false},
        showSupportNotice: false,
        resources: {en: {translations: JSON.parse(JSON.stringify(uiCoreEnglish.translations))}},
    })
})

const MADRID = "Europe/Madrid"
const event = {id: "event", presentation: {timezones: {configured: [MADRID], primary: MADRID}}}
const wrapper = ({children}: {children: React.ReactNode}) => (
    <MyTimeZoneProvider zone={MADRID}>
        <EventTimeZoneProvider event={event}>{children}</EventTimeZoneProvider>
    </MyTimeZoneProvider>
)
const label = (instant: string) => zoneLabel(MADRID, {t: mockI18n.t, lang: "en"}, new Date(instant))

it("shows only the time, in the event's zone, on the same day", () => {
    const deadline = "2028-03-13T17:15:00Z"
    const {result} = renderHook(() => useZonedTime(new Date("2028-03-13T17:05:00Z")), {wrapper})
    expect(result.current(deadline)).toBe(`6:15 PM ${label(deadline)}`)
    expect(result.current(null)).toBe("")
})

it("adds the date when the time is on another day in the zone", () => {
    const closed = "2028-03-13T17:00:00Z"
    const {result} = renderHook(() => useZonedTime(new Date("2028-03-14T09:00:00Z")), {wrapper})
    expect(result.current(closed)).toBe(`Mar 13, 2028, 6:00 PM ${label(closed)}`)
})
