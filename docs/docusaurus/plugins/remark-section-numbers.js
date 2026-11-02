// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("node:path");
const { createSlugger, parseMarkdownHeadingId } = require("@docusaurus/utils");

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
 */
module.exports = function remarkSectionNumbers({ documents = {} } = {}) {
  const chapters = new Map();
  for (const pages of Object.values(documents))
    pages.forEach((page, index) => {
      if (chapters.has(page))
        throw new Error(`Section numbering lists ${page} twice.`);
      chapters.set(page, index + 1);
    });
  return async (tree, file) => {
    const document = path
      .relative(path.resolve(__dirname, ".."), file.path || "document")
      .split(path.sep)
      .join("/");
    const chapter = chapters.get(document);
    if (!chapter) return;
    const title = file.data.frontMatter?.title;
    if (typeof title !== "string" || !title.trim())
      file.fail("A numbered chapter needs a front matter title.");
    const headings = [];
    (function collect(node) {
      if (node.type === "heading") headings.push(node);
      for (const child of node.children || []) collect(child);
    })(tree);
    const own = headings.find((heading) => heading.depth === 1);
    if (own)
      file.fail("A numbered chapter takes its h1 from the front matter title; remove this h1.", own);

    // The ids of the Docusaurus headings plugin, from the unnumbered texts.
    const { toString } = await import("mdast-util-to-string");
    const slugs = createSlugger();
    const ids = headings.map((heading) => {
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
    const base = createSlugger().slug(`${chapter} ${title}`) || `chapter-${chapter}`;
    let titleId = base;
    for (let n = 1; ids.includes(titleId); n++) titleId = `${base}-${n}`;
    // The headings plugin slugs preset ids again: each must come back unchanged.
    const check = createSlugger();
    [titleId, ...ids].forEach((id, index) => {
      if (!id || check.slug(id, { maintainCase: true }) !== id)
        file.fail(`Section numbering cannot keep the anchor "${id}".`, headings[index - 1]);
    });

    const counters = [];
    headings.forEach((heading, index) => {
      counters.length = Math.min(counters.length, heading.depth - 1);
      while (counters.length < heading.depth - 1) counters.push(0);
      counters[heading.depth - 2]++;
      heading.children.unshift({ type: "text", value: `${[chapter, ...counters].join(".")} ` });
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
  };
};
