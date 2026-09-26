# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests for scripts.dev.mode and the devcontainer files it describes."""

import copy
import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
import unittest.mock
from pathlib import Path

from scripts.dev.mode.checkout import Checkout, parse_dotenv
from scripts.dev.mode.cli import ModeError, _selected_servers
from scripts.dev.mode.docker import (
    Compose,
    ContainerState,
    ContainerSummary,
    PortBinding,
    container_state,
    parse_ports,
    parse_ps,
)
from scripts.dev.mode.manifest import (
    ManifestError,
    ReadyWhen,
    load_manifest,
    parse_manifest,
)
from scripts.dev.mode.plan import (
    ConflictKind,
    PlanError,
    Readiness,
    closure,
    dependencies,
    find_conflicts,
    healthcheck_drifted,
    published_ports,
    readiness,
)
from scripts.dev.mode.servers import Devcontainer, listening_sockets

ROOT = Path(__file__).resolve().parents[2]
DEVCONTAINER = ROOT / ".devcontainer"
PROJECT = "step_devcontainer"


def strip_jsonc(text):
    """JSON with comments and trailing commas, as devcontainer.json allows."""
    output = []
    index = 0
    in_string = False
    while index < len(text):
        char = text[index]
        if in_string:
            output.append(char)
            if char == "\\":
                output.append(text[index + 1])
                index += 2
                continue
            in_string = char != '"'
            index += 1
        elif char == '"':
            in_string = True
            output.append(char)
            index += 1
        elif text.startswith("//", index):
            end = text.find("\n", index)
            index = len(text) if end < 0 else end
        elif text.startswith("/*", index):
            index = text.index("*/", index) + 2
        else:
            output.append(char)
            index += 1
    return re.sub(r",(\s*[}\]])", r"\1", "".join(output))


def load_jsonc(path):
    return json.loads(strip_jsonc(path.read_text()))


def valid_document():
    return {
        "modes": [
            {
                "name": "ui",
                "summary": "UI",
                "config": "ui/devcontainer.json",
                "composeFiles": ["docker-compose.yml"],
                "services": ["devcontainer"],
                "servers": ["storybook"],
                "defaultServers": ["storybook"],
                "readyTimeoutSeconds": 60,
            }
        ],
        "servers": [
            {
                "name": "storybook",
                "port": 6006,
                "command": ["yarn", "storybook"],
                "readyPath": "/index.json",
            }
        ],
        "services": [{"name": "keycloak", "url": "http://127.0.0.1:8090"}],
    }


def state(service="svc", status="running", health=None, exit_code=0, folder=None):
    return ContainerState(
        id="0123456789ab" + "0" * 52,
        name=service,
        service=service,
        status=status,
        health=health,
        exit_code=exit_code,
        restart_policy="no",
        local_folder=folder,
    )


