# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests for scripts.dev.affected: the workspace graph, changes and selection."""

import io
import json
import os
import subprocess
import tempfile
import textwrap
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

from scripts.dev.affected import cli
from scripts.dev.affected.changes import (
    BaseSource,
    Change,
    ChangeSet,
    Scope,
    Status,
    collect,
    from_files,
    parse_name_status,
)
from scripts.dev.affected.config import (
    ConfigError,
    Cost,
    Glob,
    expand_checks,
    parse_config,
)
from scripts.dev.affected.model import Impact, load_model
from scripts.dev.affected.report import json_report
from scripts.dev.affected.workspaces import cargo_lock_impact

ROOT = Path(__file__).resolve().parents[2]

# Git without the developer's configuration or signing, with a fixed identity.
GIT_ENV = {
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_AUTHOR_NAME": "Test",
    "GIT_AUTHOR_EMAIL": "test@example.invalid",
    "GIT_COMMITTER_NAME": "Test",
    "GIT_COMMITTER_EMAIL": "test@example.invalid",
}

FIXTURE_MODEL = """\
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

[units.docs]
summary = "documentation"

[units.toolchain]
summary = "the devenv toolchain"

[units.rust-toolchain]
summary = "rust-toolchain.toml"
consumers = ["cargo:*"]

[units.ci]
summary = "workflows"

[units.schema]
summary = "database schema"

[units.story-support]
summary = "story runner shared by the Storybooks"

[units.service]
test_inputs = ["schema/**"]

[[paths]]
globs = ["devenv.nix"]
units = ["toolchain"]
broad = "the toolchain changed"

[[paths]]
globs = ["rust-toolchain.toml"]
units = ["rust-toolchain"]

[[paths]]
globs = ["docs/**", "**/*.md"]
units = ["docs"]

[[paths]]
globs = [".github/**"]
units = ["ci"]

[[paths]]
globs = ["schema/**"]
units = ["schema"]

[[paths]]
globs = ["story-support/**"]
units = ["story-support"]

[[checks]]
id = "jest:{unit}"
for_each = ["core-ui", "portal"]
kind = "test"
runner = "jest"
cost = "fast"
cwd = "{path}"
command = "yarn test"
focus = ["yarn", "jest"]
workflows = ["tests.yml"]

[[checks]]
id = "lint:{unit}"
for_each = ["core-ui", "portal"]
kind = "lint"
runner = "shell"
cost = "fast"
trigger = "changed"
cwd = "{path}"
command = "yarn lint"

[[checks]]
id = "stories:{unit}"
for_each = ["portal"]
units = ["kit"]
kind = "test"
runner = "storybook"
cost = "slow"
cwd = "{path}"
command = "yarn test:stories"
[checks.also]
portal = ["story-support"]

[[checks]]
id = "cargo-test:{unit}"
for_each = ["core", "service", "p1", "other/core"]
kind = "test"
runner = "cargo"
cost = "fast"
cwd = "packages"
command = "cargo test -p {unit}"
actions = ["setup-rust"]

[[checks]]
id = "wasm-freshness"
units = ["core"]
kind = "build"
runner = "shell"
cost = "fast"
cwd = "."
command = "check-wasm"
paths = ["packages/*/rust/*.tgz"]

[[checks]]
id = "schema-contracts"
units = ["schema", "service"]
kind = "test"
runner = "shell"
cost = "integration"
cwd = "."
command = "run-db"

[[checks]]
id = "docs-build"
units = ["docs"]
kind = "build"
runner = "shell"
cost = "slow"
cwd = "docs"
command = "build-docs"
"""

CARGO_LOCK = """\
version = 3

[[package]]
name = "core"
version = "0.1.0"
dependencies = ["serde"]

[[package]]
name = "service"
version = "0.1.0"
dependencies = ["core", "tokio"]

[[package]]
name = "p1"
version = "0.1.0"
dependencies = ["core"]

[[package]]
name = "serde"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "aaaa"

[[package]]
name = "tokio"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bbbb"
"""


def write(root: Path, files: dict[str, str]) -> None:
    for name, content in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(textwrap.dedent(content))


