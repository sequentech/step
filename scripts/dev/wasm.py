# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Build sequent-core's WebAssembly package for the frontends.

The default action is the development build. It fingerprints every input of the
package and returns at once when nothing changed; otherwise it compiles into a
persistent Cargo target directory and publishes one development package, which
portal dev servers and Storybook load instead of the installed tgz.
``--release-package`` regenerates the committed tgz files and their yarn.lock
hashes; ``--check-package`` fails when those no longer match the sources.
"""

from __future__ import annotations

import argparse
import contextlib
import datetime
import enum
import fcntl
import gzip
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
from collections.abc import Iterable, Iterator, Mapping, Sequence
from dataclasses import asdict, dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = Path(__file__).resolve()

CRATE = "sequent-core"
TARGET = "wasm32-unknown-unknown"
FEATURES = ("wasmtest", "default_features")
PACKAGE_FILES = ("index.js", "index_bg.wasm", "index.d.ts")
CONSUMERS = ("ui-core", "admin-portal", "voting-portal", "ballot-verifier")
INPUTS_FILE = "build-inputs.json"
BINDGEN = "wasm-bindgen"
SCHEMA = 1
# npm stamps every packed entry with this time; reusing it keeps archives stable.
NPM_EPOCH = 499162500
# The previous build stays on disk for pages that loaded it before a reload.
KEPT_BUILDS = 2
SHORT = 16

# Everything that decides how the crate is compiled. It is part of the source
# fingerprint, so changing it invalidates the development and committed packages.
RECIPE = {
    "crate": CRATE,
    "target": TARGET,
    "profile": "release",
    "features": list(FEATURES),
    # Paths are remapped so the module does not depend on the checkout or Cargo
    # home location. Warnings stay quiet as in the devenv's RUSTFLAGS.
    "rustflags": [
        "--remap-path-prefix={cargo_home}=/cargo",
        "--remap-path-prefix={root}=/step",
        "-Awarnings",
    ],
    "bindgen": ["--typescript", "--target", "web", "--out-name", "index"],
}

# The recipe owns rustflags, the profile and the target directory, so development
# and release builds compile alike whatever the shell exports.
REMOVED_ENV = (
    "RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_INCREMENTAL",
    "CARGO_BUILD_TARGET",
    "CARGO_BUILD_TARGET_DIR",
)
REMOVED_ENV_PREFIXES = ("CARGO_PROFILE_", "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_")
# C compiler settings reach crates such as ring through the cc crate.
COMPILER_ENV = (
    "AR",
    "AR_wasm32-unknown-unknown",
    "AR_wasm32_unknown_unknown",
    "CC",
    "CC_wasm32-unknown-unknown",
    "CC_wasm32_unknown_unknown",
    "CFLAGS",
    "CFLAGS_wasm32-unknown-unknown",
    "CFLAGS_wasm32_unknown_unknown",
    "CRATE_CC_NO_DEFAULTS",
    "TARGET_AR",
    "TARGET_CC",
    "TARGET_CFLAGS",
)

LIB_KINDS = frozenset({"lib", "rlib", "cdylib", "dylib", "staticlib", "proc-macro"})
SKIPPED_DIRS = frozenset({"target", "node_modules", "pkg"})
SKIPPED_SUFFIXES = ("~", ".swp", ".swo", ".bak", ".orig", ".rej", ".rs.bk")
MISSING = "missing"
TREE_LINE = re.compile(r"^(?P<name>\S+) v(?P<version>\S+)(?P<rest>.*)$")
TREE_NOTE = re.compile(r"\(([^()]*)\)")
# cargo tree marks a package it already printed with a trailing "(*)".
TREE_REPEAT = re.compile(r"\s*\(\*\)$")
REFERENCE = re.compile(
    r'(?:include_str|include_bytes)!\s*\(\s*"([^"\\]+)"\s*\)|#\[path\s*=\s*"([^"\\]+)"\]'
)


class Action(enum.Enum):
    BUILD = "build"
    STATUS = "status"
    CLEAN = "clean"
    RELEASE_PACKAGE = "release-package"
    CHECK_PACKAGE = "check-package"


class Output(enum.Enum):
    """How the bound module is processed before it is published."""

    DEVELOPMENT = "development"
    RELEASE = "release"


# wasm-pack's release profile runs wasm-opt -O; development skips it for speed.
WASM_OPT = {Output.DEVELOPMENT: (), Output.RELEASE: ("-O",)}
# Development keeps release semantics but compiles incrementally in many codegen
# units, which cuts a leaf edit's compile about threefold. The package does not.
PROFILE_OVERRIDES = {
    Output.DEVELOPMENT: {
        "CARGO_PROFILE_RELEASE_INCREMENTAL": "true",
        "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "256",
    },
    Output.RELEASE: {},
}


class State(enum.Enum):
    OK = "ok"
    FAILED = "failed"


class WasmError(Exception):
    """A failure the developer can act on; the message says how."""


@dataclass(frozen=True)
class Layout:
    """Where one checkout keeps its sources, packages and build state."""

    root: Path

    @property
    def packages(self) -> Path:
        return self.root / "packages"

    @property
    def crate(self) -> Path:
        return self.packages / CRATE

    @property
    def yarn_lock(self) -> Path:
        return self.packages / "yarn.lock"

    @property
    def state(self) -> Path:
        # rust-local-target is ignored by git, Docker contexts and cargo-watch.
        return self.packages / "rust-local-target" / "sequent-core-wasm"

    @property
    def dist(self) -> Path:
        return self.state / "dist"

    @property
    def builds(self) -> Path:
        return self.dist / "builds"

    @property
    def closure_cache(self) -> Path:
        return self.state / "closure.json"

    @property
    def log(self) -> Path:
        return self.state / "build.log"

    def cargo_target(self, output: Output) -> Path:
        return self.state / f"cargo-{output.value}"

    def archive(self, consumer: str, name: str) -> Path:
        return self.packages / consumer / "rust" / name

    def relative(self, path: Path) -> str:
        try:
            return path.resolve().relative_to(self.root.resolve()).as_posix()
        except ValueError:
            return path.resolve().as_posix()


@dataclass(frozen=True)
class LocalPackage:
    """A path dependency whose files are hashed directly."""

    name: str
    root: str
    sources: str
    build_script: str | None
    features: str


@dataclass(frozen=True)
class Closure:
    """The packages the WASM build compiles, as resolved by Cargo."""

    local: tuple[LocalPackage, ...]
    external: tuple[str, ...]
    lib_name: str
    package: dict[str, object]

    def to_json(self) -> dict[str, object]:
        return {
            "local": [asdict(package) for package in self.local],
            "external": list(self.external),
            "lib_name": self.lib_name,
            "package": self.package,
        }

    @classmethod
    def from_json(cls, data: Mapping[str, object]) -> Closure:
        return cls(
            local=tuple(LocalPackage(**package) for package in data["local"]),
            external=tuple(data["external"]),
            lib_name=str(data["lib_name"]),
            package=dict(data["package"]),
        )

    def dependency_version(self, name: str) -> str | None:
        for spec in self.external:
            package, _, _ = spec.partition("|")
            parts = package.split(" ")
            if parts[0] == name:
                return parts[1]
        return None

    def crate_files(self, root: Path) -> list[Path]:
        """The README and license files wasm-pack copies into the package."""
        readme = self.package.get("readme")
        files = [root / str(readme) if readme else root / "README.md"]
        license_file = self.package.get("license_file")
        if license_file:
            files.append(root / str(license_file))
        else:
            files += sorted(path for path in root.glob("LICENSE*") if path.is_file())
        return files


Record = tuple[str, str, str]


def now() -> str:
    return datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds")


def digest_file(path: Path, algorithm: str = "sha256") -> str:
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, algorithm).hexdigest()


def digest_records(records: Iterable[Record]) -> str:
    digest = hashlib.sha256()
    for record in sorted(set(records)):
        digest.update("\t".join(record).encode() + b"\n")
    return digest.hexdigest()


def canonical(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def write_atomic(path: Path, data: bytes) -> None:
    """Replace ``path`` in one rename, so readers see the old or new content."""
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temporary = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.")
    try:
        with os.fdopen(handle, "wb") as file:
            file.write(data)
            file.flush()
            os.fsync(file.fileno())
        os.chmod(temporary, 0o644)
        os.replace(temporary, path)
    except BaseException:
        with contextlib.suppress(FileNotFoundError):
            os.unlink(temporary)
        raise


def capture(command: Sequence[str], cwd: Path) -> str:
    try:
        result = subprocess.run(
            command, cwd=cwd, check=False, capture_output=True, text=True
        )
    except FileNotFoundError as error:
        raise WasmError(
            f"{command[0]} not found; run inside the devenv shell (devenv shell)"
        ) from error
    if result.returncode != 0:
        command_line = " ".join(command)
        raise WasmError(
            f"{command_line} failed ({result.returncode}):\n{result.stderr.strip()}"
        )
    return result.stdout


def stream(
    command: Sequence[str], cwd: Path, env: Mapping[str, str], log: io.TextIOBase
) -> None:
    """Run a build step, echoing its output to the terminal and the build log."""
    log.write(f"$ {' '.join(command)}\n")
    log.flush()
    try:
        process = subprocess.Popen(
            command,
            cwd=cwd,
            env=dict(env),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            errors="replace",
        )
    except FileNotFoundError as error:
        raise WasmError(
            f"{command[0]} not found; run inside the devenv shell (devenv shell)"
        ) from error
    assert process.stdout is not None
    for line in process.stdout:
        sys.stderr.write(line)
        log.write(line)
    if process.wait() != 0:
        log.flush()
        raise WasmError(f"{Path(command[0]).name} exited with {process.returncode}")


def parse_tree(output: str, layout: Layout) -> tuple[dict[str, str], list[str]]:
    """Split ``cargo tree --format '{p}|{f}'`` into local roots and dependencies.

    Returns the features of each local package by root, and one line per external
    package, version, source and feature set. Host and target copies of a package
    appear separately when their features differ.
    """
    local: dict[str, str] = {}
    external: set[str] = set()
    for line in output.splitlines():
        if not line.strip():
            continue
        spec, _, features = TREE_REPEAT.sub("", line).rpartition("|")
        match = TREE_LINE.match(spec.strip())
        if match is None:
            raise WasmError(f"unexpected cargo tree line: {line}")
        notes = [
            note for note in TREE_NOTE.findall(match["rest"]) if note != "proc-macro"
        ]
        path = next((note for note in notes if note.startswith("/")), None)
        if path is not None:
            local[layout.relative(Path(path))] = features
            continue
        source = " ".join(notes)
        external.add(
            f"{match['name']} {match['version']} {source}".strip() + f"|{features}"
        )
    return local, sorted(external)


def package_sources(
    package: Mapping[str, object], layout: Layout
) -> tuple[str, str | None, str | None]:
    """Return the source directory, build script and library name of a package."""
    root = Path(str(package["manifest_path"])).parent
    library = None
    build_script = None
    sources = root
    for target in package["targets"]:
        kinds = set(target["kind"])
        if kinds & LIB_KINDS:
            library = str(target["name"])
            sources = Path(str(target["src_path"])).parent
        if "custom-build" in kinds:
            build_script = layout.relative(Path(str(target["src_path"])))
    if build_script is not None:
        # A build script may read any file of its package.
        sources = root
    return layout.relative(sources), build_script, library


def resolve_closure(layout: Layout) -> Closure:
    tree = capture(
        [
            "cargo",
            "tree",
            "--locked",
            "--color",
            "never",
            "--package",
            CRATE,
            "--target",
            TARGET,
            "--features",
            ",".join(FEATURES),
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}|{f}",
        ],
        layout.crate,
    )
    metadata = json.loads(
        capture(
            ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
            layout.crate,
        )
    )
    members = {
        layout.relative(Path(str(package["manifest_path"])).parent): package
        for package in metadata["packages"]
    }
    local_features, external = parse_tree(tree, layout)
    local = []
    lib_name = None
    crate_package: dict[str, object] = {}
    for root, features in sorted(local_features.items()):
        member = members.get(root)
        if member is None:
            # Not a workspace member, so its layout is unknown: hash all of it.
            local.append(LocalPackage(Path(root).name, root, root, None, features))
            continue
        sources, build_script, library = package_sources(member, layout)
        local.append(
            LocalPackage(str(member["name"]), root, sources, build_script, features)
        )
        if member["name"] == CRATE:
            lib_name = library
            crate_package = {
                key: member.get(key)
                for key in (
                    "name",
                    "version",
                    "authors",
                    "description",
                    "license",
                    "license_file",
                    "repository",
                    "homepage",
                    "keywords",
                    "readme",
                )
            }
    if lib_name is None:
        raise WasmError(f"cargo tree did not report the {CRATE} library")
    return Closure(tuple(local), tuple(external), lib_name, crate_package)


def config_files(layout: Layout) -> list[Path]:
    """Cargo configuration and toolchain files that apply to the crate directory."""
    found = []
    directory = layout.crate
    while True:
        for name in (
            ".cargo/config.toml",
            ".cargo/config",
            "rust-toolchain.toml",
            "rust-toolchain",
        ):
            candidate = directory / name
            if candidate.is_file():
                found.append(candidate)
        if directory == layout.root or directory.parent == directory:
            return found
        directory = directory.parent


def resolution_key(layout: Layout, closure_roots: Iterable[str], cargo: str) -> str:
    paths = [
        layout.packages / "Cargo.lock",
        layout.packages / "Cargo.toml",
        *config_files(layout),
        *(layout.root / root / "Cargo.toml" for root in closure_roots),
    ]
    records = [("file", layout.relative(path), file_state(path)) for path in paths]
    records.append(("recipe", "", canonical(RECIPE)))
    records.append(("tool", "cargo", cargo))
    return digest_records(records)


def load_closure(layout: Layout, cargo: str) -> Closure:
    """Reuse the resolved closure while Cargo's inputs to resolution are unchanged."""
    with contextlib.suppress(FileNotFoundError, ValueError, KeyError, TypeError):
        cached = json.loads(layout.closure_cache.read_text())
        closure = Closure.from_json(cached["closure"])
        roots = [package.root for package in closure.local]
        if cached["key"] == resolution_key(layout, roots, cargo):
            return closure
    closure = resolve_closure(layout)
    key = resolution_key(layout, [package.root for package in closure.local], cargo)
    write_atomic(
        layout.closure_cache,
        json.dumps({"key": key, "closure": closure.to_json()}, indent=2).encode(),
    )
    return closure