class ManifestTest(unittest.TestCase):
    def test_valid_document(self):
        manifest = parse_manifest(valid_document())
        mode = manifest.mode("ui")
        self.assertEqual(mode.services, ("devcontainer",))
        self.assertEqual(mode.default_servers, ("storybook",))
        self.assertEqual(manifest.servers["storybook"].port, 6006)
        self.assertEqual(manifest.settings("keycloak").url, "http://127.0.0.1:8090")
        # Services without settings have neither URL nor probe.
        self.assertIsNone(manifest.settings("postgres").probe)

    def test_unknown_mode_names_the_choices(self):
        with self.assertRaisesRegex(ManifestError, "choose one of ui"):
            parse_manifest(valid_document()).mode("backend")

    def assert_rejected(self, change, message):
        document = valid_document()
        change(document)
        with self.assertRaisesRegex(ManifestError, message):
            parse_manifest(document)

    def test_rejects_a_mode_without_the_devcontainer(self):
        self.assert_rejected(
            lambda d: d["modes"][0].update(services=["keycloak"]),
            "must include 'devcontainer'",
        )

    def test_rejects_undefined_servers(self):
        self.assert_rejected(
            lambda d: d["modes"][0].update(servers=["storybook", "portal"]),
            "undefined servers portal",
        )

    def test_rejects_default_servers_the_mode_lacks(self):
        self.assert_rejected(
            lambda d: d["modes"][0].update(servers=[], defaultServers=["storybook"]),
            "default servers storybook not listed",
        )

    def test_rejects_duplicate_modes_and_ports(self):
        self.assert_rejected(
            lambda d: d["modes"].append(copy.deepcopy(d["modes"][0])),
            "mode 'ui' is defined twice",
        )
        self.assert_rejected(
            lambda d: d["servers"].append(dict(d["servers"][0], name="other")),
            "server port '6006' is defined twice",
        )

    def test_rejects_wrong_types_and_values(self):
        self.assert_rejected(
            lambda d: d["servers"][0].update(port=True), "'port' must be a int"
        )
        self.assert_rejected(
            lambda d: d["servers"][0].update(port=70000), "out of range"
        )
        self.assert_rejected(
            lambda d: d["servers"][0].update(readyPath="index"), "start with '/'"
        )
        self.assert_rejected(
            lambda d: d["servers"][0].update(command=[]), "empty command"
        )
        self.assert_rejected(lambda d: d["services"][0].update(probe=[]), "empty probe")
        self.assert_rejected(
            lambda d: d["modes"][0].update(readyTimeoutSeconds=0), "must be positive"
        )
        self.assert_rejected(
            lambda d: d["modes"][0].pop("composeFiles"), "missing 'composeFiles'"
        )
        self.assert_rejected(
            lambda d: d["modes"][0].update(services=["devcontainer", "devcontainer"]),
            "repeats an entry",
        )

    def test_ready_when(self):
        document = valid_document()
        document["services"].append({"name": "init", "readyWhen": "exited"})
        manifest = parse_manifest(document)
        self.assertIs(manifest.settings("init").ready_when, ReadyWhen.EXITED)
        self.assertIs(manifest.settings("keycloak").ready_when, ReadyWhen.RUNNING)
        self.assert_rejected(
            lambda d: d["services"].append({"name": "x", "readyWhen": "healthy"}),
            "readyWhen must be one of running, exited",
        )
        self.assert_rejected(
            lambda d: d["services"].append(
                {"name": "x", "readyWhen": "exited", "probe": ["true"]}
            ),
            "a job that exits cannot be probed",
        )

    def test_repository_manifest_loads(self):
        manifest = load_manifest(ROOT)
        self.assertEqual(
            manifest.mode_names(), ["ui-only", "ui-keycloak", "backend", "full"]
        )


class DotenvTest(unittest.TestCase):
    def test_parses_compose_style_lines(self):
        values = parse_dotenv(
            "# comment\n"
            "COMPOSE_PROJECT_NAME=step_devcontainer\n"
            "export QUOTED='/home/me/my step'\n"
            'DOUBLE="harvest:${HARVEST_PORT}"\n'
            "EMPTY=\n"
            "INLINE=value # trailing comment\n"
            "HASH=a#b\n"
            "not a pair\n"
        )
        self.assertEqual(
            values,
            {
                "COMPOSE_PROJECT_NAME": "step_devcontainer",
                "QUOTED": "/home/me/my step",
                "DOUBLE": "harvest:${HARVEST_PORT}",
                "EMPTY": "",
                "INLINE": "value",
                "HASH": "a#b",
            },
        )


class CheckoutTest(unittest.TestCase):
    def make_checkout(self, directory, host_folder):
        root = Path(directory) / "step"
        (root / ".devcontainer").mkdir(parents=True)
        (root / ".devcontainer" / ".env").write_text(
            "COMPOSE_PROJECT_NAME=step_devcontainer\n"
        )
        env = {
            "COMPOSE_PROJECT_NAME": PROJECT,
            "LOCAL_WORKSPACE_FOLDER": str(host_folder),
        }
        return root, Checkout(root, env)

    def test_compose_runs_from_the_host_path_when_it_is_the_same_checkout(self):
        with tempfile.TemporaryDirectory() as directory:
            host = Path(directory) / "host-view"
            root, _ = self.make_checkout(directory, host)
            # The devcontainer sees the checkout at a second path, like its
            # extra mount of the checkout's parent.
            host.symlink_to(root)
            checkout = Checkout(root, {"LOCAL_WORKSPACE_FOLDER": str(host)})
            self.assertEqual(checkout.compose_dir, host / ".devcontainer")
            self.assertTrue(checkout.binds_resolve_on_host)

    def test_compose_falls_back_to_the_local_path(self):
        with tempfile.TemporaryDirectory() as directory:
            # The host path is not visible, or is another checkout.
            other = Path(directory) / "other"
            (other / ".devcontainer").mkdir(parents=True)
            (other / ".devcontainer" / ".env").write_text(
                "COMPOSE_PROJECT_NAME=other\n"
            )
            for host in (Path(directory) / "missing", other):
                root = Path(directory) / f"step-{host.name}"
                (root / ".devcontainer").mkdir(parents=True)
                (root / ".devcontainer" / ".env").write_text("")
                checkout = Checkout(root, {"LOCAL_WORKSPACE_FOLDER": str(host)})
                self.assertEqual(checkout.compose_dir, root / ".devcontainer")
                self.assertFalse(checkout.binds_resolve_on_host)

    def test_names_and_paths_default_to_the_canonical_checkout(self):
        checkout = Checkout(Path("/workspaces/step"), {"COMPOSE_PROJECT_NAME": PROJECT})
        self.assertEqual(checkout.project, PROJECT)
        self.assertEqual(checkout.name_prefix, "")
        self.assertEqual(checkout.container_root, "/workspaces/step")
        self.assertEqual(checkout.host_root, Path("/workspaces/step"))

    def test_missing_project_and_volumes_are_errors(self):
        checkout = Checkout(Path("/tmp/x"), {})
        with self.assertRaisesRegex(Exception, "COMPOSE_PROJECT_NAME"):
            checkout.project
        with self.assertRaisesRegex(Exception, "DEVCONTAINER_NIX_VOLUME"):
            checkout.cache_volumes()