def fixture_files() -> dict[str, str]:
    return {
        "scripts/dev/affected.toml": FIXTURE_MODEL,
        "packages/package.json": json.dumps(
            {"private": True, "workspaces": {"packages": ["core-ui", "portal", "kit"]}}
        ),
        "packages/yarn.lock": "# yarn lockfile v1\n",
        "packages/core-ui/package.json": json.dumps(
            {
                "name": "@x/core-ui",
                "peerDependencies": {"wasm-pkg": "file:./rust/wasm-pkg-0.1.0.tgz"},
            }
        ),
        "packages/core-ui/rust/wasm-pkg-0.1.0.tgz": "tgz",
        "packages/core-ui/src/Button.tsx": "export {}\n",
        "packages/portal/package.json": json.dumps(
            {
                "name": "portal",
                "dependencies": {
                    "@x/core-ui": "*",
                    "wasm-pkg": "file:./rust/wasm-pkg-0.1.0.tgz",
                },
                "devDependencies": {"@x/kit": "*"},
            }
        ),
        "packages/portal/rust/wasm-pkg-0.1.0.tgz": "tgz",
        "packages/portal/src/Screen.tsx": "export {}\n",
        # A plain range for a package that others take from a local file.
        "packages/kit/package.json": json.dumps(
            {"name": "@x/kit", "peerDependencies": {"wasm-pkg": "*"}}
        ),
        "packages/kit/fixtures/election.ts": "export {}\n",
        "packages/Cargo.toml": """\
            [workspace]
            members = ["core", "service", "plugins/*"]

            [workspace.dependencies]
            shared = { path = "shared" }
        """,
        "packages/Cargo.lock": CARGO_LOCK,
        "packages/core/Cargo.toml": '[package]\nname = "core"\n',
        "packages/core/src/lib.rs": "pub fn core() {}\n",
        "packages/service/Cargo.toml": """\
            [package]
            name = "service"

            [dependencies]
            core = { path = "../core" }
            shared = { workspace = true }

            [dev-dependencies]
            testkit = { path = "../testkit" }

            [build-dependencies]
            buildhelper = { path = "../buildhelper" }

            [target.'cfg(unix)'.dependencies]
            native = { path = "../native" }
        """,
        "packages/service/src/lib.rs": (
            'const DATA: &str = include_str!("../../../docs/data.txt");\n'
        ),
        "packages/shared/Cargo.toml": '[package]\nname = "shared"\n',
        "packages/testkit/Cargo.toml": '[package]\nname = "testkit"\n',
        "packages/buildhelper/Cargo.toml": '[package]\nname = "buildhelper"\n',
        "packages/native/Cargo.toml": '[package]\nname = "native"\n',
        "packages/plugins/p1/Cargo.toml": """\
            [package]
            name = "p1"

            [dependencies]
            core = { path = "../../core" }
        """,
        "packages/other/Cargo.toml": '[workspace]\nmembers = ["crates/core"]\n',
        "packages/other/crates/core/Cargo.toml": '[package]\nname = "core"\n',
        "docs/data.txt": "data\n",
        "docs/guide.md": "# Guide\n",
        "schema/001.sql": "create table t ();\n",
        "story-support/runner.mjs": "export {}\n",
        "rust-toolchain.toml": '[toolchain]\nchannel = "1.0"\n',
        "devenv.nix": "{}\n",
        ".github/workflows/tests.yml": "on: push\n",
        ".github/actions/setup-rust/action.yml": "runs: {}\n",
    }


