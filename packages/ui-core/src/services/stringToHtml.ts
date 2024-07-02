// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactNode} from "react"
import sanitizeHtml from "sanitize-html"
import parse from "html-react-parser"

export const stringToHtml = (html: string): ReactNode =>
    parse(
        sanitizeHtml(html, {
            allowedAttributes: {
                a: ["href", "class", "target", "name", "title"],
            },
        })
    )
