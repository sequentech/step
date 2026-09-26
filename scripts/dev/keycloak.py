# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Opt-in React login/OTP previews and live pages on an existing Keycloak."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import zipfile
from enum import Enum
from pathlib import Path
from urllib.parse import urlsplit

from .mode.checkout import Checkout, CheckoutError, load_checkout, parse_dotenv

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = Path("packages/keycloak-ui")
THEMES = Path(".cache/keycloak-ui/themes")
SOURCE = Path("packages/keycloak-extensions/sequent-theme/src/main/resources/theme")
OVERLAY = "docker-compose-keycloak-ui.yml"
REACT_PAGES = ("login.ftl", "message-otp.login.ftl")
HOT_CLIENT = '<script type="module" src="/@vite/client"></script>'
HOT_SCRIPTS = (
    """<script type="module">
import RefreshRuntime from "/@react-refresh";
RefreshRuntime.injectIntoGlobalHook(window);
window.$RefreshReg$ = () => {};
window.$RefreshSig$ = () => type => type;
window.__vite_plugin_react_preamble_installed__ = true;
</script>
"""
    + HOT_CLIENT
    + '\n<script type="module" src="/src/main.tsx"></script>'
)
FALLBACK = """<#if (matchAttributes![])?has_content || recaptchaEnabled??
    || ['structured', 'pattern']?seq_contains(
        realm.attributes['credential-input-policy']!'standard')
    || (social.providers![])?has_content || usernameHidden??
    || (auth?has_content && auth.showTryAnotherWayLink())>
<#include "sequent-login.ftl">
<#else>
"""


class Runtime(Enum):
    HOT = "hot"
    BUILT = "built"


def run(
    command: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None
) -> int:
    return subprocess.run(command, cwd=cwd, env=env, check=True).returncode


