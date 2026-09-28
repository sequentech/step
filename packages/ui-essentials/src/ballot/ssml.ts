// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** A piece of a spoken prompt, as a transcript shows it. */
export type SsmlSegment =
    | {kind: "text"; text: string; lang?: string}
    | {kind: "break"; time?: string}

const NAMED_ENTITIES: Record<string, string> = {amp: "&", lt: "<", gt: ">", quot: '"', apos: "'"}

/** XML's own entity and character references; anything else, a bare `&` included, stays. */
function decodeEntities(text: string): string {
    return text.replace(/&(#x[0-9a-f]+|#[0-9]+|[a-z]+);/gi, (whole, name: string) => {
        if (name[0] !== "#") return NAMED_ENTITIES[name] ?? whole
        const code =
            name[1] === "x" || name[1] === "X" ? parseInt(name.slice(2), 16) : Number(name.slice(1))
        return code > 0 && code <= 0x10ffff ? String.fromCodePoint(code) : whole
    })
}

const TEXT_NODE = 3
const ELEMENT_NODE = 1
const CDATA_SECTION_NODE = 4

/**
 * The words of an SSML prompt, for display: what the text-to-speech would read, the
 * language each part is read in (`<lang xml:lang>`, or `xml:lang` on any element),
 * and where it pauses (`<break>`).
 *
 * The prompt is parsed as XML and only its text is kept, so nothing in it ever
 * becomes markup in the page. Every other element (`prosody`, `say-as`, `sub`, …)
 * shows what is written inside it. A prompt the XML parser refuses, such as one
 * with a bare `&`, is shown as its text with the tags taken out.
 */
export function ssmlSegments(ssml: string): SsmlSegment[] {
    const source = /^\s*(<\?xml[^>]*\?>\s*)?<speak[\s>]/.test(ssml)
        ? ssml
        : `<speak>${ssml}</speak>`
    const document =
        typeof DOMParser === "undefined"
            ? undefined
            : new DOMParser().parseFromString(source, "application/xml")
    if (!document || document.getElementsByTagName("parsererror").length > 0) {
        const text = decodeEntities(ssml.replace(/<[^>]*>/g, ""))
        return text ? [{kind: "text", text}] : []
    }

    const segments: SsmlSegment[] = []
    const walk = (node: Node, lang: string | undefined): void => {
        for (const child of Array.from(node.childNodes)) {
            if (child.nodeType === TEXT_NODE || child.nodeType === CDATA_SECTION_NODE) {
                const text = child.nodeValue ?? ""
                if (text) segments.push(lang ? {kind: "text", text, lang} : {kind: "text", text})
            } else if (child.nodeType === ELEMENT_NODE) {
                const element = child as Element
                if (element.localName === "break") {
                    const time = element.getAttribute("time")
                    segments.push(time ? {kind: "break", time} : {kind: "break"})
                } else {
                    walk(element, element.getAttribute("xml:lang") || lang)
                }
            }
        }
    }
    const root = document.documentElement
    walk(root, root.getAttribute("xml:lang") || undefined)
    return segments
}
