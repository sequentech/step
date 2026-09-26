# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Command line: ``python3 -m scripts.dev.bench <scenario> [options]``."""

from __future__ import annotations

import argparse
import os
import signal
import sys
from collections.abc import Sequence
from dataclasses import replace
from pathlib import Path

from . import ci, focused_test, rust, summarize, ui_update, wasm, workspace
from .common import default_output_dir
from .edits import EditError, EditSpec, load_edits
from .isolation import Dind, IsolationError, parse_seed, remove_as_root
from .probes import ProbeError, resolve_probes
from .results import CacheState, validate_label

DEFAULT_DIND_IMAGE = "docker:29-dind"
DEFAULT_DIND_PREFIX = "step-bench-dind-"
DEFAULT_REPOSITORY = "sequentech/step"


def caller_path(value: str) -> Path:
    """Relative paths resolve against the directory ``step-dev`` was called from."""
    path = Path(value).expanduser()
    if not path.is_absolute():
        path = Path(os.environ.get("STEP_DEV_CWD", os.getcwd())) / path
    return path.resolve()


def checkout_path(value: str) -> Path:
    path = caller_path(value)
    if not (path / "packages").is_dir() or not (path / ".devcontainer").is_dir():
        raise argparse.ArgumentTypeError(f"not a step checkout: {path}")
    return path


def positive_int(value: str) -> int:
    number = int(value)
    if number < 1:
        raise argparse.ArgumentTypeError("must be at least 1")
    return number