def write(path: Path, content: str | bytes) -> None:
    """Replace files within stable mounted directories."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_bytes(content.encode() if isinstance(content, str) else content)
    temporary.replace(path)


def page_template(source: str, bridge: str, runtime: Runtime, *, login: bool) -> str:
    if source.count("</head>") != 1:
        raise ValueError("Generated Keycloak template has no unique head element")
    if runtime is Runtime.HOT:
        source, count = re.subn(
            r'<script\b(?=[^>]*\btype="module")(?=[^>]*\bsrc=)[^>]*></script>',
            lambda _: HOT_SCRIPTS,
            source,
        )
        if count != 1:
            raise ValueError("Generated Keycloak template has no unique module entry")
        source = re.sub(r"<base\b[^>]*>", "", source)
        source = re.sub(
            r'<link\b[^>]*rel="(?:stylesheet|modulepreload)"[^>]*>', "", source
        )
    source = source.replace("</head>", bridge + "\n</head>")
    return FALLBACK + source + "\n</#if>\n" if login else source


def theme_source(root: Path, theme: str, name: str) -> Path:
    visited: set[str] = set()
    while theme not in visited:
        visited.add(theme)
        directory = root / SOURCE / theme / "login"
        candidate = directory / name
        if candidate.is_file():
            return candidate
        properties = directory / "theme.properties"
        if not properties.is_file():
            break
        theme = parse_dotenv(properties.read_text()).get("parent", "")
    raise ValueError(f"No inherited {name} in the Sequent theme sources")


def prepare(root: Path, runtime: Runtime, skip_build: bool) -> None:
    package = root / PACKAGE
    if not skip_build:
        run(["yarn", "build-keycloak-theme"], cwd=package)
    archive = package / "dist_keycloak/sequent-ui.jar"
    if not archive.is_file():
        raise ValueError("Run step-dev keycloak prepare before --skip-build")
    parents: dict[str, str] = json.loads((package / "themes.json").read_text())
    bridge = (package / "context.ftl").read_text()
    with zipfile.ZipFile(archive) as jar:
        for name, parent in parents.items():
            target = root / THEMES / name / "login"
            prefix = f"theme/{name}/login/"
            for entry in jar.namelist():
                if not entry.startswith(prefix + "resources/") or entry.endswith("/"):
                    continue
                relative = Path(entry.removeprefix(prefix))
                if ".." in relative.parts:
                    raise ValueError("Theme archive contains an unsafe resource path")
                write(target / relative, jar.read(entry))
            for page in REACT_PAGES:
                text = jar.read(prefix + page).decode()
                write(
                    target / page,
                    page_template(text, bridge, runtime, login=page == "login.ftl"),
                )
            write(
                target / "sequent-login.ftl",
                theme_source(root, parent, "login.ftl").read_text(),
            )
            template = theme_source(root, parent, "template.ftl").read_text()
            if runtime is Runtime.HOT:
                template = template.replace("</head>", HOT_CLIENT + "\n</head>")
            write(target / "template.ftl", template)
            write(
                target / "theme.properties",
                f"parent={parent}\nimport=common/keycloak\n",
            )
            # Unported pages inherit Sequent's profile widgets and flows.
            for page in target.glob("*.ftl"):
                if page.name not in (*REACT_PAGES, "template.ftl", "sequent-login.ftl"):
                    page.unlink()
    print(f"Prepared {', '.join(parents)} ({runtime.value}) in {root / THEMES}")


def upstream(value: str) -> str:
    parsed = urlsplit(value)
    if (
        parsed.scheme not in ("http", "https")
        or not parsed.hostname
        or parsed.username
        or parsed.password
        or parsed.path not in ("", "/")
        or parsed.query
        or parsed.fragment
    ):
        raise argparse.ArgumentTypeError(
            "Use a Keycloak HTTP(S) origin without credentials"
        )
    return value.rstrip("/")


def mount_command(checkout: Checkout, docker_host: str) -> list[str]:
    if not docker_host or docker_host in (
        "unix:///var/run/docker.sock",
        "unix:///run/docker.sock",
    ):
        raise ValueError(
            "Set --docker-host or DOCKER_HOST to the isolated development daemon"
        )
    if (
        not checkout.name_prefix
        or checkout.host_root.resolve() != checkout.root.resolve()
    ):
        raise ValueError(
            "Initialize this checkout's .devcontainer/.env before mounting themes"
        )
    if not (checkout.root / THEMES / "sequent-ui-admin/login/login.ftl").is_file():
        raise ValueError("Run step-dev keycloak prepare before mounting themes")
    command = [
        "docker",
        "--host",
        docker_host,
        "compose",
        "--project-name",
        checkout.project,
        "--env-file",
        str(checkout.root / ".devcontainer/.env"),
    ]
    for name in ("docker-compose.yml", "docker-compose-ui-keycloak.yml", OVERLAY):
        command += ["-f", str(checkout.compose_dir / name)]
    return [*command, "up", "-d", "--no-build", "keycloak"]


def main(arguments: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    prepared = commands.add_parser(
        "prepare", help="Build and prepare opt-in folder themes"
    )
    prepared.add_argument(
        "--runtime", choices=[item.value for item in Runtime], default="hot"
    )
    prepared.add_argument(
        "--skip-build", action="store_true", help="Refresh FTL from the existing jar"
    )
    mounted = commands.add_parser(
        "mount", help="Mount themes into this checkout's isolated Keycloak"
    )
    mounted.add_argument("--docker-host", default=os.environ.get("DOCKER_HOST", ""))
    dev = commands.add_parser(
        "dev", help="Synthetic previews and automatic reload on real Keycloak"
    )
    dev.add_argument(
        "--keycloak-url", type=upstream, help="Proxy an existing development Keycloak"
    )
    dev.add_argument("--port", type=int, default=5174)
    dev.add_argument("--host", default="127.0.0.1")
    story = commands.add_parser(
        "storybook", help="Shared synthetic login and OTP stories"
    )
    story.add_argument("--port", type=int, default=6011)
    story.add_argument("--host", default="127.0.0.1")
    args = parser.parse_args(arguments)
    try:
        if args.command == "prepare":
            prepare(ROOT, Runtime(args.runtime), args.skip_build)
        elif args.command == "mount":
            checkout = load_checkout(ROOT)
            run(mount_command(checkout, args.docker_host))
        elif args.command == "dev":
            environment = dict(os.environ)
            if args.keycloak_url:
                environment["KEYCLOAK_UI_UPSTREAM"] = args.keycloak_url
            run(
                ["yarn", "dev", "--port", str(args.port), "--host", args.host],
                cwd=ROOT / PACKAGE,
                env=environment,
            )
        else:
            run(
                ["yarn", "storybook", "--port", str(args.port), "--host", args.host],
                cwd=ROOT / PACKAGE,
            )
        return 0
    except (OSError, ValueError, CheckoutError, subprocess.CalledProcessError) as error:
        print(f"step-dev keycloak: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
