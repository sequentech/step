// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {getLogMessageField, getLogMessageHeadField, parseLogMessage} from "./parseLogMessage"

describe("parseLogMessage", () => {
    it("returns the object for a valid JSON object message", () => {
        expect(parseLogMessage('{"user_id":"u1","username":"alice"}')).toEqual({
            user_id: "u1",
            username: "alice",
        })
    })

    it("returns null for an empty message", () => {
        expect(parseLogMessage("")).toBeNull()
    })

    it("returns null for a non-JSON message", () => {
        expect(parseLogMessage("not json")).toBeNull()
    })

    it("returns null for JSON that is not an object", () => {
        expect(parseLogMessage("42")).toBeNull()
        expect(parseLogMessage('"text"')).toBeNull()
        expect(parseLogMessage("null")).toBeNull()
        expect(parseLogMessage("[1,2]")).toBeNull()
    })

    it("returns null for a missing message", () => {
        expect(parseLogMessage(undefined)).toBeNull()
        expect(parseLogMessage(null)).toBeNull()
    })
})

describe("getLogMessageField", () => {
    it("returns string and number fields", () => {
        const message = parseLogMessage('{"user_id":"u1","count":3}')
        expect(getLogMessageField(message, "user_id")).toBe("u1")
        expect(getLogMessageField(message, "count")).toBe(3)
    })

    it("returns null for missing, nested or unparsable values", () => {
        expect(getLogMessageField(parseLogMessage('{"a":{"b":1}}'), "a")).toBeNull()
        expect(getLogMessageField(parseLogMessage('{"a":1}'), "user_id")).toBeNull()
        expect(getLogMessageField(parseLogMessage("not json"), "user_id")).toBeNull()
    })
})

describe("getLogMessageHeadField", () => {
    it("returns the field from statement.head", () => {
        const message = parseLogMessage('{"statement":{"head":{"kind":"Vote"}}}')
        expect(getLogMessageHeadField(message, "kind")).toBe("Vote")
    })

    it("returns null when statement or head is missing or not an object", () => {
        expect(getLogMessageHeadField(parseLogMessage("{}"), "kind")).toBeNull()
        expect(getLogMessageHeadField(parseLogMessage('{"statement":"x"}'), "kind")).toBeNull()
        expect(
            getLogMessageHeadField(parseLogMessage('{"statement":{"head":null}}'), "kind")
        ).toBeNull()
        expect(getLogMessageHeadField(parseLogMessage(""), "kind")).toBeNull()
    })
})
