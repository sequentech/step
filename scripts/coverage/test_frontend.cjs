// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Exercise the real Jest -> Istanbul -> paired verdict boundary. Both revisions
// retain their own tests; deleting one passing test must close the coverage gate.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const candidate = path.resolve(__dirname, "../..");
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "frontend-ratchet-"));
function run(executable, args, cwd) {
  const result = spawnSync(executable, args, {
    cwd,
    encoding: "utf8",
    timeout: 120000,
    env: { ...process.env, PYTHONDONTWRITEBYTECODE: "1" },
  });
  if (result.error) throw result.error;
  return result;
}
function fixture(name, cases) {
  const checkout = path.join(temporary, name);
  const source = path.join(checkout, "packages/fixture/src");
  fs.mkdirSync(source, { recursive: true });
  fs.mkdirSync(path.join(checkout, "scripts"));
  fs.cpSync(__dirname, path.join(checkout, "scripts/coverage"), {
    recursive: true,
  });
  fs.writeFileSync(
    path.join(checkout, ".gitignore"),
    "node_modules\n__pycache__/\n",
  );
  fs.symlinkSync(
    path.join(candidate, "packages/node_modules"),
    path.join(checkout, "packages/node_modules"),
  );
  fs.writeFileSync(
    path.join(source, "choice.js"),
    "exports.choose = flag => {\n if (flag) return 'yes';\n return 'no';\n};\n" +
      "exports.fallback = () => {\n return 'fallback';\n};\n",
  );
  fs.writeFileSync(
    path.join(source, "choice.test.js"),
    "const {choose, fallback} = require('./choice');\n" + cases,
  );
  fs.writeFileSync(
    path.join(source, "../package.json"),
    '{"name":"fixture","private":true}',
  );
  const config = require(
    path.join(candidate, "packages/ui-core/jest.config.cjs"),
  );
  config.testMatch = ["<rootDir>/src/**/*.test.js"];
  config.collectCoverageFrom = ["src/**/*.js", "!src/**/*.test.js"];
  fs.writeFileSync(
    path.join(source, "../jest.config.cjs"),
    "module.exports = " + JSON.stringify(config),
  );
  for (const args of [
    ["init", "-q"],
    ["add", "."],
    [
      "-c",
      "user.name=Fixture",
      "-c",
      "user.email=fixture@example.invalid",
      "commit",
      "-qm",
      "fixture",
    ],
  ]) {
    const result = run("git", args, checkout);
    assert.equal(result.status, 0, result.stderr);
  }
  return checkout;
}
try {
  const yes = "test('yes', () => expect(choose(true)).toBe('yes'));\n";
  const no =
    "test('no and fallback', () => { expect(choose(false)).toBe('no'); expect(fallback()).toBe('fallback') });\n";
  const base = fixture("base", yes + no);
  const scenarios = [
    ["equal", yes + no, 0, "pass"],
    ["removed", yes, 1, "regression"],
    ["skipped", yes + no.replace("test(", "test.skip("), 2, "error"],
    [
      "failed",
      yes + "test('wrong', () => expect(choose(false)).toBe('yes'));",
      2,
      "error",
    ],
    ["empty", "", 2, "error"],
  ];
  for (const [name, cases, code, status] of scenarios) {
    const head = fixture(name, cases);
    const output = path.join(temporary, `reports-${name}`);
    const result = run(
      "python3",
      [
        path.join(head, "scripts/coverage/ci.py"),
        "frontend",
        "fixture",
        "--base",
        base,
        "--head",
        head,
        "--output",
        output,
      ],
      head,
    );
    assert.equal(result.status, code, result.stdout + result.stderr);
    const directories = fs.readdirSync(output);
    assert.equal(directories.length, 1);
    const verdict = JSON.parse(
      fs.readFileSync(path.join(output, directories[0], "verdict.json")),
    );
    assert.equal(verdict.status, status);
    if (name === "removed") {
      for (const metric of ["lines", "statements", "functions", "branches"]) {
        assert.equal(verdict.metrics[metric].decreased, true, metric);
      }
    }
    console.log(`${name}: ${status}`);
  }
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