class DockerOutputTest(unittest.TestCase):
    def test_parse_ports(self):
        self.assertEqual(
            parse_ports("0.0.0.0:8090->8090/tcp, [::]:8090->8090/tcp, 5432/tcp"),
            (PortBinding("0.0.0.0", 8090, "tcp"), PortBinding("::", 8090, "tcp")),
        )
        self.assertEqual(
            parse_ports("127.0.0.1:3000-3001->3000-3001/tcp, 9000->9000/udp"),
            (
                PortBinding("127.0.0.1", 3000, "tcp"),
                PortBinding("127.0.0.1", 3001, "tcp"),
                PortBinding("", 9000, "udp"),
            ),
        )
        self.assertEqual(parse_ports(""), ())

    def test_parse_ps(self):
        line = "\t".join(
            [
                "abc",
                "keycloak",
                "running",
                PROJECT,
                "keycloak",
                "0.0.0.0:8090->8090/tcp",
            ]
        )
        [container] = parse_ps(line + "\n\n")
        self.assertEqual(container.name, "keycloak")
        self.assertEqual(container.project, PROJECT)
        self.assertEqual(container.ports, (PortBinding("0.0.0.0", 8090, "tcp"),))
        with self.assertRaisesRegex(Exception, "unexpected docker ps line"):
            parse_ps("abc\tkeycloak\n")

    def test_port_overlap(self):
        wildcard = PortBinding("", 8090, "tcp")
        self.assertTrue(wildcard.overlaps(PortBinding("127.0.0.1", 8090, "tcp")))
        self.assertTrue(
            PortBinding("::", 8090, "tcp").overlaps(
                PortBinding("10.0.0.1", 8090, "tcp")
            )
        )
        self.assertTrue(
            PortBinding("127.0.0.1", 8090, "tcp").overlaps(
                PortBinding("127.0.0.1", 8090, "tcp")
            )
        )
        self.assertFalse(
            PortBinding("127.0.0.1", 8090, "tcp").overlaps(
                PortBinding("10.0.0.1", 8090, "tcp")
            )
        )
        self.assertFalse(wildcard.overlaps(PortBinding("", 8091, "tcp")))
        self.assertFalse(wildcard.overlaps(PortBinding("", 8090, "udp")))


