// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ssmlSegments} from "./ssml"

describe("ssmlSegments", () => {
    it("reads the words out of the Lambda's SSML, with each part's language", () => {
        expect(
            ssmlSegments(
                '<speak><lang xml:lang="en-US">For English, press 1</lang>, ' +
                    '<lang xml:lang="es-ES">Para Español, pulse 2</lang></speak>'
            )
        ).toEqual([
            {kind: "text", text: "For English, press 1", lang: "en-US"},
            {kind: "text", text: ", "},
            {kind: "text", text: "Para Español, pulse 2", lang: "es-ES"},
        ])
    })

    it("keeps the pauses, as pauses rather than markup", () => {
        expect(ssmlSegments('<speak>Press one<break time="500ms"/>or two</speak>')).toEqual([
            {kind: "text", text: "Press one"},
            {kind: "break", time: "500ms"},
            {kind: "text", text: "or two"},
        ])
    })

    it("shows what is written inside any other element, and decodes entities", () => {
        expect(
            ssmlSegments(
                '<speak><prosody rate="slow">Q&amp;A <say-as interpret-as="digits">12</say-as>' +
                    '</prosody> <sub alias="Sequent">SQ</sub></speak>'
            )
        ).toEqual([
            {kind: "text", text: "Q&A "},
            {kind: "text", text: "12"},
            {kind: "text", text: " "},
            {kind: "text", text: "SQ"},
        ])
    })

    it("treats markup the parser does not accept as text without its tags", () => {
        expect(ssmlSegments("<speak>Q&A <lang xml:lang='es-ES'>sí</lang></speak>")).toEqual([
            {kind: "text", text: "Q&A sí"},
        ])
        expect(ssmlSegments("plain words")).toEqual([{kind: "text", text: "plain words"}])
    })
})
