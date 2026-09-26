# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests for scripts.dev.test: resolving targets to focused commands and running."""

import io
import json
import os
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

from scripts.dev import test as focused
from scripts.dev.affected.changes import Scope
from scripts.dev.affected.model import load_model
from scripts.dev.test_affected import fixture_files, write

MODEL = """\
schema = 1
default_base = "main"

[[workspaces]]
kind = "yarn"
manifest = "packages/package.json"

[[workspaces]]
kind = "cargo"
manifest = "packages/Cargo.toml"
target_dir = "packages/rust-local-target"

[[workspaces]]
kind = "cargo"
manifest = "packages/other/Cargo.toml"
prefix = "other/"
target_dir = "packages/other/rust-local-target"

[units.tools]
summary = "developer tools"

[units.flat]
summary = "flat script directory"

[units.docs]
summary = "documentation"

[[paths]]
globs = ["tools/**"]
units = ["tools"]

[[paths]]
globs = ["flat/**"]
units = ["flat"]

[[paths]]
globs = ["docs/**", "**/*.md"]
units = ["docs"]

[[checks]]
id = "jest:{unit}"
for_each = ["core-ui", "portal"]
kind = "test"
runner = "jest"
cost = "fast"
cwd = "{path}"
command = "yarn test --runInBand"
focus = ["yarn", "jest", "--runInBand"]

[[checks]]
id = "lint:portal"
units = ["portal"]
kind = "lint"
runner = "shell"
cost = "fast"
trigger = "changed"
cwd = "packages/portal"
command = "yarn lint"

[[checks]]
id = "stories:{unit}"
for_each = ["portal"]
kind = "test"
runner = "storybook"
cost = "slow"
cwd = "{path}"
command = "yarn test:stories"
focus = ["yarn", "test:story"]
[checks.also]
portal = ["kit"]

[[checks]]
id = "journeys:{unit}"
for_each = ["portal"]
kind = "test"
runner = "playwright"
cost = "slow"
cwd = "{path}"
command = "yarn build && yarn test:journeys"
focus = ["yarn", "test:journeys"]
prepare = "yarn build"
needs = ["{path}/dist/index.html"]
tests = ["{path}/test/journeys/**/*.spec.ts"]
requires = ["chromium"]

[[checks]]
id = "cargo-test:{unit}"
for_each = ["core", "service"]
kind = "test"
runner = "cargo"
cost = "fast"
cwd = "packages"
command = "cargo test --locked -p {unit}"
focus = ["cargo", "test", "--locked", "-p", "{unit}", "--features", "base"]

[[checks]]
id = "cargo-test:other"
units = ["cargo:packages/other/Cargo.toml"]
kind = "test"
runner = "cargo"
cost = "slow"
cwd = "packages/other"
command = "cargo test --release"

[[checks]]
id = "python:tools"
units = ["tools"]
kind = "test"
runner = "python"
cost = "fast"
cwd = "."
command = "python3 -m unittest discover -s tools -p 'test_*.py'"
focus = ["python3", "-m", "unittest"]
tests = ["tools/**/test_*.py"]

[[checks]]
id = "python:flat"
units = ["flat"]
kind = "test"
runner = "python"
cost = "fast"
cwd = "."
command = "python3 -m unittest discover -s flat"
focus = ["python3", "-m", "unittest"]
tests = ["flat/test_*.py"]

[[checks]]
id = "stack-e2e"
units = ["tools", "portal"]
kind = "test"
runner = "shell"
cost = "integration"
cwd = "."
command = "tools/run-stack.sh"
requires = ["docker"]
"""


class ResolverTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name).resolve()
        files = fixture_files()
        files["scripts/dev/affected.toml"] = MODEL
        files["packages/portal/package.json"] = json.dumps(
            {
                "name": "portal",
                "scripts": {"storybook": "storybook dev -p 6007 --no-open"},
                "dependencies": {"@x/core-ui": "*"},
                "devDependencies": {"@x/kit": "*"},
            }
        )
        files.update(
            {
                "packages/portal/src/Screen.test.tsx": "test('x', () => {})\n",
                "packages/portal/src/components/Widget.tsx": "export {}\n",
                "packages/portal/src/Screen.stories.tsx": "export default {}\n",
                "packages/kit/src/Kit.stories.tsx": "export default {}\n",
                "packages/portal/test/journeys/review.spec.ts": "test('r', () => {})\n",
                "packages/core/tests/contracts.rs": (
                    '#![cfg(all(feature = "extra", feature = "base"))]\n'
                ),
                "packages/core/tests/either.rs": '#![cfg(any(feature = "a"))]\n',
                "packages/service/src/handler.rs": "fn handler() {}\n",
                "tools/__init__.py": "",
                "tools/probe/__init__.py": "",
                "tools/probe/model.py": "",
                "tools/test_probe.py": "",
                "tools/run-stack.sh": "#!/bin/sh\n",
                "flat/run.py": "",
                "flat/test_run.py": "",
            }
        )
        write(self.root, files)
        manifest = self.root / "packages/core/Cargo.toml"
        manifest.write_text(
            manifest.read_text()
            + '\n[[test]]\nname = "contracts"\nrequired-features = ["gate"]\n'
        )
        self.model = load_model(self.root)
        cwd = mock.patch.dict(os.environ, {"STEP_DEV_CWD": str(self.root)})
        cwd.start()
        self.addCleanup(cwd.stop)

    def tearDown(self):
        self.directory.cleanup()

    def plan(self, *argv):
        arguments = list(argv)
        options = focused.parser("step-dev test").parse_args(arguments)
        options.extra = []
        options.name = options.name_option or options.name
        return focused.Resolver(self.model, options).plan()

    def commands(self, *argv):
        return [(step.cwd, step.shell()) for step in self.plan(*argv).steps]

    def test_package_runs_its_fast_tests(self):
        plan = self.plan("portal")
        self.assertEqual(
            [(step.check.id, step.shell()) for step in plan.steps],
            [("jest:portal", "yarn test --runInBand")],
        )
        self.assertEqual(
            plan.not_run,
            ["journeys:portal, stories:portal: step-dev test portal --depth broad"],
        )

    def test_broad_depth_adds_slow_tests(self):
        plan = self.plan("portal", "--depth", "broad")
        self.assertEqual(
            [step.check.id for step in plan.steps],
            ["jest:portal", "journeys:portal", "stories:portal"],
        )

    def test_package_consumers_are_named_but_not_run(self):
        plan = self.plan("core-ui")
        self.assertEqual(plan.not_run, ["consumers portal: step-dev test --affected"])

    def test_package_name_filter_narrows_every_test(self):
        self.assertEqual(
            self.commands("core", "parses_ballots"),
            [
                (
                    "packages",
                    "cargo test --locked -p core --features base -- parses_ballots",
                )
            ],
        )

    def test_package_without_tests_names_its_checks(self):
        with self.assertRaisesRegex(focused.SelectionError, "docs has no fast tests"):
            self.plan("docs")

    def test_check_id_runs_that_check(self):
        self.assertEqual(
            self.commands("stories:portal"), [("packages/portal", "yarn test:stories")]
        )

    def test_source_file_runs_related_jest_tests(self):
        self.assertEqual(
            self.commands("packages/portal/src/components/Widget.tsx"),
            [
                (
                    "packages/portal",
                    "yarn jest --runInBand --findRelatedTests "
                    "src/components/Widget.tsx",
                )
            ],
        )

    def test_test_file_runs_itself_with_a_name(self):
        self.assertEqual(
            self.commands("packages/portal/src/Screen.test.tsx", "-t", "renders"),
            [
                (
                    "packages/portal",
                    "yarn jest --runInBand src/Screen.test.tsx -t renders",
                )
            ],
        )

    def test_paths_resolve_from_the_callers_directory(self):
        with mock.patch.dict(
            os.environ, {"STEP_DEV_CWD": str(self.root / "packages/portal")}
        ):
            self.assertEqual(
                self.commands("src/Screen.test.tsx"),
                [("packages/portal", "yarn jest --runInBand src/Screen.test.tsx")],
            )

    def test_directory_runs_the_tests_under_it(self):
        self.assertEqual(
            self.commands("packages/portal/src/components"),
            [("packages/portal", "yarn jest --runInBand src/components")],
        )
        self.assertEqual(self.plan("packages/portal").heading, "package portal")

    def test_story_file_runs_in_the_storybook_that_collects_it(self):
        self.assertEqual(
            self.commands("packages/portal/src/Screen.stories.tsx"),
            [("packages/portal", "yarn test:story src/Screen.stories.tsx")],
        )
        # The portal's Storybook also collects the kit's stories.
        self.assertEqual(
            self.commands("packages/kit/src/Kit.stories.tsx"),
            [("packages/portal", "yarn test:story ../kit/src/Kit.stories.tsx")],
        )

    def test_story_ids_and_storybook_urls(self):
        self.assertEqual(
            self.commands("portal", "--story", "screens-review--populated"),
            [("packages/portal", "yarn test:story screens-review--populated")],
        )
        url = "http://localhost:6007/?path=/story/screens-review--empty"
        self.assertEqual(
            self.commands(url), [("packages/portal", f"yarn test:story '{url}'")]
        )
        with self.assertRaisesRegex(focused.SelectionError, "port 6099"):
            self.plan("http://localhost:6099/?path=/story/x--y")

    def test_playwright_spec_with_title_prepares_its_build(self):
        plan = self.plan(
            "packages/portal/test/journeys/review.spec.ts", "shows the review"
        )
        (step,) = plan.steps
        self.assertEqual(
            step.shell(),
            "yarn test:journeys test/journeys/review.spec.ts -g 'shows the review'",
        )
        self.assertEqual(step.needs, ("packages/portal/dist/index.html",))
        self.assertEqual(step.prepare, "yarn build")
        self.assertTrue(step.narrowed)

    def test_rust_source_runs_the_whole_crate(self):
        plan = self.plan("packages/service/src/handler.rs")
        self.assertEqual(
            [step.shell() for step in plan.steps], ["cargo test --locked -p service"]
        )
        self.assertIn("every test of cargo-test:service runs", plan.notes[0])

    def test_rust_test_target_gets_the_features_it_needs(self):
        (step,) = self.plan("packages/core/tests/contracts.rs").steps
        self.assertEqual(
            step.shell(),
            "cargo test --locked -p core --features base,gate,extra --test contracts",
        )

    def test_rust_test_target_with_uncertain_features_warns(self):
        plan = self.plan("packages/core/tests/either.rs")
        self.assertIn("any() or not()", plan.notes[0])

    def test_python_test_module_and_its_source(self):
        self.assertEqual(
            self.commands("tools/test_probe.py", "-k", "claims"),
            [(".", "python3 -m unittest tools.test_probe -k claims")],
        )
        self.assertEqual(
            self.commands("tools/probe/model.py"),
            [(".", "python3 -m unittest tools.test_probe")],
        )
        # Without a package, the file runs through discovery in its directory.
        self.assertEqual(
            self.commands("flat/run.py"),
            [(".", "python3 -m unittest discover -s flat -p test_run.py")],
        )

    def test_unknown_target_suggests_close_names(self):
        with self.assertRaisesRegex(focused.SelectionError, "did you mean portal"):
            self.plan("portl")

    def test_affected_runs_selected_checks_up_to_the_depth(self):
        files = ["--files", "packages/portal/src/components/Widget.tsx"]
        plan = self.plan("--affected", *files)
        self.assertEqual(
            [step.check.id for step in plan.steps], ["jest:portal", "lint:portal"]
        )
        self.assertEqual(
            plan.not_run,
            [
                "journeys:portal (slow): yarn build && yarn test:journeys",
                "stack-e2e (integration): tools/run-stack.sh",
                "stories:portal (slow): yarn test:stories",
            ],
        )
        broad = self.plan("--affected", "--depth", "broad", *files)
        self.assertEqual(len(broad.steps), 4)

    def test_affected_unknown_paths_select_everything_with_a_note(self):
        plan = self.plan("--affected", "--depth", "full", "--files", "new.txt")
        self.assertEqual(len(plan.steps), len(self.model.checks))
        self.assertEqual(
            plan.notes, ["broad selection: no rule or package claims new.txt"]
        )

    def test_affected_defaults_to_the_working_tree(self):
        options = focused.parser("step-dev test").parse_args(["--affected"])
        self.assertIs(options.scope, Scope.WORKTREE)

    def test_cargo_target_dir_is_per_checkout(self):
        step = self.plan("core").steps[0]
        with mock.patch.dict(os.environ, {"CARGO_TARGET_DIR": "rust-local-target"}):
            env = focused.environment(self.model, step)
        self.assertEqual(
            env["CARGO_TARGET_DIR"], str(self.root / "packages/rust-local-target")
        )
        with mock.patch.dict(os.environ, {"CARGO_TARGET_DIR": "/elsewhere"}):
            self.assertEqual(
                focused.environment(self.model, step)["CARGO_TARGET_DIR"], "/elsewhere"
            )
        other = self.plan("cargo-test:other").steps[0]
        with mock.patch.dict(os.environ, {}, clear=True):
            self.assertEqual(
                focused.environment(self.model, other)["CARGO_TARGET_DIR"],
                str(self.root / "packages/other/rust-local-target"),
            )

    def test_missing_tools_make_a_step_unavailable(self):
        step = self.plan("stack-e2e").steps[0]
        with mock.patch.object(focused.shutil, "which", return_value=None):
            self.assertEqual(focused.missing(self.model, step), "needs docker")
        with mock.patch.object(focused.shutil, "which", return_value="/bin/docker"):
            self.assertIsNone(focused.missing(self.model, step))
        jest = self.plan("portal").steps[0]
        with mock.patch.object(focused.shutil, "which", return_value=None):
            self.assertIn("devenv shell", focused.missing(self.model, jest))

    def test_watch_uses_jest_watch_and_polls_other_runners(self):
        (jest,) = self.plan("portal", "--watch").steps
        self.assertTrue(jest.watches)
        self.assertEqual(jest.shell(), "yarn jest --runInBand --watch")
        (cargo,) = self.plan("service", "--watch").steps
        self.assertFalse(cargo.watches)
        # cargo-watch follows the crate and every path dependency it builds.
        self.assertEqual(
            focused.cargo_watch(cargo).shell(),
            "cargo watch -w buildhelper -w core -w native -w service -w shared "
            "-w testkit -s 'cargo test --locked -p service'",
        )


class RunTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name).resolve()
        write(self.root, fixture_files())
        self.model = load_model(self.root)
        self.check = next(c for c in self.model.checks if c.id == "jest:portal")

    def tearDown(self):
        self.directory.cleanup()

    def step(self, command, narrowed=False, runner=None):
        check = self.check
        if runner is not None:
            check = focused.replace(check, runner=runner)
        return focused.Step(check, "probe", ".", command, narrowed=narrowed)

    def quietly(self, function, *arguments):
        with redirect_stdout(io.StringIO()) as output:
            result = function(*arguments)
        return result, output.getvalue()

    def test_exit_status_decides_the_outcome(self):
        outcome, _ = self.quietly(focused.execute, self.model, self.step("true"))
        self.assertIs(outcome, focused.Outcome.PASSED)
        outcome, _ = self.quietly(focused.execute, self.model, self.step("exit 3"))
        self.assertIs(outcome, focused.Outcome.FAILED)

    def test_a_narrowed_run_without_tests_is_not_a_pass(self):
        runner = focused.Runner.PYTHON
        empty = self.step(["echo", "Ran 0 tests in 0.000s"], True, runner)
        outcome, output = self.quietly(focused.execute, self.model, empty)
        self.assertIs(outcome, focused.Outcome.EMPTY)
        self.assertIn("Ran 0 tests", output)
        ran = self.step(["echo", "Ran 3 tests in 0.1s"], True, runner)
        self.assertIs(
            self.quietly(focused.execute, self.model, ran)[0], focused.Outcome.PASSED
        )

    def test_missing_needs_are_prepared_first(self):
        step = self.step("test -f built")
        step.prepare = "touch built"
        step.needs = ("built",)
        outcome, output = self.quietly(focused.execute, self.model, step)
        self.assertIs(outcome, focused.Outcome.PASSED)
        self.assertIn("==> prepare jest:portal", output)

    def test_summary_fails_when_any_step_did_not_pass(self):
        plan = focused.Plan("probe", steps=[self.step("true"), self.step("false")])
        with mock.patch.object(focused, "missing", return_value=None):
            status, output = self.quietly(focused.run, self.model, plan, False, False)
        self.assertEqual(status, 1)
        self.assertRegex(output, r"passed\s+jest:portal")
        self.assertRegex(output, r"failed\s+jest:portal")

    def test_unavailable_steps_are_reported_and_fail_the_run(self):
        plan = focused.Plan("probe", steps=[self.step("true")])
        with mock.patch.object(focused, "missing", return_value="needs docker"):
            status, output = self.quietly(focused.run, self.model, plan, False, False)
        self.assertEqual(status, 1)
        self.assertIn("unavailable", output)

    def test_dry_run_prints_the_scope_only(self):
        plan = focused.Plan("probe", steps=[self.step("exit 9")])
        status, output = self.quietly(focused.run, self.model, plan, True, False)
        self.assertEqual(status, 0)
        self.assertIn("runs      probe [fast]", output)
        self.assertNotIn("==>", output)

    def test_watched_fingerprint_ignores_build_output(self):
        source = self.root / "packages/portal/src/Screen.tsx"
        before = focused.fingerprint(self.root, ["packages/portal"])
        (self.root / "packages/portal/node_modules").mkdir()
        (self.root / "packages/portal/node_modules/x.js").write_text("x")
        self.assertEqual(focused.fingerprint(self.root, ["packages/portal"]), before)
        source.write_text("export const changed = 1\n")
        self.assertNotEqual(focused.fingerprint(self.root, ["packages/portal"]), before)

    def test_cli_reports_selection_errors(self):
        with redirect_stderr(io.StringIO()) as error:
            status = focused.main(["no-such-thing"], root=self.root)
        self.assertEqual(status, 2)
        self.assertIn("no package, check, file or story matches", error.getvalue())


