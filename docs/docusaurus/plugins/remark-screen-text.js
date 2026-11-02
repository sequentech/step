// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("node:path");

/** The handbooks whose convention is that text in double quotes is what the screen shows. */
const HANDBOOKS = [
  "docs/03-voters/02-voter_handbook/",
  "docs/02-election_managers/04-administrator_handbook/",
  "docs/02-election_managers/05-election_organizer_handbook/",
];
const BLOCKS = new Set(["paragraph", "heading", "tableCell"]);

/**
 * Mark the quoted screen texts of the handbooks with the class screen-text, so
 * the site and the PDFs can set them apart from the guidance around them. The
 * quotation marks stay in the text. Quotation marks pair up across a whole
 * paragraph, heading or table cell, line breaks included; a quotation that
 * spans code, a link or emphasis stays as it is. Code is never changed.
 */
module.exports = function remarkScreenText() {
  return (tree, file) => {
    const document = path
      .relative(path.resolve(__dirname, ".."), file.path || "document")
      .split(path.sep)
      .join("/");
    if (!HANDBOOKS.some((folder) => document.startsWith(folder))) return;
    function split(node, quote) {
      const parts = [];
      let last = 0;
      let start = quote.open ? -1 : null;
      for (let index = 0; index < node.value.length; index++) {
        if (node.value[index] !== '"') continue;
        if (start === null) {
          start = index;
          continue;
        }
        if (start >= 0) {
          if (start > last)
            parts.push({ type: "text", value: node.value.slice(last, start) });
          parts.push({
            type: "emphasis",
            data: { hName: "span", hProperties: { className: ["screen-text"] } },
            children: [{ type: "text", value: node.value.slice(start, index + 1) }],
          });
          last = index + 1;
        }
        start = null;
      }
      quote.open = start !== null;
      if (!parts.length) return [node];
      if (last < node.value.length)
        parts.push({ type: "text", value: node.value.slice(last) });
      return parts;
    }
    function inline(parent, quote) {
      if (!parent.children) return;
      parent.children = parent.children.flatMap((node) => {
        if (node.type === "text") return split(node, quote);
        inline(node, quote);
        return [node];
      });
    }
    function transform(parent) {
      for (const node of parent.children || []) {
        if (BLOCKS.has(node.type)) inline(node, { open: false });
        else transform(node);
      }
    }
    transform(tree);
  };
};