class ComposeTest(unittest.TestCase):
    def checkout(self, directory):
        root = Path(directory) / "step"
        (root / ".devcontainer").mkdir(parents=True)
        (root / ".devcontainer" / ".env").write_text("")
        env = {
            "COMPOSE_PROJECT_NAME": "step-wt_devcontainer",
            "LOCAL_WORKSPACE_FOLDER": str(root),
            "KC_HOSTNAME": "localhost",
        }
        return Checkout(root, env)

    def test_argv_names_the_project_and_the_host_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            checkout = self.checkout(directory)
            compose = Compose(checkout, ("docker-compose.yml", "overlay.yml"))
            folder = str(checkout.root / ".devcontainer")
            self.assertEqual(
                compose.argv("up", "--detach"),
                [
                    "compose",
                    "--project-name",
                    "step-wt_devcontainer",
                    "--project-directory",
                    folder,
                    "--file",
                    f"{folder}/docker-compose.yml",
                    "--file",
                    f"{folder}/overlay.yml",
                    "up",
                    "--detach",
                ],
            )

    def test_environment_leaves_interpolation_to_the_env_file(self):
        with tempfile.TemporaryDirectory() as directory:
            compose = Compose(self.checkout(directory), ("docker-compose.yml",))
            outer = {
                "KC_HOSTNAME": "stale",
                "COMPOSE_PROJECT_NAME": "step_devcontainer",
                "COMPOSE_FILE": "other.yml",
                "DOCKER_HOST": "unix:///tmp/docker.sock",
            }
            with unittest.mock.patch.dict(os.environ, outer, clear=True):
                self.assertEqual(
                    compose.environment(), {"DOCKER_HOST": "unix:///tmp/docker.sock"}
                )


class ContainerStateTest(unittest.TestCase):
    def test_reads_health_restart_policy_and_labels(self):
        document = {
            "Id": "abc",
            "Name": "/keycloak",
            "State": {
                "Status": "running",
                "ExitCode": 0,
                "Health": {"Status": "healthy"},
            },
            "HostConfig": {"RestartPolicy": {"Name": "always"}},
            "Config": {
                "Healthcheck": {"Test": ["CMD-SHELL", "true"]},
                "Labels": {
                    "com.docker.compose.service": "keycloak",
                    "devcontainer.local_folder": "/home/me/step",
                },
            },
        }
        self.assertEqual(
            container_state(document),
            ContainerState(
                "abc",
                "keycloak",
                "keycloak",
                "running",
                "healthy",
                0,
                "always",
                "/home/me/step",
                ("CMD-SHELL", "true"),
            ),
        )

    def test_missing_sections_mean_no_health_and_no_restart(self):
        parsed = container_state(
            {"Id": "abc", "Name": "/job", "State": {"Status": "exited"}}
        )
        self.assertEqual(
            (parsed.health, parsed.restart_policy, parsed.service), (None, "no", "")
        )


class ServerSelectionTest(unittest.TestCase):
    def setUp(self):
        self.manifest = load_manifest(ROOT)
        self.mode = self.manifest.mode("ui-only")

    def names(self, spec):
        return [
            server.name for server in _selected_servers(self.manifest, self.mode, spec)
        ]

    def test_specs(self):
        self.assertEqual(self.names("default"), ["storybook-ui-essentials"])
        self.assertEqual(self.names("none"), [])
        self.assertEqual(
            self.names("admin-portal, storybook-voting-portal"),
            ["admin-portal", "storybook-voting-portal"],
        )

    def test_rejects_servers_of_other_modes(self):
        backend = self.manifest.mode("backend")
        with self.assertRaisesRegex(
            ModeError, "not in mode backend; its servers are none"
        ):
            _selected_servers(self.manifest, backend, "admin-portal")


class RecordedServerTest(unittest.TestCase):
    """Process ids step-dev recorded only count in the container that ran them."""

    def devcontainer(self, directory, container_id):
        root = Path(directory)
        (root / ".cache" / "dev-mode").mkdir(parents=True)
        (root / ".cache" / "dev-mode" / "storybook.pid").write_text(
            f"4242 {container_id}\n"
        )
        checkout = Checkout(root, {})
        return Devcontainer(checkout, state("devcontainer"))

    def test_pid_of_the_current_container(self):
        server = parse_manifest(valid_document()).servers["storybook"]
        with tempfile.TemporaryDirectory() as directory:
            devcontainer = self.devcontainer(directory, state("devcontainer").id)
            self.assertEqual(devcontainer._recorded_pid(server), 4242)

    def test_pid_of_a_replaced_container_is_ignored(self):
        server = parse_manifest(valid_document()).servers["storybook"]
        with tempfile.TemporaryDirectory() as directory:
            devcontainer = self.devcontainer(directory, "f" * 64)
            self.assertIsNone(devcontainer._recorded_pid(server))


SERVICES = {
    "devcontainer": {},
    "postgres-volume-init": {},
    "postgres": {"depends_on": {"postgres-volume-init": {}}},
    "postgres-keycloak": {"depends_on": {"postgres": {}}},
    "harvest": {"depends_on": {"devcontainer": {}}, "volumes_from": ["devcontainer"]},
    "keycloak": {
        "depends_on": {"postgres-keycloak": {}, "harvest": {}},
        "container_name": "keycloak",
        "ports": [{"target": 8090, "published": "8090", "protocol": "tcp"}],
    },
}


