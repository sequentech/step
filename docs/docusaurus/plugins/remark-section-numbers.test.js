// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const assert = require("node:assert/strict");
const plugin = require("./remark-section-numbers");
const documents = require("./remark-section-numbers.json");
// The Docusaurus plugins that run after this one, in their order.
const contentTitle = require("@docusaurus/mdx-loader/lib/remark/contentTitle").default;
const headings = require("@docusaurus/mdx-loader/lib/remark/headings").default;
const toc = require("@docusaurus/mdx-loader/lib/remark/toc").default;

const site = path.resolve(__dirname, "..");
const manual = { documents: { "AGD-X": ["docs/a.md", "docs/b.md", "docs/c.md"] } };
const title = "Secure Use of the Admin Portal";

/** Run the remark steps of a Docusaurus docs page, with or without numbering. */
async function compile(markdown, { page = "docs/c.md", frontMatter = { title }, numbered = true } = {}) {
  const { unified } = await import("unified");
  const { default: remarkParse } = await import("remark-parse");
  const { default: remarkFrontmatter } = await import("remark-frontmatter");
  const tree = unified().use(remarkParse).use(remarkFrontmatter).parse(`---\ntitle: x\n---\n\n${markdown}`);
  const file = {
    path: path.join(site, page),
    data: { frontMatter },
    fail(message) {
      throw new Error(message);
    },
  };
  if (numbered) await plugin(manual)(tree, file);
  await contentTitle({})(tree, file);
  await headings({ anchorsMaintainCase: false })(tree, file);
  await toc({})(tree, file);
  return { tree, file };
}
async function outline(tree) {
  const { toString } = await import("mdast-util-to-string");
  const found = [];
  (function walk(node) {
    if (node.type === "heading")
      found.push({ depth: node.depth, text: toString(node), id: node.data.hProperties.id });
    for (const child of node.children || []) walk(child);
  })(tree);
  return found;
}
function tocOf(tree) {
  const items = tree.children.at(-1).data.estree.body[0].declaration.declarations[0].init.elements;
  return items.map((item) =>
    Object.fromEntries(item.properties.map(({ key, value }) => [key.name ?? key.value, value.value])),
  );
}

test("titles a listed page with its chapter and numbers its sections below it", async () => {
  const { tree, file } = await compile(
    "Intro.\n\n## Signing in\n\n### With a passkey\n\n### With a password\n\n## Confirming sensitive actions\n\n#### Deep detail\n",
  );
  assert.equal(file.data.contentTitle, "3 Secure Use of the Admin Portal");
  // The theme renders this content title instead of a synthetic one: one h1.
  assert.equal(tree.children[1].name, "header");
  assert.deepEqual(
    (await outline(tree)).map(({ depth, text }) => [depth, text]),
    [
      [1, "3 Secure Use of the Admin Portal"],
      [2, "3.1 Signing in"],
      [3, "3.1.1 With a passkey"],
      [3, "3.1.2 With a password"],
      [2, "3.2 Confirming sensitive actions"],
      [4, "3.2.0.1 Deep detail"],
    ],
  );
  assert.deepEqual(
    tocOf(tree).map(({ value, level }) => [level, value]),
    [
      [2, "3.1 Signing in"],
      [3, "3.1.1 With a passkey"],
      [3, "3.1.2 With a password"],
      [2, "3.2 Confirming sensitive actions"],
      [4, "3.2.0.1 Deep detail"],
    ],
  );
});

test("keeps every anchor Docusaurus derives from the unnumbered headings", async () => {
  const markdown = [
    "## Your Ballot ID",
    "## Example",
    "### Example",
    "## Example",
    "## Step 1: Open the `ballot` *now*",
    "## Custom anchor {#your-ballot-id-copy}",
    "## Secure Use of the Admin Portal",
    // Same slug as the numbered title: the inserted h1 must not take it.
    "## 3 Secure Use of the Admin Portal",
  ].join("\n\n");
  const plain = await outline((await compile(markdown, { numbered: false })).tree);
  const { tree } = await compile(markdown);
  const numbered = (await outline(tree)).slice(1);
  assert.deepEqual(
    numbered.map(({ id }) => id),
    plain.map(({ id }) => id),
  );
  assert.deepEqual(plain.map(({ id }) => id), [
    "your-ballot-id",
    "example",
    "example-1",
    "example-2",
    "step-1-open-the-ballot-now",
    "your-ballot-id-copy",
    "secure-use-of-the-admin-portal",
    "3-secure-use-of-the-admin-portal",
  ]);
  assert.deepEqual(
    numbered.map(({ text }) => text),
    [
      "3.1 Your Ballot ID",
      "3.2 Example",
      "3.2.1 Example",
      "3.3 Example",
      "3.4 Step 1: Open the ballot now",
      "3.5 Custom anchor",
      "3.6 Secure Use of the Admin Portal",
      "3.7 3 Secure Use of the Admin Portal",
    ],
  );
  assert.deepEqual(
    tocOf(tree).map(({ id }) => id),
    plain.map(({ id }) => id),
  );
});

test("leaves pages outside the manuals as Docusaurus renders them", async () => {
  const markdown = "## Signing in\n\n### Example\n";
  const options = { page: "docs/other.md", numbered: false };
  const plain = await compile(markdown, options);
  const other = await compile(markdown, { ...options, numbered: true });
  assert.deepEqual(other.tree, plain.tree);
  assert.equal(other.file.data.contentTitle, undefined);
});

test("refuses a chapter without a title, a second h1, and anchors it cannot keep", async () => {
  await assert.rejects(compile("## Section\n", { frontMatter: {} }), /front matter title/);
  await assert.rejects(compile("# Own title\n\n## Section\n"), /h1/);
  for (const markdown of ["## A {#same}\n\n## B {#same}\n", "## A {#Mixed.Case}\n"])
    await assert.rejects(compile(markdown), /anchor/);
});

test("refuses a page listed twice", () => {
  assert.throws(
    () => plugin({ documents: { "AGD-X": ["docs/a.md"], "AGD-Y": ["docs/b.md", "docs/a.md"] } }),
    /twice/,
  );
});

test("numbers the four AGD manuals, each page once and titled, before Docusaurus's plugins", () => {
  assert.deepEqual(Object.keys(documents), ["AGD-01", "AGD-02", "AGD-03", "AGD-04"]);
  const pages = Object.values(documents).flat();
  assert.equal(new Set(pages).size, pages.length);
  for (const page of pages) {
    const source = fs.readFileSync(path.join(site, page), "utf8");
    assert.match(source, /^---\n(?:.*\n)*?title: *\S.*\n(?:.*\n)*?---\n/, page);
  }
  const config = require("../docusaurus.config.js");
  const [[, { docs }]] = config.presets;
  assert.deepEqual(docs.beforeDefaultRemarkPlugins, [[plugin, { documents }]]);
});