class Fixture(unittest.TestCase):
    """A temporary checkout with Yarn and Cargo workspaces and a model."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name).resolve()
        write(self.root, fixture_files())

    def tearDown(self):
        self.directory.cleanup()

    def model(self):
        return load_model(self.root)

    def select(self, *paths):
        model = self.model()
        return model, model.select(from_files(self.root, paths, self.root))

    def selected(self, *paths):
        _, selection = self.select(*paths)
        return {decision.check.id for decision in selection.selected()}


class GraphTest(Fixture):
    def test_yarn_workspace_dependencies_become_edges(self):
        units = self.model().units
        self.assertEqual(units["portal"].depends, {"core-ui", "kit"})
        self.assertEqual(units["core-ui"].depends, set())
        self.assertEqual(units["portal"].path, "packages/portal")

    def test_local_file_dependencies_are_inputs(self):
        units = self.model().units
        # A package's own archive is already its file.
        self.assertEqual(units["portal"].inputs, [])
        # The plain range resolves to the committed archives of that name.
        self.assertEqual(
            sorted(glob.pattern for glob in units["kit"].inputs),
            [
                "packages/core-ui/rust/wasm-pkg-0.1.0.tgz",
                "packages/portal/rust/wasm-pkg-0.1.0.tgz",
            ],
        )

    def test_cargo_path_dependencies_of_every_kind_are_edges(self):
        units = self.model().units
        self.assertEqual(
            units["service"].depends,
            {"core", "shared", "testkit", "buildhelper", "native", "rust-toolchain"},
        )
        self.assertEqual(units["p1"].depends, {"core", "rust-toolchain"})
        # Path dependencies outside the member list are packages of the graph.
        self.assertEqual(units["testkit"].workspace, "packages/Cargo.toml")

    def test_second_cargo_workspace_is_prefixed(self):
        units = self.model().units
        self.assertEqual(units["other/core"].path, "packages/other/crates/core")
        self.assertEqual(units["other/core"].workspace, "packages/other/Cargo.toml")
        self.assertEqual(units["core"].path, "packages/core")

    def test_clashing_names_without_prefix_are_rejected(self):
        model_file = self.root / "scripts/dev/affected.toml"
        model_file.write_text(model_file.read_text().replace('prefix = "other/"\n', ""))
        with self.assertRaisesRegex(ConfigError, "two packages are called core"):
            self.model()

    def test_included_files_outside_the_crate_are_inputs(self):
        units = self.model().units
        self.assertIn(
            "docs/data.txt", [glob.pattern for glob in units["service"].inputs]
        )

    def test_consumer_declarations_add_edges(self):
        units = self.model().units
        self.assertIn("rust-toolchain", units["other/core"].depends)
        self.assertNotIn("rust-toolchain", units["portal"].depends)

    def test_area_without_summary_is_rejected(self):
        model_file = self.root / "scripts/dev/affected.toml"
        model_file.write_text(model_file.read_text() + "\n[units.typo]\ndepends = []\n")
        with self.assertRaisesRegex(ConfigError, "units.typo: no package"):
            self.model()

    def test_rule_naming_an_unknown_unit_is_rejected(self):
        model_file = self.root / "scripts/dev/affected.toml"
        model_file.write_text(
            model_file.read_text().replace('units = ["schema"]', 'units = ["nope"]')
        )
        with self.assertRaisesRegex(ConfigError, "unknown unit 'nope'"):
            self.model()


class ClaimTest(Fixture):
    def test_nested_package_owns_its_files(self):
        model = self.model()
        claim = model.claim("packages/other/crates/core/src/lib.rs")
        self.assertEqual(claim.units, ("other/core",))

    def test_rules_come_before_packages(self):
        claim = self.model().claim("packages/portal/README.md")
        self.assertEqual(claim.units, ("docs",))
        self.assertEqual(claim.source, "rule **/*.md")

    def test_unclaimed_path_is_unknown(self):
        self.assertIsNone(self.model().claim("unknown/file.txt"))
        # A directory prefix is not a file of the package.
        self.assertIsNone(self.model().claim("packages/portalx/file.txt"))


class SelectionTest(Fixture):
    def test_shared_ui_change_reaches_consumers_with_chains(self):
        model, selection = self.select("packages/core-ui/src/Button.tsx")
        self.assertEqual(list(selection.units), ["core-ui", "portal"])
        self.assertEqual(selection.units["portal"].impact, Impact.DEPENDENCY)
        self.assertEqual(selection.units["portal"].chain, ["core-ui", "portal"])
        chosen = {decision.check.id for decision in selection.selected()}
        self.assertEqual(
            chosen,
            {"jest:core-ui", "jest:portal", "lint:core-ui", "stories:portal"},
        )
        self.assertFalse(selection.fallback)

    def test_lint_follows_only_its_own_files(self):
        _, selection = self.select("packages/core-ui/src/Button.tsx")
        decision = next(d for d in selection.decisions if d.check.id == "lint:portal")
        self.assertFalse(decision.selected)
        self.assertIn("only through dependencies", decision.reasons[0])

    def test_leaf_change_selects_only_its_checks(self):
        self.assertEqual(
            self.selected("packages/portal/src/Screen.tsx"),
            {"jest:portal", "lint:portal", "stories:portal"},
        )

    def test_documentation_only_selects_documentation_checks(self):
        _, selection = self.select("docs/guide.md", "packages/portal/README.md")
        self.assertEqual({d.check.id for d in selection.selected()}, {"docs-build"})
        self.assertEqual(list(selection.units), ["docs"])

    def test_also_adds_units_to_one_family_member(self):
        self.assertEqual(self.selected("story-support/runner.mjs"), {"stories:portal"})

    def test_unknown_path_selects_every_check(self):
        model, selection = self.select("new-top-level.txt")
        self.assertEqual(len(selection.selected()), len(model.checks))
        self.assertEqual(
            selection.fallback, ["no rule or package claims new-top-level.txt"]
        )
        self.assertIsNone(selection.files[0].claim)

    def test_broad_rule_selects_every_check_with_its_reason(self):
        model, selection = self.select("devenv.nix")
        self.assertEqual(len(selection.selected()), len(model.checks))
        self.assertEqual(selection.fallback, ["the toolchain changed: devenv.nix"])

    def test_toolchain_file_affects_every_cargo_package(self):
        _, selection = self.select("rust-toolchain.toml")
        self.assertTrue(
            {"core", "service", "p1", "other/core", "testkit"} <= set(selection.units)
        )
        self.assertNotIn("portal", selection.units)
        self.assertIn("cargo-test:other/core", self.selected("rust-toolchain.toml"))

    def test_rust_source_selects_cargo_consumers_and_wasm_freshness(self):
        chosen = self.selected("packages/core/src/lib.rs")
        self.assertEqual(
            chosen,
            {
                "cargo-test:core",
                "cargo-test:service",
                "cargo-test:p1",
                "wasm-freshness",
                # It covers service, which depends on core.
                "schema-contracts",
            },
        )

    def test_committed_wasm_archive_selects_its_consumers(self):
        _, selection = self.select("packages/core-ui/rust/wasm-pkg-0.1.0.tgz")
        self.assertEqual(set(selection.units), {"core-ui", "portal", "kit"})
        self.assertEqual(selection.files[0].inputs, ("kit",))
        chosen = {d.check.id for d in selection.selected()}
        self.assertIn("wasm-freshness", chosen)
        self.assertIn("jest:portal", chosen)

    def test_test_inputs_select_only_the_owners_checks(self):
        _, selection = self.select("schema/001.sql")
        self.assertEqual(selection.units["service"].impact, Impact.TEST_INPUT)
        chosen = {d.check.id for d in selection.selected()}
        # service's tests read the schema; its consumers do not.
        self.assertEqual(chosen, {"cargo-test:service", "schema-contracts"})

    def test_workspace_manifest_affects_every_member(self):
        _, selection = self.select("packages/package.json")
        self.assertEqual(set(selection.units), {"core-ui", "portal", "kit"})
        self.assertEqual(
            selection.files[0].claim.source, "manifest of packages/package.json"
        )

    def test_yarn_lock_affects_every_yarn_package(self):
        _, selection = self.select("packages/yarn.lock")
        self.assertEqual(set(selection.units), {"core-ui", "portal", "kit"})

    def test_cargo_lock_without_base_affects_the_whole_workspace(self):
        _, selection = self.select("packages/Cargo.lock")
        claim = selection.files[0].claim
        self.assertIn("no base version to compare", claim.source)
        self.assertEqual(
            set(claim.units),
            {"core", "service", "p1", "shared", "testkit", "buildhelper", "native"},
        )

    def test_workflow_and_action_changes_select_the_checks_they_run(self):
        self.assertEqual(
            self.selected(".github/workflows/tests.yml"),
            {"jest:core-ui", "jest:portal"},
        )
        self.assertEqual(
            self.selected(".github/actions/setup-rust/action.yml"),
            {
                "cargo-test:core",
                "cargo-test:service",
                "cargo-test:p1",
                "cargo-test:other/core",
            },
        )

    def test_path_triggers_match_changed_files(self):
        _, selection = self.select("packages/portal/rust/wasm-pkg-0.1.0.tgz")
        decision = next(
            d for d in selection.decisions if d.check.id == "wasm-freshness"
        )
        self.assertEqual(
            decision.reasons[0],
            "packages/portal/rust/wasm-pkg-0.1.0.tgz matches packages/*/rust/*.tgz",
        )

    def test_no_changes_select_nothing(self):
        model = self.model()
        selection = model.select(
            ChangeSet(root=self.root, scope=Scope.FILES, changes=[])
        )
        self.assertEqual(selection.selected(), [])
        self.assertEqual(selection.decisions[0].reasons, ["no changes"])

    def test_json_report_is_versioned_and_complete(self):
        model, selection = self.select("packages/core-ui/src/Button.tsx")
        report = json_report(model, selection)
        self.assertEqual(report["schema"], 1)
        self.assertEqual(report["base"]["scope"], "files")
        self.assertEqual(report["fallback"], {"active": False, "reasons": []})
        portal = next(unit for unit in report["units"] if unit["id"] == "portal")
        self.assertEqual(portal["impact"], "dependency")
        self.assertEqual(portal["chain"], ["core-ui", "portal"])
        check = next(c for c in report["checks"] if c["id"] == "jest:portal")
        self.assertEqual(check["cwd"], "packages/portal")
        self.assertEqual(check["command"], "yarn test")
        self.assertTrue(check["selected"])
        self.assertEqual(
            report["summary"]["selected"], {"fast": 3, "slow": 1, "integration": 0}
        )
        self.assertEqual(
            [c["id"] for c in report["checks"]],
            sorted(c["id"] for c in report["checks"]),
        )


class CargoLockTest(unittest.TestCase):
    def test_changed_dependency_selects_its_local_consumers(self):
        new = CARGO_LOCK.replace('checksum = "bbbb"', 'checksum = "cccc"')
        self.assertEqual(cargo_lock_impact(CARGO_LOCK, new), {"service"})

    def test_shared_dependency_reaches_transitive_consumers(self):
        new = CARGO_LOCK.replace('checksum = "aaaa"', 'checksum = "dddd"')
        self.assertEqual(cargo_lock_impact(CARGO_LOCK, new), {"core", "service", "p1"})

    def test_added_dependency_counts_for_the_package_that_gained_it(self):
        new = CARGO_LOCK.replace(
            'dependencies = ["core"]', 'dependencies = ["core", "tokio"]'
        )
        self.assertEqual(cargo_lock_impact(CARGO_LOCK, new), {"p1"})

    def test_removed_package_is_followed_in_the_old_graph(self):
        new = CARGO_LOCK.replace(
            'dependencies = ["core", "tokio"]', 'dependencies = ["core"]'
        )
        new = new.split('[[package]]\nname = "tokio"')[0]
        self.assertEqual(cargo_lock_impact(CARGO_LOCK, new), {"service"})

    def test_unchanged_lock_affects_nothing(self):
        self.assertEqual(cargo_lock_impact(CARGO_LOCK, CARGO_LOCK), set())

    def test_versioned_references_resolve_to_that_version(self):
        old = CARGO_LOCK + textwrap.dedent(
            """
            [[package]]
            name = "serde"
            version = "2.0.0"
            source = "registry+https://github.com/rust-lang/crates.io-index"
            checksum = "eeee"
            """
        )
        # With two versions locked, Cargo writes versioned references.
        old = old.replace('dependencies = ["serde"]', 'dependencies = ["serde 1.0.0"]')
        old = old.replace(
            'dependencies = ["core"]', 'dependencies = ["core", "serde 2.0.0"]'
        )
        new = old.replace('checksum = "eeee"', 'checksum = "ffff"')
        # core uses serde 1.0.0, so only p1 sees the 2.0.0 change.
        self.assertEqual(cargo_lock_impact(old, new), {"p1"})


def git(root, *arguments):
    subprocess.run(["git", *arguments], cwd=root, check=True, capture_output=True)


class GitTest(Fixture):
    def setUp(self):
        super().setUp()
        patcher = mock.patch.dict(os.environ, GIT_ENV)
        patcher.start()
        self.addCleanup(patcher.stop)
        git(self.root, "init", "--quiet", "--initial-branch=main")
        git(self.root, "add", ".")
        git(self.root, "commit", "--quiet", "-m", "base")
        git(self.root, "checkout", "--quiet", "-b", "feature")

    def commit(self, files, message="change"):
        write(self.root, files)
        git(self.root, "add", ".")
        git(self.root, "commit", "--quiet", "-m", message)

    def test_committed_changes_since_the_merge_base(self):
        self.commit({"packages/portal/src/Screen.tsx": "export const a = 1\n"})
        # A later commit on the base branch is not a change of this branch.
        git(self.root, "checkout", "--quiet", "main")
        self.commit({"docs/guide.md": "# Changed on main\n"})
        git(self.root, "checkout", "--quiet", "feature")
        changes = collect(self.root, None, "main", Scope.COMMITTED)
        self.assertEqual(
            changes.changes, [Change("packages/portal/src/Screen.tsx", Status.MODIFIED)]
        )
        self.assertEqual(changes.base_source, BaseSource.DEFAULT)
        self.assertFalse(changes.problems)

    def test_worktree_scope_adds_staged_unstaged_and_untracked_files(self):
        write(self.root, {"packages/portal/src/Screen.tsx": "export const b = 2\n"})
        write(self.root, {"packages/portal/src/New.tsx": "export {}\n"})
        committed = collect(self.root, "main", "main", Scope.COMMITTED)
        self.assertEqual(committed.changes, [])
        worktree = collect(self.root, "main", "main", Scope.WORKTREE)
        self.assertEqual(
            worktree.changes,
            [
                Change("packages/portal/src/New.tsx", Status.UNTRACKED),
                Change("packages/portal/src/Screen.tsx", Status.MODIFIED),
            ],
        )

    def test_renames_count_as_removal_and_addition(self):
        git(self.root, "mv", "docs/guide.md", "packages/portal/guide.txt")
        git(self.root, "commit", "--quiet", "-m", "move")
        changes = collect(self.root, "main", "main", Scope.COMMITTED)
        self.assertEqual(
            changes.changes,
            [
                Change("docs/guide.md", Status.DELETED),
                Change("packages/portal/guide.txt", Status.ADDED),
            ],
        )

    def test_missing_base_falls_back_to_every_check(self):
        changes = collect(self.root, "origin/missing", "main", Scope.COMMITTED)
        self.assertEqual(changes.changes, [])
        self.assertEqual(
            changes.problems,
            ["base origin/missing is not a known commit; fetch it or pass --base"],
        )
        model = self.model()
        selection = model.select(changes)
        self.assertEqual(len(selection.selected()), len(model.checks))

    def test_unrelated_history_has_no_merge_base(self):
        git(self.root, "checkout", "--quiet", "--orphan", "unrelated")
        self.commit({"docs/guide.md": "# Other history\n"}, "unrelated")
        changes = collect(self.root, "main", "main", Scope.COMMITTED)
        self.assertEqual(changes.problems, ["HEAD and main have no merge base"])
        self.assertIsNone(changes.merge_base)

    def test_upstream_is_the_default_unless_it_is_the_same_branch(self):
        git(self.root, "branch", "--quiet", "base-branch", "main")
        git(self.root, "branch", "--quiet", "--set-upstream-to=base-branch")
        self.assertEqual(
            collect(self.root, None, "main", Scope.COMMITTED).base_source,
            BaseSource.UPSTREAM,
        )
        # A branch tracking its own remote copy compares with the default base.
        git(self.root, "remote", "add", "origin", str(self.root))
        git(self.root, "update-ref", "refs/remotes/origin/feature", "HEAD")
        git(self.root, "branch", "--quiet", "--set-upstream-to=origin/feature")
        changes = collect(self.root, None, "main", Scope.COMMITTED)
        self.assertEqual(changes.base_source, BaseSource.DEFAULT)
        self.assertEqual(changes.base_ref, "main")

    def test_cargo_lock_change_is_narrowed_by_the_locked_graph(self):
        self.commit(
            {
                "packages/Cargo.lock": CARGO_LOCK.replace(
                    'checksum = "bbbb"', 'checksum = "cccc"'
                )
            }
        )
        model = self.model()
        selection = model.select(collect(self.root, "main", "main", Scope.COMMITTED))
        claim = selection.files[0].claim
        self.assertEqual(claim.units, ("service",))
        self.assertIn("locked dependencies changed", claim.source)

    def test_worktree_lock_is_read_from_disk(self):
        write(
            self.root,
            {
                "packages/Cargo.lock": CARGO_LOCK.replace(
                    'checksum = "aaaa"', 'checksum = "dddd"'
                )
            },
        )
        model = self.model()
        selection = model.select(collect(self.root, "main", "main", Scope.WORKTREE))
        self.assertEqual(selection.files[0].claim.units, ("core", "p1", "service"))

    def test_unreadable_lock_affects_the_whole_workspace(self):
        self.commit({"packages/Cargo.lock": "not [ toml"})
        model = self.model()
        selection = model.select(collect(self.root, "main", "main", Scope.COMMITTED))
        self.assertIn("unreadable", selection.files[0].claim.source)
        self.assertIn("testkit", selection.files[0].claim.units)

    def test_cli_prints_json_for_ci(self):
        self.commit({"packages/core-ui/src/Button.tsx": "export const c = 3\n"})
        output = io.StringIO()
        with redirect_stdout(output):
            status = cli.main(["--base", "main", "--json"], root=self.root)
        self.assertEqual(status, 0)
        report = json.loads(output.getvalue())
        self.assertEqual(report["base"]["ref"], "main")
        self.assertEqual(report["base"]["source"], "argument")
        self.assertEqual(
            [file["path"] for file in report["files"]],
            ["packages/core-ui/src/Button.tsx"],
        )
        self.assertIn(
            "jest:portal", [c["id"] for c in report["checks"] if c["selected"]]
        )


class ParsingTest(unittest.TestCase):
    def test_name_status_output(self):
        output = "M\0a.txt\0A\0b c.txt\0D\0d.txt\0T\0link\0"
        self.assertEqual(
            parse_name_status(output),
            [
                Change("a.txt", Status.MODIFIED),
                Change("b c.txt", Status.ADDED),
                Change("d.txt", Status.DELETED),
                Change("link", Status.TYPE_CHANGED),
            ],
        )

    def test_listed_files_are_normalized_against_the_callers_directory(self):
        root = Path("/repo")
        changes = from_files(
            root, ["src/a.ts", "../docs/b.md", ""], Path("/repo/packages")
        )
        self.assertEqual(
            [change.path for change in changes.changes],
            ["docs/b.md", "packages/src/a.ts"],
        )
        with self.assertRaisesRegex(ValueError, "outside the repository"):
            from_files(root, ["../../etc/passwd"], Path("/repo/packages"))

    def test_globs(self):
        cases = [
            ("docs/**", "docs/a/b.md", True),
            ("docs/**", "docs", False),
            ("**/*.md", "README.md", True),
            ("**/*.md", "a/b/README.md", True),
            ("packages/*/rust/*.tgz", "packages/a/rust/x.tgz", True),
            ("packages/*/rust/*.tgz", "packages/a/b/rust/x.tgz", False),
            ("packages/Dockerfile*", "packages/Dockerfile.prod", True),
            ("scripts/test_?.py", "scripts/test_a.py", True),
            ("a.b", "axb", False),
        ]
        for pattern, path, expected in cases:
            with self.subTest(pattern=pattern, path=path):
                self.assertIs(Glob(pattern).matches(path), expected)

    def test_globs_must_stay_inside_the_repository(self):
        for pattern in ("/etc/*", "../x", "a/../../b", ""):
            with self.subTest(pattern=pattern), self.assertRaises(ConfigError):
                Glob(pattern)


class ConfigTest(unittest.TestCase):
    def base(self, **check):
        table = {
            "id": "c",
            "kind": "test",
            "runner": "shell",
            "cost": "fast",
            "cwd": ".",
            "command": "true",
        }
        table.update(check)
        return {"schema": 1, "default_base": "main", "checks": [table]}

    def test_valid_control(self):
        config = parse_config(self.base())
        (check,) = expand_checks(config.checks, {})
        self.assertEqual(check.cost, Cost.FAST)
        self.assertIsNone(check.owner)

    def test_unknown_keys_and_values_are_rejected(self):
        with self.assertRaisesRegex(ConfigError, "unknown keys typo"):
            parse_config(self.base(typo=1))
        with self.assertRaisesRegex(ConfigError, "'cost' must be one of"):
            expand_checks(parse_config(self.base(cost="cheap")).checks, {})
        with self.assertRaisesRegex(ConfigError, "unknown requirement"):
            expand_checks(parse_config(self.base(requires=["gpu"])).checks, {})
        with self.assertRaisesRegex(ConfigError, "schema must be 1"):
            parse_config({**self.base(), "schema": 2})

    def test_families_need_the_unit_field_and_known_members(self):
        config = parse_config(self.base(for_each=["a"]))
        with self.assertRaisesRegex(ConfigError, "must contain"):
            expand_checks(config.checks, {"a": "packages/a"})
        config = parse_config(self.base(id="t:{unit}", for_each=["missing"]))
        with self.assertRaisesRegex(ConfigError, "unknown unit 'missing'"):
            expand_checks(config.checks, {})

    def test_family_overrides_must_name_members(self):
        config = parse_config(
            self.base(id="t:{unit}", for_each=["a"], commands={"b": "x"})
        )
        with self.assertRaisesRegex(ConfigError, "outside for_each: b"):
            expand_checks(config.checks, {"a": "packages/a", "b": "packages/b"})

    def test_family_fills_unit_and_path(self):
        config = parse_config(
            self.base(
                id="t:{unit}",
                for_each=["a", "b"],
                cwd="{path}",
                command="run {unit}",
                commands={"b": "special {path}"},
            )
        )
        checks = expand_checks(config.checks, {"a": "packages/a", "b": "packages/b"})
        self.assertEqual(
            [(c.id, c.owner, c.cwd, c.command) for c in checks],
            [
                ("t:a", "a", "packages/a", "run a"),
                ("t:b", "b", "packages/b", "special packages/b"),
            ],
        )

    def test_placeholders_outside_families_are_rejected(self):
        with self.assertRaisesRegex(ConfigError, "need for_each"):
            expand_checks(parse_config(self.base(command="run {unit}")).checks, {})

    def test_duplicate_ids_are_rejected(self):
        config = parse_config(self.base())
        with self.assertRaisesRegex(ConfigError, "duplicate check id"):
            expand_checks(config.checks + config.checks, {})


class RepositoryModelTest(unittest.TestCase):
    """The committed model against this checkout."""

    @classmethod
    def setUpClass(cls):
        cls.model = load_model(ROOT)

    def select(self, *paths):
        selection = self.model.select(from_files(ROOT, paths, ROOT))
        return selection, {decision.check.id for decision in selection.selected()}

    def test_every_tracked_file_is_claimed(self):
        output = subprocess.run(
            ["git", "ls-files", "-z"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        unknown = [
            path
            for path in output.split("\0")
            if path and self.model.claim(path) is None
        ]
        self.assertEqual(
            unknown, [], "add a [[paths]] rule to scripts/dev/affected.toml"
        )

    def test_checks_name_existing_workflows_and_actions(self):
        for check in self.model.checks:
            for workflow in check.workflows:
                self.assertTrue(
                    (ROOT / ".github/workflows" / workflow).is_file(),
                    (check.id, workflow),
                )
            for action in check.actions:
                self.assertTrue((ROOT / ".github/actions" / action).is_dir(), check.id)

    def test_documentation_only_change(self):
        selection, chosen = self.select(
            "docs/docusaurus/docs/07-developers/03-development-environment/fast-feedback.md"
        )
        self.assertEqual(chosen, {"docs-build", "reuse"})
        self.assertFalse(selection.fallback)

    def test_shared_component_change(self):
        _, chosen = self.select(
            "packages/ui-essentials/src/components/Header/Header.tsx"
        )
        self.assertTrue(
            {"jest:ui-essentials", "jest:voting-portal", "stories:voting-portal"}
            <= chosen
        )
        self.assertFalse(any(check.startswith("cargo-test:") for check in chosen))
        self.assertNotIn("lint:voting-portal", chosen)

    def test_sequent_core_change_checks_the_committed_package(self):
        _, chosen = self.select("packages/sequent-core/src/lib.rs")
        self.assertTrue(
            {"cargo-test:sequent-core", "cargo-test:windmill", "wasm-freshness"}
            <= chosen
        )
        # Frontends test the committed tgz; its regeneration selects them.
        self.assertNotIn("jest:voting-portal", chosen)
        _, chosen = self.select("packages/voting-portal/rust/sequent-core-0.1.0.tgz")
        self.assertTrue(
            {"jest:voting-portal", "vitest:workbench", "wasm-freshness"} <= chosen
        )

    def test_workbench_stories_run_in_the_voting_portal_storybook(self):
        _, chosen = self.select("packages/workbench/src/components/PolicyPanel.tsx")
        self.assertIn("stories:voting-portal", chosen)
        self.assertNotIn("jest:voting-portal", chosen)

    def test_fixture_change_reaches_stories_journeys_and_workbench(self):
        _, chosen = self.select("packages/ui-test-kit/fixtures/scenarios.ts")
        self.assertTrue(
            {
                "contracts:ui-test-kit",
                "stories:voting-portal",
                "journeys:admin-portal",
                "smoke:workbench",
            }
            <= chosen
        )

    def test_selection_model_change_selects_every_check(self):
        selection, chosen = self.select("scripts/dev/affected.toml")
        self.assertEqual(len(chosen), len(self.model.checks))
        self.assertEqual(
            selection.fallback,
            ["the selection model changed: scripts/dev/affected.toml"],
        )


if __name__ == "__main__":
    unittest.main()
