// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const { test } = require("node:test");
const assert = require("node:assert/strict");
const plugin = require("./remark-code-tabs");

const code = (meta, value = "echo hello") => ({
  type: "code",
  lang: "bash",
  meta,
  value,
});
function render(children, path = "/docs/guide.md") {
  const tree = { type: "root", children };
  plugin()(tree, {
    path,
    fail(message) {
      throw new Error(message);
    },
  });
  return tree.children;
}
const variants = () => [
  code('group="engine" tab="k6" title="Run" {1}'),
  code('group="engine" tab="Chromium"'),
];
const attributes = (node) =>
  Object.fromEntries(node.attributes.map(({ name, value }) => [name, value]));

test("preserves code, highlighting and titles while synchronizing separated examples", () => {
  const [first, shared, second] = render([
    ...variants(),
    code(null),
    ...variants(),
  ]);
  assert.equal(first.name, "CodeTabs");
  assert.equal(attributes(first).groupId, attributes(second).groupId);
  assert.deepEqual(shared, code(null));
  assert.equal(first.children[0].children[0].meta, 'title="Run" {1}');
  assert.equal(first.children[0].children[0].value, "echo hello");
});

test("scopes unrelated documents and groups independently", () => {
  assert.notEqual(
    attributes(render(variants())[0]).groupId,
    attributes(render(variants(), "/docs/other.md")[0]).groupId,
  );
  const other = variants().map((node) => ({
    ...node,
    meta: node.meta.replace("engine", "language"),
  }));
  assert.notEqual(
    attributes(render(variants())[0]).groupId,
    attributes(render(other)[0]).groupId,
  );
});

test("supports arbitrary labels and counts, nested blocks and single unwrapped variants", () => {
  const items = ["Go", "Rust", "PHP 8"].map((label) =>
    code(`group="language" tab="${label}"`),
  );
  assert.equal(render(items)[0].children.length, 3);
  assert.equal(
    render([{ type: "blockquote", children: items }])[0].children[0].name,
    "CodeTabs",
  );
  assert.equal(render([items[0]])[0].type, "code");
});

test("rejects incomplete, duplicate and empty metadata at build time", () => {
  for (const meta of [
    'group="engine"',
    'tab="k6"',
    'group="" tab="k6"',
    'group="a" group="b" tab="k6"',
  ]) {
    assert.throws(() => render([code(meta)]), /Code tabs/);
  }
  assert.throws(
    () => render([variants()[0], variants()[0]]),
    /Duplicate code tab/,
  );
});