class ParsingTest(unittest.TestCase):
    def test_tests_ran_per_runner(self):
        cases = [
            (
                focused.Runner.CARGO,
                "test result: ok. 0 passed; 0 failed; 0 ignored",
                False,
            ),
            (
                focused.Runner.CARGO,
                "test result: ok. 0 passed; 0 failed\n"
                "test result: ok. 2 passed; 0 failed",
                True,
            ),
            (focused.Runner.PYTHON, "Ran 0 tests in 0.000s\n\nOK", False),
            (focused.Runner.PYTHON, "Ran 1 test in 0.001s", True),
            (focused.Runner.JEST, "Tests:       3 skipped, 3 total", False),
            (focused.Runner.JEST, "\x1b[1mTests:\x1b[22m 2 passed, 2 total", True),
            (focused.Runner.VITEST, "      Tests  4 passed (4)", True),
            (focused.Runner.NODE, "# tests 0\n# pass 0", False),
            (focused.Runner.NODE, "# tests 2\n# pass 2", True),
            (focused.Runner.PLAYWRIGHT, "1 passed", True),
        ]
        for runner, output, expected in cases:
            with self.subTest(runner=runner, output=output):
                self.assertIs(focused.tests_ran(runner, output), expected)

    def test_cfg_features(self):
        self.assertEqual(focused.cfg_features("fn main() {}"), ([], True))
        self.assertEqual(
            focused.cfg_features('#![cfg(feature = "native")]\n'), (["native"], True)
        )
        self.assertEqual(
            focused.cfg_features(
                '#![cfg(all(\n    feature = "a",\n    feature = "b",\n))]\nuse x;\n'
            ),
            (["a", "b"], True),
        )
        self.assertEqual(
            focused.cfg_features('#![cfg(any(feature = "a", feature = "b"))]'),
            (["a", "b"], False),
        )

    def test_depth_allows_costs_up_to_it(self):
        cost = focused.Cost
        self.assertTrue(focused.Depth.FAST.allows(cost.FAST))
        self.assertFalse(focused.Depth.FAST.allows(cost.SLOW))
        self.assertTrue(focused.Depth.BROAD.allows(cost.SLOW))
        self.assertFalse(focused.Depth.BROAD.allows(cost.INTEGRATION))
        self.assertTrue(focused.Depth.FULL.allows(cost.INTEGRATION))

    def test_static_prefix(self):
        self.assertEqual(focused.static_prefix("scripts/dev/**"), "scripts/dev")
        self.assertEqual(focused.static_prefix("packages/*/rust"), "packages")
        self.assertEqual(focused.static_prefix("**/*.md"), "")