class PlanTest(unittest.TestCase):
    def test_dependencies_include_volumes_from_services_only(self):
        service = {
            "depends_on": {"db": {}},
            "volumes_from": [
                "devcontainer",
                "service:cache:ro",
                "container:legacy",
                "db",
            ],
        }
        self.assertEqual(dependencies(service), ["db", "devcontainer", "cache"])

    def test_closure_orders_dependencies_first(self):
        self.assertEqual(
            closure(SERVICES, ["keycloak"]),
            [
                "postgres-volume-init",
                "postgres",
                "postgres-keycloak",
                "devcontainer",
                "harvest",
                "keycloak",
            ],
        )
        self.assertEqual(closure(SERVICES, ["devcontainer"]), ["devcontainer"])

    def test_closure_tolerates_cycles(self):
        services = {"a": {"depends_on": {"b": {}}}, "b": {"depends_on": {"a": {}}}}
        self.assertEqual(closure(services, ["a"]), ["b", "a"])

    def test_closure_names_the_missing_service(self):
        with self.assertRaisesRegex(
            PlanError, "'postgres' \\(needed by postgres-keycloak\\)"
        ):
            closure(
                {"postgres-keycloak": SERVICES["postgres-keycloak"]},
                ["postgres-keycloak"],
            )

    def test_published_ports(self):
        service = {
            "ports": [
                {"target": 8090, "published": "8090", "protocol": "tcp"},
                {"target": 80, "published": "3000-3001", "host_ip": "127.0.0.1"},
                {"target": 5432},
            ]
        }
        self.assertEqual(
            published_ports(service),
            [
                PortBinding("", 8090, "tcp"),
                PortBinding("127.0.0.1", 3000, "tcp"),
                PortBinding("127.0.0.1", 3001, "tcp"),
            ],
        )


def summary(name, project, state="running", ports=()):
    return ContainerSummary(name, name, state, project, name, tuple(ports))


class ConflictTest(unittest.TestCase):
    def conflicts(self, containers, devcontainer=None, names=("keycloak",)):
        return find_conflicts(
            "step-wt_devcontainer",
            "/home/me/step-wt",
            SERVICES,
            list(names),
            containers,
            devcontainer,
        )

    def test_no_conflict_on_an_empty_host(self):
        self.assertEqual(self.conflicts([]), [])

    def test_own_containers_are_not_conflicts(self):
        own = summary(
            "keycloak", "step-wt_devcontainer", ports=[PortBinding("", 8090, "tcp")]
        )
        self.assertEqual(self.conflicts([own]), [])

    def test_container_name_taken_by_another_project(self):
        other = summary("keycloak", PROJECT, state="exited")
        [conflict] = self.conflicts([other])
        self.assertIs(conflict.kind, ConflictKind.CONTAINER_NAME)
        self.assertIn("docker compose -p step_devcontainer stop", conflict.resolution)

    def test_port_published_by_another_running_project(self):
        other = summary(
            "step-keycloak", PROJECT, ports=[PortBinding("0.0.0.0", 8090, "tcp")]
        )
        [conflict] = self.conflicts([other])
        self.assertIs(conflict.kind, ConflictKind.HOST_PORT)
        self.assertIn(
            "8090/tcp is published by container step-keycloak", conflict.detail
        )

    def test_stopped_containers_hold_no_port(self):
        stopped = summary(
            "step-keycloak", PROJECT, "exited", [PortBinding("", 8090, "tcp")]
        )
        self.assertEqual(self.conflicts([stopped]), [])

    def test_a_standalone_container_is_named_in_the_hint(self):
        other = summary("keycloak", "", state="running")
        [conflict] = self.conflicts([other])
        self.assertIn("docker stop keycloak", conflict.resolution)

    def test_another_checkout_with_the_same_folder_name(self):
        devcontainer = state("devcontainer", folder="/home/other/step-wt")
        [conflict] = self.conflicts([], devcontainer, names=("devcontainer",))
        self.assertIs(conflict.kind, ConflictKind.CHECKOUT)
        self.assertEqual(
            self.conflicts(
                [], state("devcontainer", folder="/home/me/step-wt"), ("devcontainer",)
            ),
            [],
        )
        # Containers created before the CLI labelled them are not conflicts.
        self.assertEqual(
            self.conflicts([], state("devcontainer"), ("devcontainer",)), []
        )


