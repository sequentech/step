// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ReactNode} from "react"
import sanitizeHtml from "sanitize-html"
import parse from "html-react-parser"

const SANITIZE_OPTIONS: sanitizeHtml.IOptions = {
    allowedAttributes: {
        a: ["href", "class", "target", "name", "title"],
    },
}

/**
 * Escapes every `<` after the last `>` in the string. No `>` can close them, so
 * they cannot open a tag, yet the parser still consumes the rest of the string
 * looking for one and discards it -- `"a <b and c"` would render as `"a "`.
 *
 * Nothing in the escaped run could have become an element in the first place: a
 * start tag needs a `>`, and there is none left. Everything at or before the last
 * `>` is untouched -- which covers every `<` inside a quoted attribute of a tag
 * that actually closes -- so no tag that used to parse stops parsing, and no tag
 * that was being swallowed as another tag's attributes gets freed. That this also
 * runs before sanitize-html, and so can never widen the allowlist, is only the
 * backstop.
 *
 * The invariant is *not* "escaping only removes markup": escaping an earlier `<`
 * can re-tokenize a later one into a real element.
 */
const escapeStrayAngleBrackets = (html: string): string => {
    const lastClose = html.lastIndexOf(">")
    return (
        html.slice(0, lastClose + 1) +
        html
            .slice(lastClose + 1)
            .split("<")
            .join("&lt;")
    )
}

/**
 * Renders a translation, or any other localized string, as sanitized HTML.
 *
 * sanitize-html keeps its default `allowedTags`, which include block elements
 * such as `p`, `ul` and `h1`-`h6`. MUI maps the `body1`, `body2` and `inherit`
 * Typography variants to a `<p>`, so a `Typography` rendering the result of this
 * function must pass `component="div"`: block markup inside a paragraph is
 * invalid nesting, and the browser closes the paragraph early, dropping the MUI
 * styling from the content. Containers that already render a `div` -- `WarnBox`,
 * `Alert`, `Dialog`, `styled(Box)` -- need nothing.
 *
 * See `translateHtml` when the translation interpolates values, and
 * `stringToText` when the result is needed as a plain string.
 */
export const stringToHtml = (html: string): ReactNode =>
    parse(sanitizeHtml(escapeStrayAngleBrackets(html), SANITIZE_OPTIONS))

/**
 * Reduces a localized string to plain text, for the places that render one into
 * a DOM attribute -- a placeholder, a title -- where markup cannot be rendered
 * and would otherwise show up as literal tags. Parsing the sanitized output
 * decodes the entities that sanitize-html introduces.
 *
 * The result is decoded text, not a sanitized fragment: an escaped `&lt;` in the
 * input comes back as a literal `<`. Render it as text only -- passing it back
 * into `stringToHtml`, or any other HTML sink, would re-arm the markup it
 * decoded.
 */
export const stringToText = (html: string): string => {
    const text = parse(
        sanitizeHtml(escapeStrayAngleBrackets(html), {allowedTags: [], allowedAttributes: {}})
    )
    return typeof text === "string" ? text : ""
}

const HTML_ESCAPES: Record<string, string> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
}

export const escapeHtml = (value: string): string =>
    value.replace(/[&<>"']/g, (character) => HTML_ESCAPES[character])

/**
 * The value types a translation may interpolate. Deliberately excludes objects
 * and arrays: i18next resolves dotted placeholders such as `{{user.name}}`
 * against nested objects, and treats a `replace` object as the interpolation
 * source, so either would slip past a shallow escape.
 */
export type TranslationValue = string | number | boolean | null | undefined

export type TranslationValues = Record<string, TranslationValue>

export type TranslateFunction = (key: string, values?: TranslationValues) => string

/**
 * Escapes the values interpolated into a translation that is rendered as HTML.
 *
 * Translations are client-editable localization overrides and may legitimately
 * contain markup, but the values interpolated into them are data and must never
 * become markup. i18next's `escapeValue` cannot enforce that: a translation
 * using the `{{- value}}` syntax opts out of escaping, so whoever writes the
 * override also decides whether the value is escaped. Escaping here instead
 * leaves nothing for the translation to opt out of.
 *
 * Numbers and booleans pass through untouched so that plurals and `count` keep
 * working; anything else is escaped as the string i18next would interpolate.
 *
 * Use this when the translated string is built away from the site that renders
 * it; use `translateHtml` when it is translated and rendered together.
 */
export const escapeTranslationValues = (values: TranslationValues): TranslationValues =>
    Object.fromEntries(
        Object.entries(values).map(([name, value]) => [
            name,
            value === null ||
            value === undefined ||
            typeof value === "number" ||
            typeof value === "boolean"
                ? value
                : escapeHtml(String(value)),
        ])
    )

/**
 * Renders a translation as HTML. Use this instead of `stringToHtml(t(...))`
 * whenever the translation interpolates values; see `escapeTranslationValues`.
 */
export const translateHtml = (
    t: TranslateFunction,
    key: string,
    values?: TranslationValues
): ReactNode => stringToHtml(values ? t(key, escapeTranslationValues(values)) : t(key))