def file_state(path: Path) -> str:
    return digest_file(path) if path.is_file() else MISSING


def skipped(name: str) -> bool:
    return name.startswith(".") or name.endswith(SKIPPED_SUFFIXES)


def walk(directory: Path) -> Iterator[Path]:
    for current, dirs, files in os.walk(directory):
        base = Path(current)
        dirs[:] = sorted(
            name
            for name in dirs
            if not skipped(name)
            and name not in SKIPPED_DIRS
            and not re.fullmatch(r"rust-.*-target", name)
            # Nested packages are separate closure members, if they are any.
            and not (base / name / "Cargo.toml").is_file()
        )
        for name in sorted(files):
            if not skipped(name):
                yield base / name


def referenced_files(source: Path) -> Iterator[Path]:
    """Files a Rust source names through include_str!, include_bytes! or #[path]."""
    try:
        text = source.read_text(errors="replace")
    except OSError:
        return
    for match in REFERENCE.finditer(text):
        reference = match.group(1) or match.group(2)
        yield Path(os.path.normpath(source.parent / reference))


def package_files(layout: Layout, package: LocalPackage, closure: Closure) -> set[Path]:
    root = layout.root / package.root
    sources = layout.root / package.sources
    files = {root / "Cargo.toml", *walk(sources)}
    if package.build_script is not None:
        files.add(layout.root / package.build_script)
    for source in [path for path in files if path.suffix == ".rs"]:
        files.update(referenced_files(source))
    if package.name == CRATE:
        files.update(closure.crate_files(root))
    return files