class HealthcheckDriftTest(unittest.TestCase):
    RABBITMQ = ["CMD", "rabbitmq-diagnostics", "-q", "check_port_listener", "5672"]

    def container(self, test):
        return ContainerState(
            "i", "rabbitmq", "rabbitmq", "running", None, 0, "no", None, test
        )

    def test_container_created_before_the_check(self):
        service = {"healthcheck": {"test": self.RABBITMQ}}
        self.assertTrue(healthcheck_drifted(service, self.container(None)))
        self.assertTrue(healthcheck_drifted(service, self.container(("CMD", "true"))))
        self.assertFalse(
            healthcheck_drifted(service, self.container(tuple(self.RABBITMQ)))
        )

    def test_image_checks_and_disabled_checks_are_kept(self):
        # Without a Compose health check the container runs its image's one.
        self.assertFalse(healthcheck_drifted({}, self.container(("CMD", "true"))))
        disabled = {"healthcheck": {"test": ["NONE"], "disable": True}}
        self.assertFalse(healthcheck_drifted(disabled, self.container(None)))


class ReadinessTest(unittest.TestCase):
    def check(self, container, probe_ok, expected, detail, when=ReadyWhen.RUNNING):
        self.assertEqual(readiness(container, probe_ok, when), (expected, detail))

    def test_servers(self):
        self.check(None, None, Readiness.MISSING, "not created")
        self.check(state(), None, Readiness.READY, "running")
        self.check(state(health="healthy"), None, Readiness.READY, "healthy")
        self.check(state(health="starting"), None, Readiness.STARTING, "starting")
        # Unhealthy while compiling can still recover; the deadline decides.
        self.check(state(health="unhealthy"), None, Readiness.STARTING, "unhealthy")
        self.check(state(), False, Readiness.STARTING, "probe failing")
        self.check(state(health="healthy"), True, Readiness.READY, "healthy")
        self.check(state(status="created"), None, Readiness.STARTING, "created")
        self.check(
            state(status="restarting", exit_code=101),
            None,
            Readiness.STARTING,
            "restarting after exit code 101",
        )
        # A server that stops, even cleanly, is not up.
        self.check(state(status="exited"), None, Readiness.FAILED, "exited with code 0")
        self.check(
            state(status="exited", exit_code=1),
            None,
            Readiness.FAILED,
            "exited with code 1",
        )

    def test_jobs(self):
        job = ReadyWhen.EXITED
        self.check(state(), None, Readiness.STARTING, "running", job)
        self.check(state(status="exited"), None, Readiness.COMPLETED, "completed", job)
        # A job with a restart policy is done once it has exited successfully.
        self.check(
            state(status="restarting"), None, Readiness.COMPLETED, "completed", job
        )
        self.check(
            state(status="restarting", exit_code=2),
            None,
            Readiness.STARTING,
            "restarting after exit code 2",
            job,
        )
        self.check(
            state(status="exited", exit_code=1),
            None,
            Readiness.FAILED,
            "exited with code 1",
            job,
        )
        self.assertTrue(Readiness.COMPLETED.done)
        self.assertFalse(Readiness.STARTING.done)


class ListenerTest(unittest.TestCase):
    def test_listening_sockets(self):
        header = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when"
        counters = "00000000:00000000 00:00000000 00000000  1000        0"
        tcp = (
            f"{header} retrnsmt   uid  timeout inode\n"
            f"   0: 00000000:1776 00000000:0000 0A {counters} 111 1\n"
            f"   1: 0100007F:0BB8 0100007F:D4F2 01 {counters} 222 1\n"
        )
        any_address = "0" * 32
        tcp6 = (
            f"{header} retrnsmt   uid  timeout inode\n"
            f"   0: {any_address}:0BBA {any_address}:0000 0A {counters} 333 1\n"
        )
        # 0x1776 = 6006 listens; 0x0BB8 = 3000 is an established connection.
        self.assertEqual(listening_sockets([tcp, tcp6]), {6006: {"111"}, 3002: {"333"}})


def compose_available():
    if shutil.which("docker") is None:
        return False
    completed = subprocess.run(
        ["docker", "compose", "version"], capture_output=True, text=True, check=False
    )
    return completed.returncode == 0


