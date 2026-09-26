# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import tempfile
import unittest
from pathlib import Path

from .isolation import (
    IsolationError,
    parse_memory,
    parse_seed,
    read_env_file,
    require_isolated,
    socket_path,
)
from .probes import (
    ProbeError,
    ProbeKind,
    all_healthy,
    expand,
    parse_probe,
    resolve_probes,
    strip_ansi,
)
from .workspace import cli_outcome, probe_env


def container(test=None, running=True, status=None):
    return {
        "Config": {"Healthcheck": {"Test": test}} if test is not None else {},
        "State": {
            "Running": running,
            **({"Health": {"Status": status}} if status else {}),
        },
    }


class ProbeParsingTest(unittest.TestCase):
    def test_parses_each_kind(self):
        cases = {
            "a=http:http://x:8080/healthz": (
                ProbeKind.HTTP,
                None,
                "http://x:8080/healthz",
            ),
            "b=exec:harvest:http://127.0.0.1:3030/ready": (
                ProbeKind.EXEC,
                "harvest",
                "http://127.0.0.1:3030/ready",
            ),
            "c=healthy:*": (ProbeKind.HEALTHY, "*", None),
            "d=running:keycloak": (ProbeKind.RUNNING, "keycloak", None),
            "e=log:windmill:Running `[^`]*main": (
                ProbeKind.LOG,
                "windmill",
                "Running `[^`]*main",
            ),
        }
        for text, (kind, name, argument) in cases.items():
            with self.subTest(text=text):
                probe = parse_probe(text)
                self.assertEqual(
                    (probe.kind, probe.container, probe.argument),
                    (kind, name, argument),
                )
                self.assertTrue(probe.required)
                self.assertEqual(str(probe), text)

    def test_question_mark_makes_a_probe_optional(self):
        probe = parse_probe("beat?=exec:beat:http://127.0.0.1:3030/ready")
        self.assertEqual(probe.name, "beat")
        self.assertFalse(probe.required)
        self.assertEqual(str(probe), "beat?=exec:beat:http://127.0.0.1:3030/ready")

    def test_rejects_malformed_probes(self):
        cases = {
            "no-equals": "NAME=KIND",
            "Bad=healthy:*": "slug",
            "x=ping:host": "unknown probe kind",
            "x=healthy:": "needs arguments",
            "x=exec:harvest": "CONTAINER:EXEC",
            "x=log:windmill:(": "invalid regular expression",
        }
        for text, message in cases.items():
            with self.subTest(text=text), self.assertRaisesRegex(ProbeError, message):
                parse_probe(text)

    def test_extra_probes_add_to_or_replace_the_preset(self):
        probes = resolve_probes(
            "full-stack",
            [
                "harvest=running:harvest",
                "storybook=exec:devcontainer:http://127.0.0.1:6006/",
            ],
        )
        by_name = {probe.name: probe for probe in probes}
        self.assertEqual(by_name["harvest"].kind, ProbeKind.RUNNING)
        self.assertIn("storybook", by_name)
        self.assertFalse(by_name["beat"].required)
        with self.assertRaisesRegex(ProbeError, "unknown target"):
            resolve_probes("ui-only", [])

    def test_expands_env_and_daemon_placeholders(self):
        env = {"SUPER_ADMIN_TENANT_ID": "t1"}
        self.assertEqual(
            expand(
                "http://{daemon_ip}:8090/realms/tenant-{env:SUPER_ADMIN_TENANT_ID}",
                env,
                "10.0.0.2",
            ),
            "http://10.0.0.2:8090/realms/tenant-t1",
        )
        with self.assertRaisesRegex(ProbeError, "MISSING"):
            expand("{env:MISSING}", env, None)
        with self.assertRaisesRegex(ProbeError, "daemon"):
            expand("{daemon_ip}", env, None)

    def test_strips_terminal_colors_from_logs(self):
        self.assertEqual(
            strip_ansi("\x1b[1m\x1b[32m    Running\x1b[0m `main`"), "    Running `main`"
        )


class HealthTest(unittest.TestCase):
    def test_containers_without_a_health_check_are_ignored(self):
        self.assertTrue(
            all_healthy(
                [
                    container(["CMD", "true"], status="healthy"),
                    container(),
                    container(["NONE"]),
                ]
            )
        )

    def test_needs_at_least_one_checked_container(self):
        self.assertFalse(all_healthy([container()]))

    def test_unhealthy_starting_or_stopped_containers_are_not_ready(self):
        for other in (
            container(["CMD", "true"], status="unhealthy"),
            container(["CMD", "true"], status="starting"),
            container(["CMD", "true"], running=False, status="healthy"),
            container(["CMD", "true"]),
        ):
            with self.subTest(other=other):
                self.assertFalse(
                    all_healthy([container(["CMD", "true"], status="healthy"), other])
                )


class IsolationTest(unittest.TestCase):
    def test_only_absolute_unix_sockets(self):
        self.assertEqual(
            socket_path("unix:///tmp/d/docker.sock"), Path("/tmp/d/docker.sock")
        )
        for host in (
            "tcp://127.0.0.1:2375",
            "unix://relative.sock",
            "/var/run/docker.sock",
        ):
            with self.subTest(host=host), self.assertRaises(IsolationError):
                socket_path(host)

    def test_refuses_the_default_socket_before_contacting_it(self):
        for host in ("unix:///var/run/docker.sock", "unix:///run/docker.sock"):
            with (
                self.subTest(host=host),
                self.assertRaisesRegex(IsolationError, "default"),
            ):
                require_isolated(host)

    def test_parses_docker_stats_sizes(self):
        cases = {"1.5GiB": 1610612736, "512kB": 512000, "0B": 0, "12.3MiB ": 12897484}
        for text, expected in cases.items():
            with self.subTest(text=text):
                self.assertEqual(parse_memory(text), expected)
        self.assertIsNone(parse_memory("12 parsecs"))
        self.assertIsNone(parse_memory("--"))

    def test_seed_needs_both_references(self):
        self.assertEqual(
            parse_seed("minio/minio:latest=quay.io/minio/minio:latest"),
            ("minio/minio:latest", "quay.io/minio/minio:latest"),
        )
        for text in ("minio/minio", "=b", "a="):
            with self.subTest(text=text), self.assertRaises(IsolationError):
                parse_seed(text)

    def test_reads_dotenv_values_and_later_files_override(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".devcontainer").mkdir()
            (root / ".devcontainer" / ".env.development").write_text(
                "# comment\nA=1\nexport B=\"two words\"\nC='x=y'\nnot a pair\n"
            )
            (root / ".devcontainer" / ".env").write_text("A=override\n")
            self.assertEqual(
                read_env_file(root / ".devcontainer" / ".env.development"),
                {"A": "1", "B": "two words", "C": "x=y"},
            )
            self.assertEqual(probe_env(root)["A"], "override")
            self.assertEqual(read_env_file(root / "missing"), {})


class CliOutcomeTest(unittest.TestCase):
    def test_reads_the_last_outcome_line(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "up.log"
            log.write_text(
                "[2026] building\n"
                '{"outcome":"success","containerId":"abc","remoteUser":"vscode"}\n'
            )
            self.assertEqual(cli_outcome(log)["containerId"], "abc")
            log.write_text("no json here\n")
            self.assertEqual(cli_outcome(log), {})


if __name__ == "__main__":
    unittest.main()
