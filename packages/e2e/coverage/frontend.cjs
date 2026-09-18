// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
// Statement identities differ between SWC unit and webpack E2E transforms.
// Export source-line maps independently; the Python reporter unions those lines.
const fs = require("node:fs");
const path = require("node:path");
const { createCoverageMap } = require("istanbul-lib-coverage");
const { createSourceMapStore } = require("istanbul-lib-source-maps");
const babel = require("@babel/core");
const root = path.resolve(__dirname, "../../..");
const artifacts = process.env.E2E_ARTIFACTS;
const portals = [
  "admin-portal",
  "voting-portal",
  "ballot-verifier",
  "results-portal",
];
const excluded =
  /(?:\/gql\/|\/stories\/|\/__mocks__\/|\/translations\/|\.(?:test|spec|stories|d)\.[jt]sx?$|\/setup[^/]*\.[jt]sx?$)/;

function walk(directory) {
  if (!fs.existsSync(directory)) return [];
  return fs
    .readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) =>
      entry.isDirectory()
        ? walk(path.join(directory, entry.name))
        : [path.join(directory, entry.name)],
    );
}
function normalize(filename) {
  const relative = path.relative(root, filename).replaceAll(path.sep, "/");
  return relative.startsWith("packages/") &&
    !relative.includes("node_modules/") &&
    !excluded.test(filename)
    ? relative
    : undefined;
}
async function lines(map) {
  const remapped = await createSourceMapStore().transformCoverage(map);
  const output = {};
  for (const filename of remapped.files()) {
    const relative = normalize(filename);
    if (relative) {
      const coverage = remapped.fileCoverageFor(filename).getLineCoverage();
      const length = fs.readFileSync(path.join(root, relative), "utf8").split("\n").length;
      if (Object.keys(coverage).some((line) => Number(line) < 1 || Number(line) > length))
        throw new Error(`Coverage locations do not match source: ${relative}`);
      output[relative] = coverage;
    }
  }
  return output;
}
async function main() {
  const baseline = createCoverageMap({});
  for (const portal of portals) {
    for (const filename of walk(path.join(root, "packages", portal, "src"))) {
      if (!/\.[jt]sx?$/.test(filename) || excluded.test(filename)) continue;
      babel.transformFileSync(filename, {
        babelrc: false,
        configFile: false,
        presets: [
          require.resolve("@babel/preset-typescript"),
          [require.resolve("@babel/preset-react"), { runtime: "automatic" }],
        ],
        plugins: [
          [
            require.resolve("babel-plugin-istanbul"),
            {
              cwd: root,
              include: ["packages/*/src/**/*"],
              onCover: (_name, coverage) => baseline.addFileCoverage(coverage),
            },
          ],
        ],
      });
    }
  }
  const output = { baseline: await lines(baseline) };
  for (const kind of ["e2e", "unit"]) {
    const map = createCoverageMap({});
    for (const file of walk(
      path.join(artifacts, "coverage/frontend", kind),
    ).filter((file) => file.endsWith(".json"))) {
      map.merge(JSON.parse(fs.readFileSync(file, "utf8")));
    }
    output[kind] = await lines(map);
    if (kind === "e2e" && !map.files().length)
      throw new Error("No frontend E2E coverage files");
  }
  fs.writeFileSync(
    path.join(artifacts, "coverage/frontend-lines.json"),
    JSON.stringify(output),
  );
}
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