def workspace_settings(layout: Layout) -> str:
    """Workspace manifest tables that change compilation; members are not among them."""
    manifest = tomllib.loads((layout.packages / "Cargo.toml").read_text())
    workspace = manifest.get("workspace", {})
    return canonical(
        {
            "profile": manifest.get("profile", {}),
            "package": workspace.get("package", {}),
        }
    )


def source_records(layout: Layout, closure: Closure) -> list[Record]:
    """The inputs shared by every build of the package, independent of the machine."""
    records: list[Record] = [
        ("recipe", "", canonical(RECIPE)),
        ("workspace", "packages/Cargo.toml", workspace_settings(layout)),
    ]
    records += [
        ("file", layout.relative(path), file_state(path))
        for path in config_files(layout)
    ]
    for spec in closure.external:
        package, _, features = spec.partition("|")
        records.append(("dependency", package, features))
    for package in closure.local:
        records.append(("package", package.root, package.features))
        records += [
            ("file", layout.relative(path), file_state(path))
            for path in package_files(layout, package, closure)
        ]
    return sorted(set(records))


def cargo_home() -> Path:
    return Path(os.environ.get("CARGO_HOME") or Path.home() / ".cargo")


def environment_records(tools: Mapping[str, str], output: Output) -> list[Record]:
    """What else a build on this machine depends on: tools, compilers and itself."""
    records: list[Record] = [("tool", name, version) for name, version in tools.items()]
    records += [
        ("env", name, os.environ[name]) for name in COMPILER_ENV if name in os.environ
    ]
    for name in ("config.toml", "config"):
        path = cargo_home() / name
        if path.is_file():
            records.append(("file", str(path), digest_file(path)))
    records.append(("output", output.value, " ".join(WASM_OPT[output])))
    records += [
        ("profile", name, value) for name, value in PROFILE_OVERRIDES[output].items()
    ]
    records.append(("script", "scripts/dev/wasm.py", digest_file(SCRIPT)))
    return records


