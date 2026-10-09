// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {parse, Root} from "postcss"

export interface PresentationCssScope {
    // Base URL that relative url() values resolve against (the page's base URI).
    baseUrl: string
    // Bucket URL whose files may be referenced in addition to the page origin.
    publicBucketUrl?: string
}

const ALLOWED_AT_RULES = new Set([
    "media",
    "supports",
    "container",
    "layer",
    "keyframes",
    "-webkit-keyframes",
])

// Properties whose value can render text next to page content.
const GENERATED_TEXT_PROPERTIES = new Set([
    "content",
    "quotes",
    "list-style",
    "list-style-type",
    "text-overflow",
    "text-emphasis",
    "text-emphasis-style",
    "hyphenate-character",
])
const VENDOR_PREFIX = /^-(webkit|moz|ms|o)-/
const EMPTY_STRING = /""|''/g
const TEXT_SOURCE = /["']|var\(/

const URL_FUNCTION = "url("
const URL_ARGUMENT = /^\s*(?:"([^"\n\r\f]*)"|'([^'\n\r\f]*)'|([^\s"'()]*))\s*\)/
// Without escapes, block comments and control characters, the browser and the style
// engine split strings and function names exactly where this scanner does.
const FORBIDDEN_ANYWHERE = ["\\", "/*"]
// eslint-disable-next-line no-control-regex
const CONTROL_CHARACTER = /[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/
// A line comment for the style engine, and the rules and functions that take a URL as a string.
const FORBIDDEN_OUTSIDE_STRINGS = ["//", "@import", "image-set(", "image(", "src("]
// Emotion appends the name in "label:<name>;" text to the generated class name.
const EMOTION_LABEL = /label:\s*([^\s;{]+)\s*(;|$)/g
const CLASS_NAME_PART = /^[\w-]+$/
const QUOTES = ['"', "'"]
const ASCII_UPPER_CASE = /[A-Z]/g
const LINE_BREAK = /[\n\r\f]/
const DATA_PROTOCOL = "data:"
const HTTP_PROTOCOLS = ["http:", "https:"]

type UrlCheck = (url: string) => boolean

// CSS names are ASCII case-insensitive; unlike toLowerCase() this keeps every index in place.
const asciiLowerCase = (text: string): string =>
    text.replace(ASCII_UPPER_CASE, (char) => char.toLowerCase())

const parseUrl = (value: string, base?: string): URL | null => {
    try {
        return new URL(value, base)
    } catch {
        return null
    }
}

const urlCheck = ({baseUrl, publicBucketUrl}: PresentationCssScope): UrlCheck => {
    const pageOrigin = parseUrl(baseUrl)?.origin
    const bucketHref = publicBucketUrl ? parseUrl(publicBucketUrl, baseUrl)?.href : undefined
    const bucketPrefix = bucketHref && (bucketHref.endsWith("/") ? bucketHref : `${bucketHref}/`)

    return (value) => {
        const url = parseUrl(value, baseUrl)
        if (url === null) {
            return false
        }
        if (url.protocol === DATA_PROTOCOL) {
            return true
        }
        return (
            HTTP_PROTOCOLS.includes(url.protocol) &&
            (url.origin === pageOrigin || (!!bucketPrefix && url.href.startsWith(bucketPrefix)))
        )
    }
}

// Rewrites every url() in the text as url("..."). Returns null when a url() points
// elsewhere or the text holds anything else that could load a resource.
const rewriteUrls = (text: string, isAllowed: UrlCheck): string | null => {
    if (
        CONTROL_CHARACTER.test(text) ||
        FORBIDDEN_ANYWHERE.some((sequence) => text.includes(sequence))
    ) {
        return null
    }
    const lower = asciiLowerCase(text)
    let result = ""
    let i = 0
    while (i < text.length) {
        const char = text[i]
        if (QUOTES.includes(char)) {
            const end = text.indexOf(char, i + 1)
            if (end < 0 || LINE_BREAK.test(text.slice(i, end))) {
                return null
            }
            result += text.slice(i, end + 1)
            i = end + 1
        } else if (lower.startsWith(URL_FUNCTION, i)) {
            const match = URL_ARGUMENT.exec(text.slice(i + URL_FUNCTION.length))
            if (match === null) {
                return null
            }
            const url = match[1] ?? match[2] ?? match[3]
            if (url.includes('"') || !isAllowed(url)) {
                return null
            }
            result += `${URL_FUNCTION}"${url}")`
            i += URL_FUNCTION.length + match[0].length
        } else if (FORBIDDEN_OUTSIDE_STRINGS.some((sequence) => lower.startsWith(sequence, i))) {
            return null
        } else {
            result += char
            i += 1
        }
    }
    return result
}

const changesClassName = (text: string): boolean =>
    Array.from(text.matchAll(EMOTION_LABEL)).some((match) => !CLASS_NAME_PART.test(match[1]))

const showsText = (property: string, value: string): boolean =>
    GENERATED_TEXT_PROPERTIES.has(asciiLowerCase(property).replace(VENDOR_PREFIX, "")) &&
    TEXT_SOURCE.test(asciiLowerCase(value.replace(EMPTY_STRING, "")))

// Keeps the styling in election event CSS while dropping what could load resources
// from other origins or replace the text of the page.
export const sanitizePresentationCss = (css: string, scope: PresentationCssScope): string => {
    let root: Root
    try {
        root = parse(css)
    } catch {
        return ""
    }
    const isAllowed = urlCheck(scope)

    root.walk((node) => {
        if (node.type === "comment") {
            node.remove()
        } else if (node.type === "atrule") {
            const params = rewriteUrls(node.params, isAllowed)
            if (!ALLOWED_AT_RULES.has(asciiLowerCase(node.name)) || params === null) {
                node.remove()
                return
            }
            node.params = params
            delete node.raws.params
        } else if (node.type === "rule") {
            const selector = rewriteUrls(node.selector, isAllowed)
            if (selector === null) {
                node.remove()
                return
            }
            node.selector = selector
            delete node.raws.selector
        } else if (node.type === "decl") {
            const value = rewriteUrls(node.value, isAllowed)
            if (
                value === null ||
                rewriteUrls(node.prop, isAllowed) === null ||
                showsText(node.prop, node.value) ||
                changesClassName(`${node.prop}:${value};`)
            ) {
                node.remove()
                return
            }
            node.value = value
            delete node.raws.value
            delete node.raws.between
            delete node.raws.important
        }
    })

    const sanitized = root.toString()
    return rewriteUrls(sanitized, isAllowed) === null || changesClassName(sanitized)
        ? ""
        : sanitized
}
