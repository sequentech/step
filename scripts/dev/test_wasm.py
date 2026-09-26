# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Exercise the WASM build decisions against fake Cargo and wasm-bindgen tools.

Each test gets its own checkout. The fake cargo derives the module from the Rust
sources with comments removed, so a comment-only edit rebuilds to identical output,
and fails on ``compile_error!``. Real compilation is covered by running the command
in the devcontainer.
"""

import contextlib
import hashlib
import io
import json
import os
import shutil
import subprocess
import tarfile
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest.mock import patch

import wasm

FAKE_TOOL = r"""#!/usr/bin/env python3
import hashlib, json, os, sys
from pathlib import Path

tool = Path(sys.argv[0]).name
args = sys.argv[1:]
fixtures = Path(os.environ["FAKE_WASM_FIXTURES"])
with (fixtures / "calls.log").open("a") as log:
    log.write(json.dumps({"tool": tool, "args": args, "env": {
        name: os.environ.get(name)
        for name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_TARGET_DIR",
                     "CARGO_PROFILE_RELEASE_LTO", "CARGO_PROFILE_RELEASE_INCREMENTAL")
    }}) + "\n")
if tool == "rustc":
    print("rustc 1.96.0 (fake)\nrelease: " + os.environ.get("FAKE_RUSTC", "1.96.0"))
elif tool == "wasm-opt":
    if args == ["--version"]:
        print("wasm-opt version 123")
    else:
        source, output = Path(args[0]), Path(args[args.index("-o") + 1])
        output.write_bytes(source.read_bytes() + b"optimised")
elif tool == "wasm-bindgen":
    if args == ["--version"]:
        print("wasm-bindgen " + os.environ.get("FAKE_BINDGEN", "0.2.128"))
    else:
        module = Path(args[0]).read_bytes()
        out = Path(args[args.index("--out-dir") + 1])
        out.mkdir(parents=True, exist_ok=True)
        (out / "index_bg.wasm").write_bytes(module)
        digest = hashlib.sha256(module).hexdigest()
        (out / "index.js").write_text("export default function init() {}\n// " + digest)
        (out / "index.d.ts").write_text("export default function init(): void\n")
        (out / "index_bg.wasm.d.ts").write_text("")
elif args[0] == "-V":
    print("cargo 1.96.0 (fake)")
elif args[0] == "tree":
    print((fixtures / "tree.txt").read_text())
elif args[0] == "metadata":
    print((fixtures / "metadata.json").read_text())
elif args[0] == "build":
    root = Path(os.environ["FAKE_WASM_ROOT"])
    digest = hashlib.sha256()
    for crate in ("sequent-core", "strand"):
        for path in sorted((root / "packages" / crate / "src").rglob("*.rs")):
            text = path.read_text()
            if "compile_error!" in text:
                print("error: fake compile error in " + path.name, file=sys.stderr)
                sys.exit(101)
            code = [line for line in text.splitlines() if not line.startswith("//")]
            digest.update("\n".join(code).encode())
    target = Path(os.environ["CARGO_TARGET_DIR"]) / "wasm32-unknown-unknown" / "release"
    target.mkdir(parents=True, exist_ok=True)
    (target / "sequent_core.wasm").write_bytes(b"\0asm" + digest.digest())
    print("   Compiling sequent-core v0.1.0", file=sys.stderr)
