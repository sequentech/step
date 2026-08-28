// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactNode} from "react"
import sanitizeHtml from "sanitize-html"
import parse from "html-react-parser"

// Marks up language and direction changes inside a passage. Without these, a
// quotation in another language is read with the wrong pronunciation rules and
// right-to-left text cannot be ordered correctly (WCAG 1.3.1).
//
// Deliberately narrow: `id`, `role` and `aria-*` are NOT allowed globally.
// Election configuration is authored content, and letting it set those would let
// it collide with the ids the portal uses for its own labelling, redeclare an
// element's role, or hide visible text from assistive technology with
// `aria-hidden` — including the security-confirmation text that labels the
// start-screen checkbox.
const GLOBAL_ATTRIBUTES = ["lang", "dir"]

// Header/data-cell relationships in admin-authored tables. `id` is permitted
// here only so that `headers` can reference a cell.
const TABLE_CELL_ATTRIBUTES = ["scope", "colspan", "rowspan", "headers", "abbr", "id"]

export const stringToHtml = (html: string): ReactNode =>
    parse(
        sanitizeHtml(html, {
            allowedAttributes: {
                "*": GLOBAL_ATTRIBUTES,
                "a": ["href", "class", "target", "name", "title"],
                "th": TABLE_CELL_ATTRIBUTES,
                "td": TABLE_CELL_ATTRIBUTES,
            },
        })
    )