def label(value: str) -> str:
    try:
        return validate_label(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError(str(error)) from None


def common_options(parser: argparse.ArgumentParser, checkout: bool = True) -> None:
    parser.add_argument(
        "--label",
        type=label,
        required=True,
        help="series label, e.g. before or after",
    )
    parser.add_argument(
        "--output-dir",
        type=caller_path,
        default=default_output_dir(),
        help="results root; files go to <dir>/<scenario>/ (default: "
        "$STEP_BENCH_OUTPUT_DIR or ~/.cache/step-bench/results)",
    )
    if checkout:
        parser.add_argument(
            "--checkout",
            type=checkout_path,
            required=True,
            help="the step checkout to measure (never modified beyond reverted edits)",
        )


def add_workspace(
    subparsers: argparse._SubParsersAction[argparse.ArgumentParser],
) -> None:
    parser = subparsers.add_parser(
        "workspace",
        help="command-to-ready time of the devcontainer in an isolated Docker daemon",
        description=workspace.__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    common_options(parser)
    parser.add_argument(
        "--cache", choices=[CacheState.COLD.value, CacheState.WARM.value], required=True
    )
    parser.add_argument(
        "--target",
        default="full-stack",
        help="readiness preset: full-stack or devcontainer",
    )
    parser.add_argument(
        "--probe",
        action="append",
        default=[],
        metavar="NAME=KIND:ARGS",
        help="add or replace a readiness probe (see probes kinds)",
    )
    parser.add_argument(
        "--after-up",
        metavar="COMMAND",
        help="developer command started in the dev container after "
        "devcontainer up (for example a Storybook server)",
    )
    parser.add_argument(
        "--config", help="devcontainer.json path relative to the checkout"
    )
    parser.add_argument(
        "--docker-host",
        metavar="unix:///PATH",
        help="warm: the isolated daemon holding the stack",
    )
    parser.add_argument(
        "--dind-name", help="warm: container of that daemon, for its memory and disk"
    )
    parser.add_argument(
        "--sandbox-root",
        type=caller_path,
        help="cold: directory for fresh daemons (dind/) and copies (envs/)",
    )
    parser.add_argument("--dind-prefix", default=DEFAULT_DIND_PREFIX)
    parser.add_argument("--dind-image", default=DEFAULT_DIND_IMAGE)
    parser.add_argument(
        "--copy-name",
        default="step",
        help="directory name of the checkout copy; the compose files "
        "expect /workspaces/step",
    )
    parser.add_argument(
        "--empty-mount",
        action="append",
        type=Path,
        metavar="PATH",
        help="path the devcontainer bind-mounts from the host, provided "
        "empty inside the daemon (default: ~/.config/claude)",
    )
    parser.add_argument(
        "--seed-image",
        action="append",
        default=[],
        metavar="SRC=DST",
        help="cold: copy image SRC from the default daemon (read-only) "
        "into the fresh daemon as DST before timing, for images that "
        "are no longer publicly pullable",
    )
    parser.add_argument(
        "--keep",
        action="store_true",
        help="cold: keep the daemon and copy for warm samples",
    )
    parser.add_argument(
        "--samples", type=positive_int, default=None, help="default: 1 cold, 10 warm"
    )
    parser.add_argument(
        "--warmup", type=int, default=1, help="warm: untimed restarts before samples"
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=None,
        help="seconds until ready (default: 3h cold, 30min warm)",
    )
    parser.add_argument("--poll-interval", type=float, default=1.0)
    parser.add_argument("--stats-interval", type=float, default=5.0)
    parser.add_argument(
        "--devcontainer", default="devcontainer", help="devcontainer CLI executable"
    )
    parser.add_argument(
        "--no-footprint",
        action="store_true",
        help="skip the disk footprint after readiness",
    )


def run_workspace(arguments: argparse.Namespace) -> Path:
    cache = CacheState(arguments.cache)
    cold = cache is CacheState.COLD
    if cold and arguments.docker_host:
        raise IsolationError("cold samples create their own daemon; drop --docker-host")
    empty = arguments.empty_mount or [Path.home() / ".config" / "claude"]
    options = workspace.WorkspaceOptions(
        checkout=arguments.checkout,
        label=arguments.label,
        cache=cache,
        target=arguments.target,
        probes=resolve_probes(arguments.target, arguments.probe),
        config=arguments.config,
        docker_host=arguments.docker_host,
        dind_name=arguments.dind_name,
        sandbox_root=arguments.sandbox_root,
        dind_prefix=arguments.dind_prefix,
        dind_image=arguments.dind_image,
        copy_name=arguments.copy_name,
        empty_mounts=empty,
        seeds=[parse_seed(seed) for seed in arguments.seed_image],
        keep=arguments.keep,
        after_up=arguments.after_up,
        samples=arguments.samples or (1 if cold else 10),
        warmup=arguments.warmup,
        timeout=arguments.timeout or (3 * 3600.0 if cold else 1800.0),
        poll_interval=arguments.poll_interval,
        stats_interval=arguments.stats_interval,
        devcontainer=arguments.devcontainer,
        output_dir=arguments.output_dir,
        footprint=not arguments.no_footprint,
    )
    return workspace.run_workspace(options)


def add_ci(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser(
        "ci",
        help="push-to-first-actionable and push-to-done times of pull request pushes",
        description=ci.__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    common_options(parser, checkout=False)
    parser.add_argument("--repository", default=DEFAULT_REPOSITORY)
    parser.add_argument(
        "--pr",
        type=positive_int,
        action="append",
        required=True,
        help="pull request number (repeat)",
    )
    parser.add_argument(
        "--pushes", type=positive_int, default=10, help="latest pushes per pull request"
    )
    parser.add_argument(
        "--exclude",
        action="append",
        metavar="REGEX",
        help="workflow names that are not actionable (replaces defaults)",
    )


def run_ci(arguments: argparse.Namespace) -> Path:
    return ci.run_ci(
        repository=arguments.repository,
        numbers=arguments.pr,
        label=arguments.label,
        pushes_per_pr=arguments.pushes,
        excluded=arguments.exclude or list(ci.DEFAULT_EXCLUDED_WORKFLOWS),
        output_dir=arguments.output_dir,
    )


def edit_choice(
    value: str, edits_file: Path | None, builtin: dict[str, EditSpec]
) -> EditSpec:
    edits = dict(builtin)
    if edits_file is not None:
        edits.update(load_edits(edits_file))
    if value not in edits:
        raise EditError(f"unknown edit {value!r} (choose {', '.join(sorted(edits))})")
    return edits[value]


def key_values(values: Sequence[str], option: str) -> dict[str, str]:
    pairs = {}
    for value in values:
        key, separator, text = value.partition("=")
        if not separator:
            raise ValueError(f"{option} expects NAME=VALUE: {value!r}")
        pairs[key] = text
    return pairs


def add_ui_update(
    subparsers: argparse._SubParsersAction[argparse.ArgumentParser],
) -> None:
    parser = subparsers.add_parser(
        "ui-update",
        help="save-to-browser-visible time of a UI edit in running dev servers",
        description=ui_update.__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    common_options(parser)
    parser.add_argument(
        "--edit",
        required=True,
        help=f"named edit: {', '.join(ui_update.EDITS)} or one from --edits",
    )
    parser.add_argument(
        "--edits",
        type=caller_path,
        help="JSON file of named edits {name: {path, anchor, template}}",
    )
    parser.add_argument(
        "--target",
        action="append",
        required=True,
        choices=sorted(ui_update.TARGETS),
        help="dev server and page to watch (repeat to run them together)",
    )
    parser.add_argument(
        "--server-cmd",
        action="append",
        default=[],
        metavar="TARGET=CMD",
        help="override a target's dev server command ({port} expands)",
    )
    parser.add_argument(
        "--rebuild-cmd",
        help="command run from the checkout after each save, e.g. "
        "'yarn --cwd packages build:ui-essentials'",
    )
    parser.add_argument("--samples", type=positive_int, default=10)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument(
        "--port-base",
        type=int,
        default=41000,
        help="first port; targets use consecutive ports",
    )
    parser.add_argument("--startup-timeout", type=float, default=900.0)
    parser.add_argument("--sample-timeout", type=float, default=600.0)
    parser.add_argument(
        "--settle",
        type=float,
        default=3.0,
        help="seconds to wait after each sample before the next save",
    )


def run_ui_update(arguments: argparse.Namespace) -> list[Path]:
    servers = key_values(arguments.server_cmd, "--server-cmd")
    unknown = set(servers) - set(arguments.target)
    if unknown:
        raise ValueError(f"--server-cmd for targets not selected: {sorted(unknown)}")
    options = ui_update.UiUpdateOptions(
        checkout=arguments.checkout,
        label=arguments.label,
        edit_name=arguments.edit,
        edit=edit_choice(arguments.edit, arguments.edits, ui_update.EDITS),
        targets=list(dict.fromkeys(arguments.target)),
        servers=servers,
        rebuild=arguments.rebuild_cmd,
        samples=arguments.samples,
        warmup=arguments.warmup,
        port_base=arguments.port_base,
        startup_timeout=arguments.startup_timeout,
        sample_timeout=arguments.sample_timeout,
        settle=arguments.settle,
        output_dir=arguments.output_dir,
    )
    return ui_update.run_ui_update(options)


def default_target_dir(checkout: Path) -> Path:
    # The devcontainer shell's CARGO_TARGET_DIR=rust-local-target, from packages/.
    return checkout / "packages" / "rust-local-target"


def add_test(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser(
        "test",
        help="save-to-result time of a focused test command",
        description=focused_test.__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    common_options(parser)
    parser.add_argument("--suite", required=True, choices=sorted(focused_test.SUITES))
    parser.add_argument(
        "--edit",
        help="replace the suite's edit with a named Rust edit "
        f"({', '.join(rust.RUST_EDITS)}) or one from --edits",
    )
    parser.add_argument("--edits", type=caller_path, help="JSON file of named edits")
    parser.add_argument("--command", help="replace the suite's command")
    parser.add_argument(
        "--cargo-target-dir",
        type=caller_path,
        help="default: <checkout>/packages/rust-local-target",
    )
    parser.add_argument("--samples", type=positive_int, default=10)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--timeout", type=float, default=3600.0)


def run_test(arguments: argparse.Namespace) -> Path:
    suite = focused_test.SUITES[arguments.suite]
    if arguments.edit:
        suite = replace(
            suite, edit=edit_choice(arguments.edit, arguments.edits, rust.RUST_EDITS)
        )
    if arguments.command:
        suite = replace(suite, command=arguments.command)
    options = focused_test.TestOptions(
        checkout=arguments.checkout,
        label=arguments.label,
        suite_name=arguments.suite,
        suite=suite,
        target_dir=arguments.cargo_target_dir or default_target_dir(arguments.checkout),
        samples=arguments.samples,
        warmup=arguments.warmup,
        timeout=arguments.timeout,
        output_dir=arguments.output_dir,
    )
    return focused_test.run_test(options)


def add_rust(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser(
        "rust",
        help="incremental Cargo rebuild time after one edit, with unit timings",
        description=rust.__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    common_options(parser)
    parser.add_argument(
        "--edit",
        required=True,
        help=f"named edit: {', '.join(rust.RUST_EDITS)} or one from --edits",
    )
    parser.add_argument("--edits", type=caller_path, help="JSON file of named edits")
    parser.add_argument(
        "--build",
        action="append",
        required=True,
        help=f"build to time after each save: {', '.join(rust.BUILDS)} "
        "or NAME=COMMAND (repeat)",
    )
    parser.add_argument(
        "--cargo-target-dir",
        type=caller_path,
        help="default: <checkout>/packages/rust-local-target",
    )
    parser.add_argument("--samples", type=positive_int, default=10)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--timeout", type=float, default=7200.0)


def build_choices(values: Sequence[str]) -> dict[str, str]:
    builds: dict[str, str] = {}
    for value in values:
        name, separator, command = value.partition("=")
        if separator:
            builds[name] = command
        elif value in rust.BUILDS:
            builds[value] = rust.BUILDS[value]
        else:
            raise ValueError(
                f"unknown build {value!r} (choose {', '.join(rust.BUILDS)})"
            )
    return builds


def run_rust(arguments: argparse.Namespace) -> list[Path]:
    options = rust.RustOptions(
        checkout=arguments.checkout,
        label=arguments.label,
        edit_name=arguments.edit,
        edit=edit_choice(arguments.edit, arguments.edits, rust.RUST_EDITS),
        builds=build_choices(arguments.build),
        target_dir=arguments.cargo_target_dir or default_target_dir(arguments.checkout),
        samples=arguments.samples,
        warmup=arguments.warmup,
        timeout=arguments.timeout,
        output_dir=arguments.output_dir,
    )
    return rust.run_rust(options)


def add_wasm(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser(
        "wasm",
        help="Rust/WASM save-to-browser time through the packaging workflow",
        description=wasm.__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    common_options(parser)
    change = parser.add_mutually_exclusive_group(required=True)
    change.add_argument(
        "--edit", help=f"named edit: {', '.join(wasm.WASM_EDITS)} or one from --edits"
    )
    change.add_argument(
        "--no-change", action="store_true", help="run the same sequence without an edit"
    )
    parser.add_argument("--edits", type=caller_path, help="JSON file of named edits")
    parser.add_argument(
        "--build-cmd",
        help="build command run from the checkout (default: the patched "
        "build-sequent-core.sh)",
    )
    parser.add_argument(
        "--server-restart",
        choices=[policy.value for policy in wasm.ServerRestart],
        default=wasm.ServerRestart.ALWAYS.value,
        help="restart the dev server after the build (always), or keep it running "
        "and wait for it to reload the page (never)",
    )
    parser.add_argument(
        "--install-cmd",
        default="yarn install",
        help="dependency install run from packages/ after the build",
    )
    parser.add_argument(
        "--target",
        default="voting",
        choices=sorted(set(ui_update.TARGETS) - {"storybook"}),
    )
    parser.add_argument("--port", type=int, default=41100)
    parser.add_argument("--samples", type=positive_int, default=10)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--timeout", type=float, default=1800.0)


def run_wasm(arguments: argparse.Namespace) -> Path:
    edit = None
    if arguments.edit:
        edit = edit_choice(arguments.edit, arguments.edits, wasm.WASM_EDITS)
    options = wasm.WasmOptions(
        checkout=arguments.checkout,
        label=arguments.label,
        edit_name=arguments.edit or wasm.NO_CHANGE,
        edit=edit,
        build=arguments.build_cmd,
        install=arguments.install_cmd,
        restart=wasm.ServerRestart(arguments.server_restart),
        target=arguments.target,
        port=arguments.port,
        samples=arguments.samples,
        warmup=arguments.warmup,
        timeout=arguments.timeout,
        output_dir=arguments.output_dir,
    )
    return wasm.run_wasm(options)


def add_summarize(
    subparsers: argparse._SubParsersAction[argparse.ArgumentParser],
) -> None:
    parser = subparsers.add_parser("summarize", help="Markdown tables of result files")
    parser.add_argument(
        "paths", nargs="+", type=caller_path, help="result files or directories"
    )
    parser.add_argument("--phases", action="store_true", help="add phase rows")


def add_clean(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser(
        "clean", help="remove a daemon and checkout copy kept by workspace --keep"
    )
    parser.add_argument("--sandbox-root", type=caller_path, required=True)
    parser.add_argument("--name", required=True, help="the kept daemon container name")
    parser.add_argument("--dind-image", default=DEFAULT_DIND_IMAGE)


def run_clean(arguments: argparse.Namespace) -> None:
    dind = Dind(
        arguments.name,
        arguments.sandbox_root / "dind" / arguments.name,
        arguments.dind_image,
    )
    owner = dind.owner()
    if owner is None:
        raise IsolationError(
            f"{arguments.name} is not a daemon created by this harness"
        )
    dind.remove(owner)
    environment_dir = arguments.sandbox_root / "envs" / arguments.name
    if environment_dir.exists():
        remove_as_root(arguments.dind_image, sorted(environment_dir.iterdir()))
        environment_dir.rmdir()


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(
        prog="step-dev bench",
        description="Reproducible feedback-loop benchmarks. Each scenario writes one "
        "JSON result per series with commit, tools, load, services, cache state, "
        "commands and raw samples.",
    )
    subparsers = root.add_subparsers(dest="scenario", required=True)
    add_workspace(subparsers)
    add_ui_update(subparsers)
    add_test(subparsers)
    add_wasm(subparsers)
    add_rust(subparsers)
    add_ci(subparsers)
    add_summarize(subparsers)
    add_clean(subparsers)
    return root


def terminate(signum: int, frame: object) -> None:
    # SystemExit unwinds through the finally blocks that restore edited files
    # and stop owned servers.
    raise SystemExit(128 + signum)


def main(argv: Sequence[str] | None = None) -> int:
    signal.signal(signal.SIGTERM, terminate)
    arguments = parser().parse_args(argv)
    try:
        if arguments.scenario == "summarize":
            print(summarize.summarize(arguments.paths, arguments.phases))
            return 0
        if arguments.scenario == "clean":
            run_clean(arguments)
            return 0
        runners = {
            "workspace": run_workspace,
            "ui-update": run_ui_update,
            "test": run_test,
            "wasm": run_wasm,
            "rust": run_rust,
            "ci": run_ci,
        }
        written = runners[arguments.scenario](arguments)
    except (IsolationError, ProbeError, ValueError) as error:
        print(f"bench: {error}", file=sys.stderr)
        return 2
    for path in written if isinstance(written, list) else [written]:
        print(path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
