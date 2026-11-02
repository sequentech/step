// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("node:path");
const { test } = require("node:test");
const assert = require("node:assert/strict");
const plugin = require("./remark-screen-text");

const handbook = path.resolve(
  __dirname,
  "../docs/03-voters/02-voter_handbook/05-casting_your_vote.md",
);
function render(children, file = handbook) {
  const tree = { type: "root", children };
  plugin()(tree, { path: file });
  return tree.children;
}
const text = (value) => ({ type: "text", value });
const paragraph = (...children) => ({ type: "paragraph", children });
const marked = (nodes) =>
  nodes.map((node) =>
    node.type === "text"
      ? node.value
      : node.data?.hName === "span"
        ? [node.children[0].value]
        : node,
  );

test("marks each quoted screen text, across line breaks, and keeps its quotation marks", () => {
  const [result] = render([
    paragraph(text('Select "Cast\nballot", then "Finish".')),
  ]);
  assert.deepEqual(marked(result.children), [
    "Select ",
    ['"Cast\nballot"'],
    ", then ",
    ['"Finish"'],
    ".",
  ]);
  assert.deepEqual(result.children[1].data.hProperties.className, ["screen-text"]);
});

test("reaches tables, lists and links but never code", () => {
  const code = { type: "inlineCode", value: '"tally_session_id"' };
  const link = { type: "link", url: "#x", children: [text('"Locate your ballot"')] };
  const cell = { type: "tableCell", children: [code, text(" and "), link] };
  const list = { type: "list", children: [{ type: "listItem", children: [paragraph(text('"OK"'))] }] };
  const [row, items] = render([{ type: "tableRow", children: [cell] }, list]);
  assert.deepEqual(row.children[0].children[0], code);
  assert.deepEqual(marked(row.children[0].children[2].children), [['"Locate your ballot"']]);
  assert.deepEqual(marked(items.children[0].children[0].children), [['"OK"']]);
});

test("keeps quotations that span code or links as they are, and the pairs after them", () => {
  const code = { type: "inlineCode", value: "G5AT-12SZ" };
  const [result] = render([
    paragraph(text('It shows "Your ID: '), code, text('" and then "Next".')),
  ]);
  assert.deepEqual(marked(result.children), [
    'It shows "Your ID: ',
    code,
    '" and then ',
    ['"Next"'],
    ".",
  ]);
});

test("leaves other documents alone", () => {
  const other = path.resolve(__dirname, "../docs/05-reference/06-deployment_guide/08-installing_toe.md");
  const input = () => [paragraph(text('Enter "value" here.'))];
  assert.deepEqual(render(input(), other), input());
});