"""

TREE = """\
sequent-core v0.1.0 ({root}/packages/sequent-core)|areas,default,default_features,\
wasm,wasmtest
serde v1.0.228|default,derive,serde_derive,std
serde_derive v1.0.228 (proc-macro)|default
strand v0.4.0 ({root}/packages/strand)|num_bigint,wasm,wasmtest
serde v1.0.228|default,derive,serde_derive,std
wasm-bindgen v0.2.128|default,serde-serialize
serde v1.0.228|default,derive,serde_derive,std (*)
celery v0.5.5 (https://github.com/Findeton/rusty-celery.git?rev=b41459#b4145925)|
"""

LOCK = """\
# yarn lockfile v1


"sequent-core@file:./admin-portal/rust/sequent-core-0.1.0.tgz":
  version "0.1.0"
  resolved "file:./admin-portal/rust/sequent-core-0.1.0.tgz#{hash}"

"sequent-core@file:./ballot-verifier/rust/sequent-core-0.1.0.tgz":
  version "0.1.0"
  resolved "file:./ballot-verifier/rust/sequent-core-0.1.0.tgz#{hash}"

"sequent-core@file:./voting-portal/rust/sequent-core-0.1.0.tgz":
  version "0.1.0"
  resolved "file:./voting-portal/rust/sequent-core-0.1.0.tgz#{hash}"

serde@^1:
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/serde/-/serde-1.0.0.tgz#{other}"
"""


def metadata(root: Path) -> dict:
    def package(name, version, targets, **extra):
        return {
            "name": name,
            "version": version,
            "manifest_path": str(root / "packages" / name / "Cargo.toml"),
            "targets": targets,
            **extra,
        }

    def target(name, kinds, path):
        return {"name": name, "kind": kinds, "src_path": str(root / "packages" / path)}

    return {
        "packages": [
            package(
                "sequent-core",
                "0.1.0",
                [
                    target(
                        "sequent_core", ["cdylib", "rlib"], "sequent-core/src/lib.rs"
                    ),
                    target("sequent-core", ["bin"], "sequent-core/src/main.rs"),
                    target("it", ["test"], "sequent-core/tests/it.rs"),
                ],
                authors=["Felix Robles <felix@sequentech.io>"],
                description=None,
                license="AGPL-3.0-only",
                license_file=None,
                repository=None,
                homepage=None,
                keywords=[],
                readme="README.md",
            ),
            package(
                "strand",
                "0.4.0",
                [target("strand", ["cdylib", "rlib"], "strand/src/lib.rs")],
            ),
        ]
    }


class Checkout:
    """A synthetic repository with the two local crates the WASM build compiles."""

    def __init__(self, root: Path):
        self.root = root
        self.packages = root / "packages"
        self.fixtures = root.parent / f"{root.name}-fixtures"
        self.fixtures.mkdir(parents=True)
        files = {
            "rust-toolchain.toml": '[toolchain]\nchannel = "1.96.0"\n',
            "packages/Cargo.toml": (
                '[workspace]\nmembers = ["sequent-core", "strand"]\n\n'
                "[profile.release]\nopt-level = 3\n"
            ),
            "packages/Cargo.lock": "version = 4\n",
            "packages/.cargo/config.toml": "[net]\nretry = 2\n",
            "packages/sequent-core/Cargo.toml": '[package]\nname = "sequent-core"\n',
            "packages/sequent-core/README.md": "# sequent-core\n",
            "packages/sequent-core/LICENSE": "AGPL\n",
            "packages/sequent-core/src/lib.rs": "pub mod util;\n",
            "packages/sequent-core/src/main.rs": "fn main() {}\n",
            "packages/sequent-core/src/util.rs": "pub fn hash() -> u8 { 1 }\n",
            "packages/sequent-core/tests/it.rs": "#[test]\nfn it() {}\n",
            "packages/strand/Cargo.toml": '[package]\nname = "strand"\n',
            "packages/strand/README.md": "# strand\n",
            "packages/strand/src/lib.rs": (
                '#[cfg(test)]\n#[path = "../tests/support/shape.rs"]\nmod shape;\n'
            ),
            "packages/strand/tests/support/shape.rs": "fn shape() {}\n",
            "packages/yarn.lock": LOCK.format(hash="0" * 40, other="f" * 40),
        }
        for name, text in files.items():
            self.write(name, text)
        for consumer in wasm.CONSUMERS:
            (self.packages / consumer / "rust").mkdir(parents=True)
        self.set_tree(TREE)
        (self.fixtures / "metadata.json").write_text(json.dumps(metadata(root)))
        self.bin = self.fixtures / "bin"
        self.bin.mkdir()
        tool = self.bin / "fake-tool"
        tool.write_text(FAKE_TOOL)
        tool.chmod(0o755)
        for name in ("cargo", "rustc", "wasm-bindgen", "wasm-opt"):
            (self.bin / name).symlink_to(tool)
        self.layout = wasm.Layout(root)

    def write(self, name: str, text: str) -> Path:
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        return path

    def set_tree(self, tree: str) -> None:
        (self.fixtures / "tree.txt").write_text(tree.format(root=self.root))

    def calls(self, tool: str, command: str | None = None) -> list[dict]:
        log = self.fixtures / "calls.log"
        if not log.exists():
            return []
        calls = [json.loads(line) for line in log.read_text().splitlines()]
        return [
            call
            for call in calls
            if call["tool"] == tool
            and (command is None or call["args"][:1] == [command])
        ]

    def environment(self, **extra: str) -> dict[str, str]:
        env = {
            key: value
            for key, value in os.environ.items()
            if key not in ("RUSTFLAGS", "CARGO_HOME")
        }
        env.update(
            PATH=f"{self.bin}{os.pathsep}{os.environ['PATH']}",
            FAKE_WASM_FIXTURES=str(self.fixtures),
            FAKE_WASM_ROOT=str(self.root),
            CARGO_HOME=str(self.fixtures / "cargo-home"),
        )
        env.update(extra)
        return env

    def run(self, *args: str, **env: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with (
            patch.dict(os.environ, self.environment(**env), clear=True),
            contextlib.redirect_stdout(out),
            contextlib.redirect_stderr(err),
        ):
            code = wasm.main(list(args), self.layout)
        return code, out.getvalue(), err.getvalue()

    def source_fingerprint(self) -> str:
        with patch.dict(os.environ, self.environment(), clear=True):
            closure = wasm.load_closure(self.layout, "cargo 1.96.0 (fake)")
            return wasm.digest_records(wasm.source_records(self.layout, closure))

    def entry(self) -> str:
        return (self.layout.dist / "index.js").read_text()

    def status(self) -> dict:
        return json.loads((self.layout.dist / "status.json").read_text())


class WasmTestCase(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.checkout = Checkout(self.base / "step")


class ClosureTests(WasmTestCase):
    def test_tree_separates_path_packages_from_dependencies(self):
        local, external = wasm.parse_tree(
            TREE.format(root=self.checkout.root), self.checkout.layout
        )
        self.assertEqual(
            local,
            {
                "packages/sequent-core": "areas,default,default_features,wasm,wasmtest",
                "packages/strand": "num_bigint,wasm,wasmtest",
            },
        )
        # Duplicates collapse, the proc-macro and repeat markers are not part of
        # a package's identity, and a git source stays in it.
        self.assertEqual(
            external,
            [
                "celery 0.5.5 https://github.com/Findeton/rusty-celery.git?rev=b41459"
                "#b4145925|",
                "serde 1.0.228|default,derive,serde_derive,std",
                "serde_derive 1.0.228|default",
                "wasm-bindgen 0.2.128|default,serde-serialize",
            ],
        )

    def test_unexpected_tree_output_is_rejected(self):
        with self.assertRaisesRegex(wasm.WasmError, "unexpected cargo tree line"):
            wasm.parse_tree("warning: something\n", self.checkout.layout)

    def test_library_sources_exclude_other_targets(self):
        package = metadata(self.checkout.root)["packages"][0]
        self.assertEqual(
            wasm.package_sources(package, self.checkout.layout),
            ("packages/sequent-core/src", None, "sequent_core"),
        )

    def test_a_build_script_makes_the_whole_package_an_input(self):
        package = metadata(self.checkout.root)["packages"][1]
        package["targets"].append(
            {
                "name": "build-script-build",
                "kind": ["custom-build"],
                "src_path": str(self.checkout.packages / "strand" / "build.rs"),
            }
        )
        self.assertEqual(
            wasm.package_sources(package, self.checkout.layout),
            ("packages/strand", "packages/strand/build.rs", "strand"),
        )

    def test_closure_is_reused_until_a_resolution_input_changes(self):
        self.checkout.source_fingerprint()
        self.checkout.source_fingerprint()
        self.assertEqual(len(self.checkout.calls("cargo", "tree")), 1)
        self.checkout.write(
            "packages/strand/Cargo.toml", '[package]\nname = "strand"\n#\n'
        )
        self.checkout.source_fingerprint()
        self.assertEqual(len(self.checkout.calls("cargo", "tree")), 2)


class FingerprintTests(WasmTestCase):
    def test_library_edits_change_the_fingerprint(self):
        before = self.checkout.source_fingerprint()
        self.checkout.write(
            "packages/sequent-core/src/util.rs", "pub fn hash() -> u8 { 2 }\n"
        )
        self.assertNotEqual(self.checkout.source_fingerprint(), before)

    def test_files_outside_the_library_build_are_ignored(self):
        before = self.checkout.source_fingerprint()
        self.checkout.write(
            "packages/sequent-core/tests/it.rs", "#[test]\nfn other() {}\n"
        )
        self.checkout.write("packages/sequent-core/src/.util.rs.swp", "swap")
        self.checkout.write("packages/sequent-core/src/util.rs~", "backup")
        self.checkout.write("packages/strand/README.md", "# strand, edited\n")
        self.assertEqual(self.checkout.source_fingerprint(), before)

    def test_files_named_by_path_attributes_are_inputs(self):
        before = self.checkout.source_fingerprint()
        self.checkout.write("packages/strand/tests/support/shape.rs", "fn other() {}\n")
        self.assertNotEqual(self.checkout.source_fingerprint(), before)

    def test_packaged_readme_is_an_input(self):
        before = self.checkout.source_fingerprint()
        self.checkout.write("packages/sequent-core/README.md", "# edited\n")
        self.assertNotEqual(self.checkout.source_fingerprint(), before)

    def test_profile_and_config_changes_are_inputs(self):
        before = self.checkout.source_fingerprint()
        self.checkout.write(
            "packages/Cargo.toml",
            '[workspace]\nmembers = ["sequent-core", "strand"]\n\n'
            "[profile.release]\nopt-level = 2\n",
        )
        profile = self.checkout.source_fingerprint()
        self.assertNotEqual(profile, before)
        self.checkout.write("packages/.cargo/config.toml", "[net]\nretry = 3\n")
        self.assertNotEqual(self.checkout.source_fingerprint(), profile)

    def test_workspace_membership_and_unrelated_lock_entries_are_not_inputs(self):
        before = self.checkout.source_fingerprint()
        self.checkout.write(
            "packages/Cargo.toml",
            '[workspace]\nmembers = ["sequent-core", "strand", "windmill"]\n\n'
            "[profile.release]\nopt-level = 3\n",
        )
        self.checkout.write("packages/Cargo.lock", "version = 4\n# windmill bumped\n")
        self.assertEqual(self.checkout.source_fingerprint(), before)

    def test_a_resolved_dependency_change_is_an_input(self):
        before = self.checkout.source_fingerprint()
        self.checkout.set_tree(TREE.replace("serde v1.0.228", "serde v1.0.229"))
        self.checkout.write("packages/Cargo.lock", "version = 4\n# serde bumped\n")
        self.assertNotEqual(self.checkout.source_fingerprint(), before)

    def test_the_fingerprint_does_not_depend_on_the_checkout_path(self):
        other = Checkout(self.base / "elsewhere" / "step")
        self.assertEqual(other.source_fingerprint(), self.checkout.source_fingerprint())


class DevelopmentBuildTests(WasmTestCase):
    def build(self, **env: str) -> tuple[int, str, str]:
        return self.checkout.run(**env)

    def test_first_build_publishes_and_a_second_run_does_nothing(self):
        code, out, _ = self.build()
        self.assertEqual(code, 0)
        build = self.checkout.status()["published"]["build"]
        self.assertIn(f'from "./builds/{build}/index.js"', self.checkout.entry())
        for name in wasm.PACKAGE_FILES:
            self.assertTrue((self.checkout.layout.builds / build / name).is_file())
        self.assertIn(f"published build {build}", out)

        code, out, _ = self.build()
        self.assertEqual(code, 0)
        self.assertIn("up to date", out)
        self.assertEqual(len(self.checkout.calls("cargo", "build")), 1)
        self.assertEqual(len(self.checkout.calls("wasm-bindgen")), 3)

    def test_an_edit_republishes_and_keeps_the_previous_build(self):
        self.build()
        first = self.checkout.status()["published"]["build"]
        self.checkout.write(
            "packages/sequent-core/src/util.rs", "pub fn hash() -> u8 { 2 }\n"
        )
        self.assertEqual(self.build()[0], 0)
        second = self.checkout.status()["published"]["build"]
        self.assertNotEqual(first, second)
        self.assertIn(f"./builds/{second}/index.js", self.checkout.entry())
        builds = {path.name for path in self.checkout.layout.builds.iterdir()}
        self.assertEqual(builds, {first, second})

        self.checkout.write(
            "packages/sequent-core/src/util.rs", "pub fn hash() -> u8 { 3 }\n"
        )
        self.assertEqual(self.build()[0], 0)
        third = self.checkout.status()["published"]["build"]
        builds = {path.name for path in self.checkout.layout.builds.iterdir()}
        self.assertEqual(builds, {second, third})

    def test_identical_output_leaves_the_package_files_alone(self):
        self.build()
        files = [
            self.checkout.layout.dist / name
            for name in ("index.js", "index.d.ts", "package.json", "status.js")
        ]
        before = [(path.read_bytes(), path.stat().st_mtime_ns) for path in files]
        self.checkout.write(
            "packages/sequent-core/src/util.rs",
            "// comment\npub fn hash() -> u8 { 1 }\n",
        )
        code, out, _ = self.build()
        self.assertEqual(code, 0)
        self.assertIn("rebuilt identical output", out)
        # Watchers rebuild on any rewrite, so nothing they load may be touched.
        after = [(path.read_bytes(), path.stat().st_mtime_ns) for path in files]
        self.assertEqual(after, before)
        self.assertIn("up to date", self.build()[1])

    def test_a_failed_build_keeps_the_published_package_and_says_so(self):
        self.build()
        published = self.checkout.status()["published"]["build"]
        files = {
            path: path.read_bytes()
            for path in self.checkout.layout.dist.rglob("*")
            if path.is_file() and path.name not in ("status.json", "status.js")
        }
        self.checkout.write(
            "packages/sequent-core/src/util.rs", 'compile_error!("x");\n'
        )
        code, out, err = self.build()
        self.assertEqual(code, 1)
        self.assertNotIn("up to date", out)
        self.assertIn("FAILED", err)
        self.assertIn(f"still serves build {published}", err)
        self.assertIn("fake compile error", err)
        for path, content in files.items():
            self.assertEqual(path.read_bytes(), content, path)
        status = self.checkout.status()
        self.assertEqual(status["state"], "failed")
        self.assertEqual(status["published"]["build"], published)
        self.assertIn(
            "console.error", (self.checkout.layout.dist / "status.js").read_text()
        )
        # Retrying the same broken sources must not report success.
        self.assertEqual(self.build()[0], 1)

        self.checkout.write(
            "packages/sequent-core/src/util.rs", "pub fn hash() -> u8 { 2 }\n"
        )
        self.assertEqual(self.build()[0], 0)
        self.assertEqual(self.checkout.status()["state"], "ok")
        self.assertEqual(
            (self.checkout.layout.dist / "status.js").read_text(), wasm.STATUS_OK
        )

    def test_reverting_a_broken_edit_clears_the_failure_without_compiling(self):
        self.build()
        original = (self.checkout.packages / "sequent-core/src/util.rs").read_text()
        self.checkout.write(
            "packages/sequent-core/src/util.rs", 'compile_error!("x");\n'
        )
        self.assertEqual(self.build()[0], 1)
        self.checkout.write("packages/sequent-core/src/util.rs", original)
        code, out, _ = self.build()
        self.assertEqual(code, 0)
        self.assertIn("up to date", out)
        self.assertEqual(self.checkout.status()["state"], "ok")
        self.assertEqual(len(self.checkout.calls("cargo", "build")), 2)

    def test_a_failure_before_any_publish_leaves_nothing_to_load(self):
        self.checkout.write(
            "packages/sequent-core/src/util.rs", 'compile_error!("x");\n'
        )
        code, _, err = self.build()
        self.assertEqual(code, 1)
        self.assertIn("no development package is published", err)
        self.assertFalse((self.checkout.layout.dist / "index.js").exists())

    def test_the_recipe_owns_rustflags_and_the_target_directory(self):
        self.build(RUSTFLAGS="-Ctarget-cpu=native", CARGO_PROFILE_RELEASE_LTO="true")
        env = self.checkout.calls("cargo", "build")[0]["env"]
        self.assertIsNone(env["RUSTFLAGS"])
        self.assertIsNone(env["CARGO_PROFILE_RELEASE_LTO"])
        self.assertEqual(env["CARGO_PROFILE_RELEASE_INCREMENTAL"], "true")
        cargo_home = self.checkout.fixtures / "cargo-home"
        self.assertEqual(
            env["CARGO_ENCODED_RUSTFLAGS"].split("\x1f"),
            [
                f"--remap-path-prefix={cargo_home}=/cargo",
                f"--remap-path-prefix={self.checkout.root}=/step",
                "-Awarnings",
            ],
        )
        self.assertEqual(
            env["CARGO_TARGET_DIR"],
            str(self.checkout.layout.cargo_target(wasm.Output.DEVELOPMENT)),
        )

    def test_a_compiler_setting_change_rebuilds(self):
        self.build()
        self.assertNotIn("up to date", self.build(CC_wasm32_unknown_unknown="clang")[1])
        self.assertIn("up to date", self.build(CC_wasm32_unknown_unknown="clang")[1])
        self.assertNotIn("up to date", self.build()[1])

    def test_a_mismatched_bindgen_cli_is_reported_before_compiling(self):
        code, _, err = self.build(FAKE_BINDGEN="0.2.100")
        self.assertEqual(code, 1)
        self.assertIn("CLI 0.2.100 cannot bind crate version 0.2.128", err)
        self.assertEqual(self.checkout.calls("cargo", "build"), [])

    def test_status_reports_staleness(self):
        self.assertEqual(self.checkout.run("--status")[0], 1)
        self.build()
        code, out, _ = self.checkout.run("--status")
        self.assertEqual(code, 0)
        self.assertIn("matches the current sources", out)
        self.checkout.write(
            "packages/sequent-core/src/util.rs", "pub fn hash() -> u8 { 2 }\n"
        )
        code, out, _ = self.checkout.run("--status")
        self.assertEqual(code, 1)
        self.assertIn("stale", out)

    def test_clean_removes_the_development_package(self):
        self.build()
        self.assertEqual(self.checkout.run("--clean")[0], 0)
        self.assertFalse(self.checkout.layout.dist.exists())
        self.assertNotIn("up to date", self.build()[1])


class ReleasePackageTests(WasmTestCase):
    def release(self, **env: str) -> tuple[int, str, str]:
        return self.checkout.run("--release-package", **env)

    def archives(self) -> dict[str, bytes]:
        return {
            consumer: self.checkout.layout.archive(
                consumer, "sequent-core-0.1.0.tgz"
            ).read_bytes()
            for consumer in wasm.CONSUMERS
        }

    def test_release_writes_every_consumer_copy_and_the_lockfile(self):
        code, out, _ = self.release()
        self.assertEqual(code, 0, out)
        archives = self.archives()
        self.assertEqual(len(set(archives.values())), 1)
        sha1 = hashlib.sha1(archives["ui-core"]).hexdigest()
        lock = self.checkout.layout.yarn_lock.read_text()
        self.assertEqual(lock, LOCK.format(hash=sha1, other="f" * 40))

        with tarfile.open(
            self.checkout.layout.archive("ui-core", "sequent-core-0.1.0.tgz")
        ) as tar:
            members = {member.name: member for member in tar.getmembers()}
            manifest = json.loads(tar.extractfile("package/package.json").read())
            inputs = json.loads(tar.extractfile(f"package/{wasm.INPUTS_FILE}").read())
            module = tar.extractfile("package/index_bg.wasm").read()
        self.assertEqual(
            set(members),
            {
                f"package/{name}"
                for name in (
                    "LICENSE",
                    "README.md",
                    "build-inputs.json",
                    "index.d.ts",
                    "index.js",
                    "index_bg.wasm",
                    "package.json",
                )
            },
        )
        self.assertEqual(
            {member.mtime for member in members.values()}, {wasm.NPM_EPOCH}
        )
        self.assertEqual(
            manifest,
            {
                "name": "sequent-core",
                "type": "module",
                "collaborators": ["Felix Robles <felix@sequentech.io>"],
                "version": "0.1.0",
                "license": "AGPL-3.0-only",
                "files": [
                    "index_bg.wasm",
                    "index.js",
                    "index.d.ts",
                    "build-inputs.json",
                ],
                "main": "index.js",
                "types": "index.d.ts",
                "sideEffects": ["./snippets/*"],
            },
        )
        self.assertTrue(module.endswith(b"optimised"))
        self.assertEqual(
            inputs["source_fingerprint"], self.checkout.source_fingerprint()
        )
        self.assertEqual(inputs["output"]["wasm_opt"], ["-O"])

    def test_the_package_compiles_with_the_plain_release_profile(self):
        self.release(CARGO_PROFILE_RELEASE_INCREMENTAL="true")
        env = self.checkout.calls("cargo", "build")[0]["env"]
        self.assertIsNone(env["CARGO_PROFILE_RELEASE_INCREMENTAL"])
        self.assertEqual(
            env["CARGO_TARGET_DIR"],
            str(self.checkout.layout.cargo_target(wasm.Output.RELEASE)),
        )

    def test_releasing_unchanged_sources_reproduces_the_archive(self):
        self.release()
        first = self.archives()["ui-core"]
        self.release()
        self.assertEqual(self.archives()["ui-core"], first)

    def test_release_requires_wasm_opt(self):
        (self.checkout.bin / "wasm-opt").unlink()
        code, _, err = self.release(
            PATH=f"{self.checkout.bin}{os.pathsep}/usr/bin:/bin"
        )
        self.assertEqual(code, 1)
        self.assertIn("nix develop ./packages/sequent-core", err)

    def test_release_requires_the_pinned_rust(self):
        code, _, err = self.release(FAKE_RUSTC="1.95.0")
        self.assertEqual(code, 1)
        self.assertIn("must be built with Rust 1.96.0", err)


class CheckPackageTests(WasmTestCase):
    def setUp(self) -> None:
        super().setUp()
        self.assertEqual(self.checkout.run("--release-package")[0], 0)

    def check(self) -> tuple[int, str, str]:
        return self.checkout.run("--check-package")

    def test_a_fresh_package_passes(self):
        code, out, _ = self.check()
        self.assertEqual(code, 0)
        self.assertIn("matches the sources", out)

    def test_a_source_change_names_the_stale_input(self):
        self.checkout.write(
            "packages/sequent-core/src/util.rs", "pub fn hash() -> u8 { 2 }\n"
        )
        code, _, err = self.check()
        self.assertEqual(code, 1)
        self.assertIn("changed file packages/sequent-core/src/util.rs", err)
        self.assertIn("--release-package", err)

    def test_a_new_source_file_is_reported(self):
        self.checkout.write("packages/strand/src/extra.rs", "fn extra() {}\n")
        code, _, err = self.check()
        self.assertEqual(code, 1)
        self.assertIn("added file packages/strand/src/extra.rs", err)

    def test_copies_that_differ_fail(self):
        archive = self.checkout.layout.archive(
            "voting-portal", "sequent-core-0.1.0.tgz"
        )
        archive.write_bytes(archive.read_bytes() + b"\0")
        code, _, err = self.check()
        self.assertEqual(code, 1)
        self.assertIn("the copies differ", err)
        self.assertIn(
            "yarn.lock records ./voting-portal/rust/sequent-core-0.1.0.tgz", err
        )

    def test_a_package_without_recorded_inputs_fails(self):
        entries = wasm.read_archive(
            self.checkout.layout.archive("ui-core", "sequent-core-0.1.0.tgz")
        )
        del entries[wasm.INPUTS_FILE]
        archive = wasm.pack(entries)
        for consumer in wasm.CONSUMERS:
            self.checkout.layout.archive(
                consumer, "sequent-core-0.1.0.tgz"
            ).write_bytes(archive)
        wasm.update_yarn_lock(
            self.checkout.layout,
            "sequent-core-0.1.0.tgz",
            hashlib.sha1(archive).hexdigest(),
        )
        code, _, err = self.check()
        self.assertEqual(code, 1)
        self.assertIn("records no source fingerprint", err)


class PublishingTests(WasmTestCase):
    def test_atomic_writes_leave_no_temporary_files(self):
        target = self.base / "out" / "file.txt"
        wasm.write_atomic(target, b"one")
        wasm.write_atomic(target, b"two")
        self.assertEqual(target.read_bytes(), b"two")
        self.assertEqual(os.listdir(target.parent), ["file.txt"])
        self.assertEqual(target.stat().st_mode & 0o777, 0o644)

    def test_describe_changes_lists_added_removed_and_changed_inputs(self):
        self.assertEqual(
            wasm.describe_changes(
                [["file", "a", "1"], ["file", "b", "1"]],
                [("file", "a", "2"), ("file", "c", "1")],
            ),
            ["changed file a", "removed file b", "added file c"],
        )

    @unittest.skipUnless(shutil.which("node"), "node is not installed")
    def test_the_consumer_helper_loads_the_published_package(self):
        helper = wasm.ROOT / "packages" / "ui-core" / "sequent-core-dev.cjs"
        script = textwrap.dedent(
            f"""
            const helper = require({json.dumps(str(helper))})
            const packages = {json.dumps(str(self.checkout.packages))}
            console.log(JSON.stringify({{
                development: helper.sequentCoreWebpackAlias("development", packages),
                production: helper.sequentCoreWebpackAlias("production", packages),
                vite: helper.sequentCoreViteAlias(packages).map((a) => a.replacement),
                state: helper.sequentCoreDevStatus(packages)?.state,
            }}))
            """
        )

        def run_helper() -> dict:
            result = subprocess.run(
                ["node", "-e", script], capture_output=True, text=True, check=True
            )
            return json.loads(result.stdout.splitlines()[-1])

        self.assertEqual(
            run_helper(), {"development": {}, "production": {}, "vite": []}
        )
        self.assertEqual(self.checkout.run()[0], 0)
        entry = str(self.checkout.layout.dist / "index.js")
        self.assertEqual(
            run_helper(),
            {
                "development": {"sequent-core$": entry},
                "production": {},
                "vite": [entry],
                "state": "ok",
            },
        )


if __name__ == "__main__":
    unittest.main()
