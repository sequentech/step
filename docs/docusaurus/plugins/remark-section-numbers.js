// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const fs = require("node:fs");
const path = require("node:path");
const { createSlugger, parseMarkdownHeadingId } = require("@docusaurus/utils");

const site = path.resolve(__dirname, "..");

function headingsOf(tree) {
  const headings = [];
  (function collect(node) {
    if (node.type === "heading") headings.push(node);
    for (const child of node.children || []) collect(child);
  })(tree);
  return headings;
}

/** The ids of the Docusaurus headings plugin, from the unnumbered texts. */
function headingIds(headings, toString) {
  const slugs = createSlugger();
  return headings.map((heading) => {
    const preset = heading.data?.hProperties?.id;
    if (preset) return slugs.slug(preset, { maintainCase: true });
    const parts = heading.children.filter(({ type }) => !["html", "jsx"].includes(type));
    const text = toString(parts.length > 0 ? parts : heading);
    const explicit = parseMarkdownHeadingId(text);
    if (!explicit.id) return slugs.slug(text, { maintainCase: false });
    // Docusaurus strips {#id} from the text only when no id is preset.
    const last = heading.children[heading.children.length - 1];
    if (heading.children.length > 1) {
      const rest = parseMarkdownHeadingId(last.value).text;
      if (rest) last.value = rest;
      else heading.children.pop();
    } else last.value = explicit.text;
    return explicit.id;
  });
}

function headingNumbers(headings, chapter) {
  const counters = [];
  return headings.map((heading) => {
    counters.length = Math.min(counters.length, heading.depth - 1);
    while (counters.length < heading.depth - 1) counters.push(0);
    counters[heading.depth - 2]++;
    return [chapter, ...counters].join(".");
  });
}

const sameText = (a, b) => {
  const plain = (text) => text.trim().replace(/\s+/g, " ").toLowerCase();
  return plain(a) === plain(b);
};

/**
 * Number the chapters and sections of the manuals, so that readers can refer
 * to them by number. `documents` maps each manual to its pages in chapter
 * order, as paths relative to the site. A listed page gets the title
 * "N Title", N being its chapter and Title its front matter title, and its
 * h2, h3 and h4 read "N.M Text", "N.M.K Text" and "N.M.K.L Text". Other
 * pages stay as they are.
 *
 * Registered before the Docusaurus remark plugins: the inserted h1 becomes
 * the content title, which the theme renders instead of its own title, and
 * the table of contents shows the numbered headings. The page metadata
 * (sidebar label, <title>) keeps the front matter title.
 *
 * Anchors stay as they were. Each heading gets the id that Docusaurus derives
 * from its unnumbered text, explicit {#id} included, in the same slugger
 * sequence (with the default markdown.anchors.maintainCase: false). The id is
 * preset, so the Docusaurus headings plugin keeps it; the h1 takes an id that
 * no heading has, since Docusaurus would otherwise suffix a later one.
 *
 * A link from a listed page to a listed page names its target by number.
 * Text that is the target's heading gets the number in front ("3.5 Checking
 * the version"); text that is the chapter title of a link to a section becomes
 * that section's number and heading; any other text gets the number after it
 * ("login checks (section 8.4)"). A target in another manual also names it
 * ("AGD-04 7.4 Configure the realms", "(AGD-04 section 7.4)"). A link to the
 * first page of another manual, without a section, names that manual: its text
 * stays.
 */