def probe_tools(layout: Layout, output: Output) -> dict[str, str]:
    tools = {
        "rustc": capture(["rustc", "-vV"], layout.crate).strip(),
        "cargo": capture(["cargo", "-V"], layout.crate).strip(),
        BINDGEN: capture([BINDGEN, "--version"], layout.crate).strip(),
    }
    if WASM_OPT[output]:
        try:
            tools["wasm-opt"] = capture(["wasm-opt", "--version"], layout.crate).strip()
        except WasmError as error:
            raise WasmError(
                "wasm-opt is required for the release package; run inside the "
                "sequent-core flake shell: nix develop ./packages/sequent-core"
            ) from error
    return tools


def check_tools(closure: Closure, tools: Mapping[str, str]) -> None:
    crate_version = closure.dependency_version(BINDGEN)
    cli_version = tools[BINDGEN].split()[-1]
    if crate_version is not None and crate_version != cli_version:
        # The CLI and the crate share an unversioned schema; they must match exactly.
        raise WasmError(
            f"{BINDGEN} CLI {cli_version} cannot bind crate version {crate_version}; "
            "use the pinned CLI from the devenv shell"
        )


def check_release_toolchain(layout: Layout, tools: Mapping[str, str]) -> None:
    toolchain = layout.root / "rust-toolchain.toml"
    if not toolchain.is_file():
        return
    channel = str(tomllib.loads(toolchain.read_text())["toolchain"]["channel"])
    release = re.search(r"^release: (\S+)$", tools["rustc"], re.M)
    if re.fullmatch(r"[0-9.]+", channel) and (release is None or release[1] != channel):
        raise WasmError(
            f"the release package must be built with Rust {channel}; "
            f"found {release[1] if release else 'an unknown rustc'}"
        )


def cargo_env(layout: Layout, output: Output) -> dict[str, str]:
    env = {
        name: value
        for name, value in os.environ.items()
        if name not in REMOVED_ENV and not name.startswith(REMOVED_ENV_PREFIXES)
    }
    flags = [
        flag.format(cargo_home=cargo_home(), root=layout.root)
        for flag in RECIPE["rustflags"]
    ]
    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(flags)
    env["CARGO_TARGET_DIR"] = str(layout.cargo_target(output))
    env.update(PROFILE_OVERRIDES[output])
    return env


