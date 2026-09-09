// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("node:path");

/** Build an MDX element without parsing or interpolating executable code. */
function element(name, attributes, children) {
  return {
    type: "mdxJsxFlowElement",
    name,
    attributes: Object.entries(attributes).map(([name, value]) => ({
      type: "mdxJsxAttribute",
      name,
      value,
    })),
    children,
  };
}

/**
 * Turn adjacent fences with group="…" tab="…" into synchronized code tabs.
 * Groups are scoped to the source document; normal fences remain untouched.
 * Other fence metadata (titles, line highlighting) stays on the code block.
 */
module.exports = function remarkCodeTabs() {
  return (tree, file) => {
    const document = path.relative(
      path.resolve(__dirname, ".."),
      file.path || "document",
    );
    function metadata(node) {
      if (node.type !== "code") return null;
      const fields = {};
      const rest = (node.meta || "")
        .replace(/(?:^|\s)(group|tab)="([^"]*)"/g, (_, key, value) => {
          if (key in fields || !value.trim())
            file.fail(`Code tabs require a nonempty, unique ${key}.`, node);
          fields[key] = value;
          return "";
        })
        .trim();
      if (!Object.keys(fields).length) return null;
      if (!fields.group || !fields.tab)
        file.fail('Code tabs require both group="…" and tab="…".', node);
      return { ...fields, rest };
    }
    function transform(parent) {
      if (!parent.children) return;
      const output = [];
      for (let index = 0; index < parent.children.length; ) {
        const node = parent.children[index];
        const first = metadata(node);
        if (!first) {
          transform(node);
          output.push(node);
          index++;
          continue;
        }
        const tabs = [];
        const labels = new Set();
        while (index < parent.children.length) {
          const code = parent.children[index];
          const info = metadata(code);
          if (!info || info.group !== first.group) break;
          if (labels.has(info.tab))
            file.fail(`Duplicate code tab "${info.tab}".`, code);
          labels.add(info.tab);
          tabs.push(
            element("CodeTab", { value: info.tab, label: info.tab }, [
              { ...code, meta: info.rest || null },
            ]),
          );
          index++;
        }
        // A lone variant needs no selector, just like a shared command.
        output.push(
          tabs.length === 1
            ? tabs[0].children[0]
            : element(
                "CodeTabs",
                {
                  groupId: `code:${document}:${first.group}`,
                  className: "code-mode-tabs",
                },
                tabs,
              ),
        );
      }
      parent.children = output;
    }
    transform(tree);
  };
};