def compose_services(files):
    """Services of the Compose files, without contacting a Docker daemon."""
    command = [
        "docker",
        "compose",
        "--project-name",
        PROJECT,
        "--project-directory",
        str(DEVCONTAINER),
        "--env-file",
        str(DEVCONTAINER / ".env.development"),
    ]
    for name in files:
        command += ["--file", str(DEVCONTAINER / name)]
    completed = subprocess.run(
        [*command, "--profile", "*", "config", "--format", "json"],
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(completed.stdout)["services"]


class DevcontainerConfigTest(unittest.TestCase):
    """The devcontainer.json of every mode agrees with the manifest."""

    # Every other key is shared with the full-stack configuration.
    MODE_KEYS = {
        "name",
        "dockerComposeFile",
        "runServices",
        "forwardPorts",
        "initializeCommand",
        "postCreateCommand",
    }

    def setUp(self):
        self.manifest = load_manifest(ROOT)
        self.default = load_jsonc(DEVCONTAINER / "devcontainer.json")

    def config(self, mode):
        return load_jsonc(DEVCONTAINER / mode.config)

    def test_run_services_match_the_manifest(self):
        for mode in self.manifest.modes:
            with self.subTest(mode=mode.name):
                config = self.config(mode)
                if "runServices" in config:
                    self.assertEqual(config["runServices"], list(mode.services))
                self.assertEqual(config["service"], "devcontainer")

    def test_compose_files_match_the_manifest(self):
        for mode in self.manifest.modes:
            with self.subTest(mode=mode.name):
                files = self.config(mode)["dockerComposeFile"]
                files = [files] if isinstance(files, str) else files
                config_dir = (DEVCONTAINER / mode.config).parent
                resolved = [
                    os.path.relpath(config_dir / name, DEVCONTAINER) for name in files
                ]
                self.assertEqual(resolved, list(mode.compose_files))

    def test_initialize_command_checks_its_own_mode(self):
        for mode in self.manifest.modes:
            with self.subTest(mode=mode.name):
                self.assertEqual(
                    self.config(mode)["initializeCommand"],
                    [".devcontainer/scripts/initialize-command.sh", mode.name],
                )

    def test_modes_share_everything_else_with_the_full_stack(self):
        for mode in self.manifest.modes:
            with self.subTest(mode=mode.name):
                config = self.config(mode)
                shared = set(config) | set(self.default)
                for key in sorted(shared - self.MODE_KEYS):
                    self.assertEqual(config.get(key), self.default.get(key), key)

    def test_only_modes_with_a_database_build_step_cli(self):
        # init-cli.sh configures step-cli against Hasura and Keycloak.
        for mode in self.manifest.modes:
            with self.subTest(mode=mode.name):
                has_hasura = "graphql-engine" in mode.services
                self.assertEqual("postCreateCommand" in self.config(mode), has_hasura)

    @unittest.skipUnless(compose_available(), "needs docker compose")
    def test_services_without_run_services_are_the_base_profile(self):
        services = compose_services(["docker-compose.yml"])
        base = sorted(
            name
            for name, service in services.items()
            if "base" in service.get("profiles", [])
        )
        for mode in self.manifest.modes:
            with self.subTest(mode=mode.name):
                if "runServices" not in self.config(mode):
                    self.assertEqual(sorted(mode.services), base)

    @unittest.skipUnless(compose_available(), "needs docker compose")
    def test_services_awaited_to_complete_are_jobs(self):
        for mode in self.manifest.modes:
            services = compose_services(mode.compose_files)
            for name in closure(services, mode.services):
                for dependency, condition in (
                    services[name].get("depends_on", {}).items()
                ):
                    if condition.get("condition") == "service_completed_successfully":
                        with self.subTest(mode=mode.name, job=dependency):
                            self.assertIs(
                                self.manifest.settings(dependency).ready_when,
                                ReadyWhen.EXITED,
                            )

    @unittest.skipUnless(compose_available(), "needs docker compose")
    def test_mode_services_start_nothing_else(self):
        for mode in self.manifest.modes:
            with self.subTest(mode=mode.name):
                services = compose_services(mode.compose_files)
                self.assertEqual(
                    sorted(closure(services, mode.services)), sorted(mode.services)
                )


class ComposeFilesTest(unittest.TestCase):
    """Guards that keep several checkouts on one Docker host apart."""

    FILES = ("docker-compose.yml", "docker-compose-base.yml")

    def active_lines(self, name):
        return [
            line
            for line in (DEVCONTAINER / name).read_text().splitlines()
            if not line.lstrip().startswith("#")
        ]

    def test_container_names_carry_the_checkout_prefix(self):
        for name in self.FILES:
            for line in self.active_lines(name):
                if "container_name:" in line:
                    self.assertIn("container_name: ${DEVCONTAINER_NAME_PREFIX:-}", line)

    def test_workspace_paths_follow_the_checkout(self):
        for name in self.FILES:
            for line in self.active_lines(name):
                if "/workspaces/step" in line:
                    self.assertIn(
                        "${DEVCONTAINER_WORKSPACE_FOLDER:-/workspaces/step}", line
                    )

    def test_vscode_tasks_do_not_pin_the_canonical_project(self):
        tasks = (ROOT / ".vscode" / "tasks.shared.json").read_text()
        self.assertNotIn("COMPOSE_PROJECT_NAME", tasks)
        self.assertNotIn("/workspaces/step", tasks)

    def test_nix_store_volume_follows_the_image_tag(self):
        base = (DEVCONTAINER / "docker-compose-base.yml").read_text()
        [tag] = re.findall(r"image: ghcr\.io/cachix/devenv/devcontainer:(\S+)", base)
        env = parse_dotenv((DEVCONTAINER / ".env.development").read_text())
        self.assertEqual(env["DEVCONTAINER_NIX_VOLUME"], f"step-devcontainer-nix-{tag}")


class InitializeCommandTest(unittest.TestCase):
    """Values initialize-command.sh derives from the checkout's location."""

    TOOLS = ("basename", "cat", "dirname", "sed", "tr")

    def derive(self, host_folder):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "checkout"
            scripts = root / ".devcontainer" / "scripts"
            scripts.mkdir(parents=True)
            shutil.copy(DEVCONTAINER / "scripts" / "initialize-command.sh", scripts)
            shutil.copy(DEVCONTAINER / ".env.development", root / ".devcontainer")
            # Only the tools the derivation needs: no docker, no python3.
            tools = Path(directory) / "bin"
            tools.mkdir()
            for tool in self.TOOLS:
                (tools / tool).symlink_to(shutil.which(tool))
            completed = subprocess.run(
                [shutil.which("bash"), str(scripts / "initialize-command.sh")],
                env={"PATH": str(tools), "LOCAL_WORKSPACE_FOLDER": host_folder},
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            return parse_dotenv((root / ".devcontainer" / ".env").read_text())

    def test_canonical_checkout_keeps_its_names(self):
        env = self.derive("/home/me/src/step")
        self.assertEqual(env["COMPOSE_PROJECT_NAME"], PROJECT)
        self.assertEqual(env["DEVCONTAINER_NAME_PREFIX"], "")
        self.assertEqual(env["DEVCONTAINER_WORKSPACE_FOLDER"], "/workspaces/step")
        self.assertEqual(env["LOCAL_WORKSPACE_FOLDER"], "/home/me/src/step")
        self.assertEqual(env["DEVCONTAINER_HOST_PARENT"], "/home/me/src")

    def test_other_checkouts_get_their_own_names(self):
        env = self.derive("/home/me/src/step-13610-modes")
        self.assertEqual(env["COMPOSE_PROJECT_NAME"], "step-13610-modes_devcontainer")
        self.assertEqual(env["DEVCONTAINER_NAME_PREFIX"], "step-13610-modes-")
        self.assertEqual(
            env["DEVCONTAINER_WORKSPACE_FOLDER"], "/workspaces/step-13610-modes"
        )
        # Characters Compose rejects are dropped, as the Dev Containers CLI does.
        env = self.derive("/home/me/src/My.Step")
        self.assertEqual(env["COMPOSE_PROJECT_NAME"], "mystep_devcontainer")
        self.assertEqual(env["DEVCONTAINER_NAME_PREFIX"], "mystep-")

    def test_the_host_parent_never_hides_container_directories(self):
        for folder in (
            "/workspaces/step",
            "/home/vscode/step",
            "/step",
            "/usr/src/step",
            "/home/step",
        ):
            with self.subTest(folder=folder):
                self.assertEqual(
                    self.derive(folder)["DEVCONTAINER_HOST_PARENT"],
                    "/mnt/checkout-parent",
                )
        self.assertEqual(
            self.derive("/Users/me/step")["DEVCONTAINER_HOST_PARENT"], "/Users/me"
        )


if __name__ == "__main__":
    unittest.main()