def compile_package(
    layout: Layout, closure: Closure, output: Output, work: Path, log: io.TextIOBase
) -> tuple[Path, dict[str, float]]:
    """Compile, bind and optionally optimise the crate into ``work``."""
    env = cargo_env(layout, output)
    timings = {}
    started = time.monotonic()
    stream(
        [
            "cargo",
            "build",
            "--lib",
            "--locked",
            f"--{RECIPE['profile']}",
            "--target",
            TARGET,
            f"--features={','.join(FEATURES)}",
        ],
        layout.crate,
        env,
        log,
    )
    timings["cargo"] = time.monotonic() - started
    module = (
        layout.cargo_target(output)
        / TARGET
        / RECIPE["profile"]
        / f"{closure.lib_name}.wasm"
    )
    bound = work / "bindgen"
    started = time.monotonic()
    stream(
        [BINDGEN, str(module), "--out-dir", str(bound), *RECIPE["bindgen"]],
        layout.crate,
        env,
        log,
    )
    timings[BINDGEN] = time.monotonic() - started
    if WASM_OPT[output]:
        started = time.monotonic()
        optimised = bound / "index_bg.opt.wasm"
        stream(
            [
                "wasm-opt",
                str(bound / "index_bg.wasm"),
                "-o",
                str(optimised),
                *WASM_OPT[output],
            ],
            layout.crate,
            env,
            log,
        )
        os.replace(optimised, bound / "index_bg.wasm")
        timings["wasm-opt"] = time.monotonic() - started
    for name in PACKAGE_FILES:
        if not (bound / name).is_file():
            raise WasmError(f"{BINDGEN} did not produce {name}")
    return bound, timings


# Development package ---------------------------------------------------------------

ENTRY = """\
// Generated by scripts/dev/wasm.py. Loads development build {build}.
import "./status.js"
export * from "./builds/{build}/index.js"
export {{default}} from "./builds/{build}/index.js"

// The glue module owns the instantiated WebAssembly memory. Importers that were
// hot-swapped would keep calling an uninitialised instance, so reload instead.
if (import.meta.webpackHot) import.meta.webpackHot.decline()
if (import.meta.hot) import.meta.hot.accept(() => window.location.reload())
"""
TYPES = """\
export * from "./builds/{build}/index.js"
export {{default}} from "./builds/{build}/index.js"
"""
STATUS_OK = "// The last sequent-core development build succeeded.\nexport {}\n"
STATUS_FAILED = "console.error({message})\nexport {{}}\n"


def load_status(layout: Layout) -> dict[str, object]:
    with contextlib.suppress(FileNotFoundError, ValueError):
        status = json.loads((layout.dist / "status.json").read_text())
        if isinstance(status, dict) and status.get("schema") == SCHEMA:
            return status
    return {}


def published_build(layout: Layout, status: Mapping[str, object]) -> str | None:
    """The build the entry module loads, if all of its files are still present."""
    published = status.get("published")
    if not isinstance(published, dict):
        return None
    build = str(published.get("build"))
    entry = layout.dist / "index.js"
    files = [layout.builds / build / name for name in PACKAGE_FILES]
    if not entry.is_file() or f"./builds/{build}/index.js" not in entry.read_text():
        return None
    return build if all(path.is_file() for path in files) else None


def write_status(layout: Layout, status: Mapping[str, object]) -> None:
    write_atomic(layout.dist / "status.json", json.dumps(status, indent=2).encode())
    published = status.get("published")
    if status["state"] == State.FAILED.value and isinstance(published, dict):
        attempt = status["last_attempt"]
        message = (
            f"sequent-core: the development WASM build failed at {attempt['at']}; "
            f"this page still runs build {published['build']} from "
            f"{published['built_at']}. Run scripts/dev/step-dev wasm for the error."
        )
        script = STATUS_FAILED.format(message=json.dumps(message))
    else:
        script = STATUS_OK
    path = layout.dist / "status.js"
    # An unchanged status module keeps successful rebuilds to one reload.
    if not path.is_file() or path.read_text() != script:
        write_atomic(path, script.encode())


def publish(
    layout: Layout,
    bound: Path,
    metadata: Mapping[str, object],
) -> tuple[str, bool]:
    """Publish a bound module; return its build id and whether the entry moved.

    Builds are immutable directories named after their content. A new one is
    written under a temporary name and renamed into place complete, then the entry
    module is replaced in one rename. The glue and its .wasm live in the same
    build directory, so a consumer can only ever load a matching pair.
    """
    content = hashlib.sha256()
    for name in PACKAGE_FILES:
        content.update(name.encode() + b"\0" + (bound / name).read_bytes())
    build = content.hexdigest()[:SHORT]
    final = layout.builds / build
    if not final.is_dir():
        layout.builds.mkdir(parents=True, exist_ok=True)
        staging = Path(tempfile.mkdtemp(prefix=".staging-", dir=layout.builds))
        try:
            for name in PACKAGE_FILES:
                shutil.copyfile(bound / name, staging / name)
            (staging / "build.json").write_text(json.dumps(metadata, indent=2))
            staging.chmod(0o755)
            os.rename(staging, final)
        except BaseException:
            shutil.rmtree(staging, ignore_errors=True)
            raise
    entry = layout.dist / "index.js"
    text = ENTRY.format(build=build)
    moved = not entry.is_file() or entry.read_text() != text
    package = {
        "name": CRATE,
        "version": metadata["version"],
        "private": True,
        "type": "module",
        "main": "index.js",
        "types": "index.d.ts",
    }
    write_atomic(layout.dist / "package.json", json.dumps(package, indent=2).encode())
    write_atomic(layout.dist / "index.d.ts", TYPES.format(build=build).encode())
    if moved:
        write_atomic(entry, text.encode())
    return build, moved


