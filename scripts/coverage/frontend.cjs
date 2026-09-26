// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Both revisions use the candidate's instrumenter and source-inventory rules.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { createRequire } = require("node:module");
const { spawnSync } = require("node:child_process");

const [checkout, packageName, output] = process.argv.slice(2);
assert(/^[a-z][a-z0-9-]*$/.test(packageName), "Expected a package identifier");
const candidate = path.resolve(__dirname, "../..");
const packageRoot = path.join(checkout, "packages", packageName);
const candidatePackage = path.join(candidate, "packages", packageName);
const resolve = createRequire(
  path.join(candidatePackage, "package.json"),
).resolve;
const config = require(path.join(candidatePackage, "jest.config.cjs"));
assert(config.collectCoverageFrom?.length, "Missing complete source inventory");
config.rootDir = packageRoot;
config.collectCoverage = true;
config.coverageThreshold = {};
config.coverageDirectory = output;
config.coverageReporters = ["text", "html", "lcov", "json", "json-summary"];
config.coverageProvider = "babel";
config.testEnvironment = resolve(`jest-environment-${config.testEnvironment}`);
for (const key of ["setupFiles", "setupFilesAfterEnv"]) {
  config[key] = (config[key] || []).map((file) => {
    const original = file.replace("<rootDir>", packageRoot);
    // Environment bootstrap may be introduced with coverage tooling. Keep
    // the base's own test helpers whenever they already exist.
    return fs.existsSync(original)
      ? original
      : file.replace("<rootDir>", candidatePackage);
  });
}
for (const entry of Object.values(config.transform)) {
  entry[0] = resolve(entry[0]);
  entry[1].presets = entry[1].presets.map((preset) =>
    Array.isArray(preset) ? [resolve(preset[0]), preset[1]] : resolve(preset),
  );
}
const configFile = path.join(output, "jest-profile.json");
fs.writeFileSync(configFile, JSON.stringify(config, null, 2));
const testResult = path.join(output, "tests.json");
const run = spawnSync(
  process.execPath,
  [
    resolve("jest/bin/jest"),
    "--config",
    configFile,
    "--runInBand",
    "--json",
    "--outputFile",
    testResult,
  ],
  {
    cwd: packageRoot,
    stdio: "inherit",
    env: { ...process.env, CI: "true" },
    timeout: 900000,
  },
);
if (run.error) throw run.error;
assert.equal(run.status, 0, "Jest did not finish successfully");
const tests = JSON.parse(fs.readFileSync(testResult, "utf8"));
assert(tests.success && tests.numPassedTests > 0, "No passing tests");
assert.equal(tests.numPendingTests, 0, "Required frontend tests were skipped");
const raw = JSON.parse(
  fs.readFileSync(path.join(output, "coverage-final.json"), "utf8"),
);
assert(Object.keys(raw).length > 0, "Empty coverage inventory");
const { createCoverageMap } = require(resolve("istanbul-lib-coverage"));
const computed = createCoverageMap(raw).getCoverageSummary().toJSON();
const summary = JSON.parse(
  fs.readFileSync(path.join(output, "coverage-summary.json"), "utf8"),
).total;
for (const metric of ["lines", "statements", "functions", "branches"]) {
  assert.equal(computed[metric].covered, summary[metric].covered);
  assert.equal(computed[metric].total, summary[metric].total);
}