class RepositoryTest(unittest.TestCase):
    """Focused commands for this checkout's packages."""

    @classmethod
    def setUpClass(cls):
        cls.model = load_model(focused.ROOT)

    def commands(self, *argv):
        options = focused.parser("step-dev test").parse_args(list(argv))
        options.extra = []
        options.name = options.name_option or options.name
        with mock.patch.dict(os.environ, {"STEP_DEV_CWD": str(focused.ROOT)}):
            plan = focused.Resolver(self.model, options).plan()
        return [(step.cwd, step.shell()) for step in plan.steps]

    def test_voting_portal_package(self):
        self.assertEqual(
            self.commands("voting-portal"),
            [("packages/voting-portal", "yarn test --runInBand")],
        )

    def test_workbench_story_runs_in_the_voting_portal_storybook(self):
        self.assertEqual(
            self.commands(
                "packages/workbench/src/components/__stories__/PolicyPanel.stories.tsx"
            ),
            [
                (
                    "packages/voting-portal",
                    "yarn test:story "
                    "../workbench/src/components/__stories__/PolicyPanel.stories.tsx",
                )
            ],
        )

    def test_sqlite_boundaries_compile_with_their_feature(self):
        self.assertEqual(
            self.commands("packages/sequent-core/tests/sqlite_feature_boundaries.rs"),
            [
                (
                    "packages",
                    "cargo test --locked -p sequent-core --features "
                    "default_features,keycloak,sqlite --test sqlite_feature_boundaries",
                )
            ],
        )

    def test_scripts_dev_source_runs_its_test_module(self):
        self.assertEqual(
            self.commands("scripts/dev/affected/model.py"),
            [(".", "python3 -m unittest scripts.dev.test_affected")],
        )


if __name__ == "__main__":
    unittest.main()