def prune(layout: Layout, keep: Iterable[str]) -> None:
    kept = set(keep)
    builds = sorted(
        (path for path in layout.builds.iterdir() if path.is_dir()),
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    for path in builds:
        if path.name.startswith(".staging-"):
            continue
        if path.name in kept or len(kept) < KEPT_BUILDS:
            kept.add(path.name)
            continue
        shutil.rmtree(path, ignore_errors=True)


@contextlib.contextmanager
def exclusive(layout: Layout) -> Iterator[None]:
    layout.state.mkdir(parents=True, exist_ok=True)
    with (layout.state / "lock").open("w") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            print("waiting for another sequent-core WASM build", file=sys.stderr)
            fcntl.flock(handle, fcntl.LOCK_EX)
        yield


@dataclass(frozen=True)
class Plan:
    """The fingerprints of one build, computed before compiling anything."""

    closure: Closure
    tools: dict[str, str]
    source_records: list[Record]
    source: str
    build: str


def plan(layout: Layout, output: Output) -> Plan:
    tools = probe_tools(layout, output)
    closure = load_closure(layout, tools["cargo"])
    check_tools(closure, tools)
    records = source_records(layout, closure)
    source = digest_records(records)
    build = digest_records([*records, *environment_records(tools, output)])
    return Plan(closure, tools, records, source, build)


def up_to_date(layout: Layout, fingerprint: str) -> str | None:
    status = load_status(layout)
    build = published_build(layout, status)
    published = status.get("published")
    if build is None or not isinstance(published, dict):
        return None
    if published.get("fingerprint") != fingerprint:
        return None
    if status.get("state") != State.OK.value:
        # The failure no longer applies: the served build matches the sources again.
        write_status(layout, {**status, "state": State.OK.value})
    return build


def record_failure(
    layout: Layout, attempt: Mapping[str, object], error: WasmError
) -> int:
    """Mark the attempt failed without touching the published build."""
    status = load_status(layout)
    write_status(
        layout,
        {
            **status,
            "schema": SCHEMA,
            "state": State.FAILED.value,
            "last_attempt": {**attempt, "error": str(error)},
        },
    )
    served = published_build(layout, status)
    kept = (
        f"the development package still serves build {served}"
        if served
        else "no development package is published"
    )
    print(
        f"sequent-core WASM build FAILED: {error}\n{kept}; log: {layout.log}",
        file=sys.stderr,
    )
    return 1


def build_development(layout: Layout) -> int:
    started = time.monotonic()
    try:
        current = plan(layout, Output.DEVELOPMENT)
    except WasmError as error:
        return record_failure(layout, {"fingerprint": None, "at": now()}, error)
    build = up_to_date(layout, current.build)
    if build is not None:
        print(
            f"sequent-core WASM up to date: fingerprint {current.build[:SHORT]}, "
            f"build {build} ({time.monotonic() - started:.2f} s)"
        )
        return 0
    with exclusive(layout):
        build = up_to_date(layout, current.build)
        if build is not None:
            print(f"sequent-core WASM up to date: build {build} was just published")
            return 0
        status = load_status(layout)
        attempt = {"fingerprint": current.build, "at": now(), "log": str(layout.log)}
        try:
            with (
                layout.log.open("w") as log,
                tempfile.TemporaryDirectory(dir=layout.state) as work,
            ):
                bound, timings = compile_package(
                    layout, current.closure, Output.DEVELOPMENT, Path(work), log
                )
                built_at = now()
                metadata = {
                    "schema": SCHEMA,
                    "version": current.closure.package["version"],
                    "fingerprint": current.build,
                    "source_fingerprint": current.source,
                    "built_at": built_at,
                    "output": Output.DEVELOPMENT.value,
                    "tools": current.tools,
                }
                build, moved = publish(layout, Path(bound), metadata)
        except WasmError as error:
            return record_failure(layout, attempt, error)
        previous = published_build(layout, status)
        write_status(
            layout,
            {
                "schema": SCHEMA,
                "state": State.OK.value,
                "published": {
                    "build": build,
                    "fingerprint": current.build,
                    "source_fingerprint": current.source,
                    "built_at": built_at,
                    "tools": current.tools,
                },
                "last_attempt": attempt,
            },
        )
        prune(layout, [build, *([previous] if previous else [])])
    steps = ", ".join(f"{name} {seconds:.1f} s" for name, seconds in timings.items())
    change = "published" if moved else "rebuilt identical output of"
    print(
        f"sequent-core WASM {change} build {build} "
        f"(fingerprint {current.build[:SHORT]}; {steps}; "
        f"total {time.monotonic() - started:.1f} s)\nentry: {layout.dist / 'index.js'}"
    )
    return 0


def report_status(layout: Layout) -> int:
    status = load_status(layout)
    build = published_build(layout, status)
    if build is None:
        print("no sequent-core development package; run scripts/dev/step-dev wasm")
        return 1
    published = status["published"]
    print(f"entry: {layout.dist / 'index.js'}")
    print(f"build {build} from {published['built_at']} ({status['state']})")
    attempt = status.get("last_attempt", {})
    if status["state"] == State.FAILED.value:
        print(f"last attempt failed at {attempt.get('at')}: {attempt.get('error')}")
    current = plan(layout, Output.DEVELOPMENT)
    fresh = published["fingerprint"] == current.build
    print(
        "matches the current sources"
        if fresh
        else f"stale: the sources now fingerprint {current.build[:SHORT]}"
    )
    return 0 if fresh and status["state"] == State.OK.value else 1


def clean(layout: Layout) -> int:
    with exclusive(layout):
        for path in layout.state.iterdir():
            if path.name == "lock":
                continue
            if path.is_dir():
                shutil.rmtree(path)
            else:
                path.unlink()
    print(f"removed {layout.state}; restart dev servers to load the installed package")
    return 0


# Committed package -----------------------------------------------------------------


def archive_name(closure: Closure) -> str:
    return f"{CRATE}-{closure.package['version']}.tgz"


def package_manifest(closure: Closure) -> bytes:
    """The package.json wasm-pack writes for ``--target web``, plus the inputs file."""
    package = closure.package
    manifest: dict[str, object] = {"name": package["name"], "type": "module"}
    if package.get("authors"):
        manifest["collaborators"] = package["authors"]
    if package.get("description"):
        manifest["description"] = package["description"]
    manifest["version"] = package["version"]
    if package.get("license"):
        manifest["license"] = package["license"]
    if package.get("repository"):
        manifest["repository"] = {"type": "git", "url": package["repository"]}
    manifest["files"] = ["index_bg.wasm", "index.js", "index.d.ts", INPUTS_FILE]
    manifest["main"] = "index.js"
    if package.get("homepage"):
        manifest["homepage"] = package["homepage"]
    manifest["types"] = "index.d.ts"
    manifest["sideEffects"] = ["./snippets/*"]
    if package.get("keywords"):
        manifest["keywords"] = package["keywords"]
    return json.dumps(manifest, indent=2).encode()


def pack(entries: Mapping[str, bytes]) -> bytes:
    """An npm-style tgz whose bytes depend only on the entries."""
    tar_buffer = io.BytesIO()
    with tarfile.open(fileobj=tar_buffer, mode="w", format=tarfile.USTAR_FORMAT) as tar:
        for name in sorted(entries):
            info = tarfile.TarInfo(f"package/{name}")
            info.size = len(entries[name])
            info.mtime = NPM_EPOCH
            info.mode = 0o644
            tar.addfile(info, io.BytesIO(entries[name]))
    return gzip.compress(tar_buffer.getvalue(), compresslevel=9, mtime=0)


def read_archive(path: Path) -> dict[str, bytes]:
    with tarfile.open(path, "r:gz") as tar:
        return {
            member.name.removeprefix("package/"): tar.extractfile(member).read()
            for member in tar.getmembers()
            if member.isfile()
        }


def lock_pattern(name: str) -> re.Pattern[str]:
    return re.compile(
        r'^(?P<prefix>  resolved "file:(?P<path>[^"#]*/'
        + re.escape(name)
        + r')#)(?P<hash>[0-9a-f]{40})(?P<suffix>")$',
        re.M,
    )


def locked_hashes(layout: Layout, name: str) -> dict[str, str]:
    text = layout.yarn_lock.read_text()
    return {match["path"]: match["hash"] for match in lock_pattern(name).finditer(text)}


def update_yarn_lock(layout: Layout, name: str, sha1: str) -> int:
    text = layout.yarn_lock.read_text()
    updated, count = lock_pattern(name).subn(rf"\g<prefix>{sha1}\g<suffix>", text)
    if count == 0:
        raise WasmError(f"packages/yarn.lock has no entry for {name}")
    write_atomic(layout.yarn_lock, updated.encode())
    return count


def release_package(layout: Layout) -> int:
    started = time.monotonic()
    current = plan(layout, Output.RELEASE)
    check_release_toolchain(layout, current.tools)
    name = archive_name(current.closure)
    crate = layout.root / next(
        package.root for package in current.closure.local if package.name == CRATE
    )
    inputs = {
        "schema": SCHEMA,
        "source_fingerprint": current.source,
        "recipe": RECIPE,
        "output": {
            "kind": Output.RELEASE.value,
            "wasm_opt": list(WASM_OPT[Output.RELEASE]),
        },
        "tools": current.tools,
        "inputs": [list(record) for record in current.source_records],
    }
    with exclusive(layout), tempfile.TemporaryDirectory(dir=layout.state) as work:
        with layout.log.open("w") as log:
            bound, timings = compile_package(
                layout, current.closure, Output.RELEASE, Path(work), log
            )
        entries = {file: (bound / file).read_bytes() for file in PACKAGE_FILES}
        entries["package.json"] = package_manifest(current.closure)
        entries[INPUTS_FILE] = json.dumps(inputs, indent=2).encode()
        for path in current.closure.crate_files(crate):
            if path.is_file():
                entries[path.name] = path.read_bytes()
        archive = pack(entries)
        for consumer in CONSUMERS:
            write_atomic(layout.archive(consumer, name), archive)
        sha1 = hashlib.sha1(archive).hexdigest()
        entries_updated = update_yarn_lock(layout, name, sha1)
    steps = ", ".join(f"{step} {seconds:.1f} s" for step, seconds in timings.items())
    print(
        f"wrote {name} ({sha1}) for {', '.join(CONSUMERS)} and {entries_updated} "
        f"yarn.lock entries; source fingerprint {current.source[:SHORT]} "
        f"({steps}; total {time.monotonic() - started:.1f} s)\n"
        "next: cd packages && yarn install --frozen-lockfile, then commit the "
        "tgz files and yarn.lock"
    )
    return 0


def describe_changes(
    recorded: Iterable[Sequence[str]], current: Iterable[Record]
) -> list[str]:
    before = {(kind, key): value for kind, key, value in recorded}
    after = {(kind, key): value for kind, key, value in current}
    changes = []
    for kind, key in sorted(before.keys() | after.keys()):
        if (kind, key) not in before:
            changes.append(f"added {kind} {key}".rstrip())
        elif (kind, key) not in after:
            changes.append(f"removed {kind} {key}".rstrip())
        elif before[kind, key] != after[kind, key]:
            changes.append(f"changed {kind} {key}".rstrip())
    return changes


def check_package(layout: Layout) -> int:
    tools = {"cargo": capture(["cargo", "-V"], layout.crate).strip()}
    closure = load_closure(layout, tools["cargo"])
    name = archive_name(closure)
    problems = []
    archives = {consumer: layout.archive(consumer, name) for consumer in CONSUMERS}
    missing = [
        layout.relative(path) for path in archives.values() if not path.is_file()
    ]
    if missing:
        problems.append(f"missing {', '.join(missing)}")
    present = {consumer: path for consumer, path in archives.items() if path.is_file()}
    hashes = {consumer: digest_file(path, "sha1") for consumer, path in present.items()}
    if len(set(hashes.values())) > 1:
        problems.append(
            "the copies differ: "
            + ", ".join(f"{consumer} {sha1[:12]}" for consumer, sha1 in hashes.items())
        )
    for path, sha1 in sorted(locked_hashes(layout, name).items()):
        archive = layout.packages / path
        actual = digest_file(archive, "sha1") if archive.is_file() else MISSING
        if actual != sha1:
            problems.append(
                f"yarn.lock records {path}#{sha1[:12]}, the file is {actual[:12]}"
            )
    if present:
        contents = read_archive(next(iter(present.values())))
        recorded = (
            json.loads(contents[INPUTS_FILE]) if INPUTS_FILE in contents else None
        )
        if recorded is None:
            problems.append(
                f"the package records no source fingerprint ({INPUTS_FILE})"
            )
        else:
            records = source_records(layout, closure)
            if recorded["source_fingerprint"] != digest_records(records):
                changes = describe_changes(recorded["inputs"], records)
                shown = "\n  ".join(changes[:20])
                more = f"\n  and {len(changes) - 20} more" if len(changes) > 20 else ""
                problems.append(
                    "it was built from other sources than the checkout:\n  "
                    + shown
                    + more
                )
            expected = list(WASM_OPT[Output.RELEASE])
            if recorded.get("output", {}).get("wasm_opt") != expected:
                problems.append(
                    "it was not optimised with the release wasm-opt settings"
                )
    if problems:
        print(
            f"the committed {name} does not match the checkout:\n- "
            + "\n- ".join(problems)
            + "\nregenerate it: nix develop ./packages/sequent-core --command "
            "scripts/dev/step-dev wasm --release-package; then run yarn install "
            "--frozen-lockfile in packages/ and commit the tgz files and yarn.lock",
            file=sys.stderr,
        )
        return 1
    fingerprint = recorded["source_fingerprint"][:SHORT]
    print(f"the committed {name} matches the sources ({fingerprint})")
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="step-dev wasm",
        description=(
            "Build sequent-core's WebAssembly package. Without options, rebuild the "
            "development package when its inputs changed and publish it for the "
            "portal dev servers and Storybook."
        ),
    )
    actions = parser.add_mutually_exclusive_group()
    for action, text in (
        (Action.STATUS, "report whether the development package matches the sources"),
        (Action.CLEAN, "remove the development package and its build cache"),
        (
            Action.RELEASE_PACKAGE,
            "regenerate the committed tgz files and their yarn.lock hashes",
        ),
        (
            Action.CHECK_PACKAGE,
            "fail when the committed tgz does not match the sources",
        ),
    ):
        actions.add_argument(
            f"--{action.value}",
            dest="action",
            action="store_const",
            const=action,
            help=text,
        )
    parser.set_defaults(action=Action.BUILD)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None, layout: Layout | None = None) -> int:
    arguments = parse_args(sys.argv[1:] if argv is None else argv)
    layout = layout or Layout(ROOT)
    handlers = {
        Action.BUILD: build_development,
        Action.STATUS: report_status,
        Action.CLEAN: clean,
        Action.RELEASE_PACKAGE: release_package,
        Action.CHECK_PACKAGE: check_package,
    }
    try:
        return handlers[arguments.action](layout)
    except WasmError as error:
        print(f"step-dev wasm: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