module.exports = function remarkSectionNumbers({
  documents = {},
  read = (file) => fs.readFileSync(file, "utf8"),
} = {}) {
  const chapters = new Map();
  for (const [manual, pages] of Object.entries(documents))
    pages.forEach((page, index) => {
      if (chapters.has(page))
        throw new Error(`Section numbering lists ${page} twice.`);
      chapters.set(page, { manual, chapter: index + 1 });
    });

  /** The numbered outline of a page: its chapter title, and number and text by anchor. */
  function outlineOf(title, chapter, headings, ids, toString) {
    const numbers = headingNumbers(headings, chapter);
    const sections = new Map();
    headings.forEach((heading, index) =>
      sections.set(ids[index], { number: numbers[index], text: toString(heading) }),
    );
    return { title, chapter: String(chapter), sections };
  }

  const pages = new Map();
  async function outlineOfPage(page, toString) {
    const source = read(path.join(site, page));
    const known = pages.get(page);
    if (known?.source === source) return known.outline;
    const { unified } = await import("unified");
    const { default: remarkParse } = await import("remark-parse");
    const { default: remarkFrontmatter } = await import("remark-frontmatter");
    const tree = unified().use(remarkParse).use(remarkFrontmatter).parse(source);
    const yaml = tree.children.find(({ type }) => type === "yaml")?.value ?? "";
    const title = /^title:\s*(.*)$/m.exec(yaml)?.[1].trim().replace(/^(["'])(.*)\1$/, "$2") ?? "";
    const headings = headingsOf(tree);
    const ids = headingIds(headings, toString);
    const outline = outlineOf(title, chapters.get(page).chapter, headings, ids, toString);
    pages.set(page, { source, outline });
    return outline;
  }

  return async (tree, file) => {
    const document = path
      .relative(path.resolve(__dirname, ".."), file.path || "document")
      .split(path.sep)
      .join("/");
    if (!chapters.has(document)) return;
    const { manual, chapter } = chapters.get(document);
    const title = file.data.frontMatter?.title;
    if (typeof title !== "string" || !title.trim())
      file.fail("A numbered chapter needs a front matter title.");
    const headings = headingsOf(tree);
    const own = headings.find((heading) => heading.depth === 1);
    if (own)
      file.fail("A numbered chapter takes its h1 from the front matter title; remove this h1.", own);

    const { toString } = await import("mdast-util-to-string");
    const ids = headingIds(headings, toString);
    const base = createSlugger().slug(`${chapter} ${title}`) || `chapter-${chapter}`;
    let titleId = base;
    for (let n = 1; ids.includes(titleId); n++) titleId = `${base}-${n}`;
    // The headings plugin slugs preset ids again: each must come back unchanged.
    const check = createSlugger();
    [titleId, ...ids].forEach((id, index) => {
      if (!id || check.slug(id, { maintainCase: true }) !== id)
        file.fail(`Section numbering cannot keep the anchor "${id}".`, headings[index - 1]);
    });

    const self = outlineOf(title, chapter, headings, ids, toString);
    await numberLinks(tree, document);
    headings.forEach((heading, index) => {
      heading.children.unshift({ type: "text", value: `${self.sections.get(ids[index]).number} ` });
      heading.data = { ...heading.data, hProperties: { ...heading.data?.hProperties, id: ids[index] } };
    });
    let start = 0;
    while (["yaml", "toml"].includes(tree.children[start]?.type)) start++;
    tree.children.splice(start, 0, {
      type: "heading",
      depth: 1,
      children: [{ type: "text", value: `${chapter} ${title}` }],
      data: { hProperties: { id: titleId } },
    });

    async function numberLinks(node, page, inHeading = false) {
      for (const child of node.children || [])
        await numberLinks(child, page, inHeading || child.type === "heading");
      if (node.type !== "link" || inHeading) return;
      const match = /^(?:([^#?:]+\.mdx?))?(?:#(.*))?$/.exec(node.url);
      if (!match || (!match[1] && !match[2])) return;
      const target = match[1]
        ? path.posix.normalize(path.posix.join(path.posix.dirname(page), match[1]))
        : page;
      if (!chapters.has(target)) return;
      const outline = target === page ? self : await outlineOfPage(target, toString);
      const section = match[2] ? outline.sections.get(match[2]) : { number: outline.chapter, text: outline.title };
      if (!section) file.fail(`The link to ${node.url} names no section of ${target}.`, node);
      const other = chapters.get(target).manual;
      if (other !== manual && !match[2] && outline.chapter === "1") return;
      const prefix = other === manual ? "" : `${other} `;
      const label = toString(node);
      if (sameText(label, section.text))
        node.children.unshift({ type: "text", value: `${prefix}${section.number} ` });
      else if (sameText(label, outline.title))
        node.children = [{ type: "text", value: `${prefix}${section.number} ${section.text}` }];
      else node.children.push({ type: "text", value: ` (${prefix}section ${section.number})` });
    }
  };
};
