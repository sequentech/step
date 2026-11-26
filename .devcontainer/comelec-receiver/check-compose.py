# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequent.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Validate the effective receiver platform configuration without emitting any values."""
import json
import os
import stat
import re
import sys
from urllib.parse import urlsplit

P = "WINDMILL_WEBAUTHN_PACKAGES"
UUID4 = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}")
LABEL = re.compile(r"[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?")
TARGETS = {"harvest", "windmill", "beat"}
LEGACY_SITE_SUBJECT = "SEQUENT_RECEIVER_ADMIN_SUBJECT"
LEGACY_ENV_SUBJECTS = P + "_ROSTER_ADMIN_SUBJECTS"
FIELDS = {P, *(P + "_" + x for x in ("TENANT_ID", "EXPECTED_RP_ID",
                                     "EXPECTED_ORIGIN", "REQUIRED_APPROVALS"))}


def demand(ok, description):
    if not ok:
        raise ValueError(description)


def host(value):
    return isinstance(value, str) and len(value) <= 253 and "." in value and all(
        LABEL.fullmatch(label) for label in value.split(".")
    )


SITE = {
    "SUPER_ADMIN_TENANT_ID": P + "_TENANT_ID",
    "SEQUENT_RECEIVER_RP_ID": P + "_EXPECTED_RP_ID",
    "SEQUENT_RECEIVER_ORIGIN": P + "_EXPECTED_ORIGIN",
    "SEQUENT_RECEIVER_APPROVALS": P + "_REQUIRED_APPROVALS",
}


def read_site(path):
    demand(os.path.isabs(path) and os.path.realpath(path) == path,
           "site file path must be canonical and absolute")
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK),
                   encoding="ascii") as f:
        info = os.fstat(f.fileno())
        demand(stat.S_ISREG(info.st_mode) and info.st_uid == os.geteuid()
               and not info.st_mode & 0o022 and info.st_size <= 4096, "unsafe site file")
        rows = f.read(4097)
    demand(len(rows) <= 4096, "oversized site file")
    values = {}
    legacy_seen = False
    for line in rows.splitlines():
        if not line or line.startswith("#"):
            continue
        key, sep, v = line.partition("=")
        if key == LEGACY_SITE_SUBJECT:
            demand(not legacy_seen and sep and UUID4.fullmatch(v),
                   "invalid deprecated site input")
            legacy_seen = True
            continue
        demand(key in SITE and key not in values and sep and re.fullmatch(r"[a-z0-9:/.-]+", v),
               "invalid site input")
        values[key] = v
    values.setdefault("SEQUENT_RECEIVER_APPROVALS", "2")
    demand(values.keys() == SITE.keys(), "missing site input")
    demand(all(k not in os.environ or os.environ[k] == v for k, v in values.items()),
           "ambient site input conflict")
    demand(not any((k.startswith(P) or k.startswith("ARCHITECT_ROSTER_ACTIVATION_")
                    or k.startswith("SEQUENT_RECEIVER_") and k not in SITE)
                   and k not in (LEGACY_SITE_SUBJECT, LEGACY_ENV_SUBJECTS) for k in os.environ),
           "unexpected ambient receiver policy")
    trust = {P: "portal-managed", **{SITE[k]: v for k, v in values.items()}}
    return {"services": {name: {"environment": {
        **trust, **({"SUPER_ADMIN_TENANT_ID": values["SUPER_ADMIN_TENANT_ID"]} if name == "beat" else {})
    }} for name in TARGETS}}


def check(document, original, overlay):
    services = document["services"]
    demand(TARGETS <= services.keys(), "missing expected receiver workload")
    trust = None
    for name, service in services.items():
        effective = service.get("environment", {})
        demand(not any(k.startswith("ARCHITECT_ROSTER_ACTIVATION_") for k in effective),
               "unreviewed activation policy in Compose model")
        settings = {k: v for k, v in effective.items() if k.startswith(P) and k != LEGACY_ENV_SUBJECTS}
        if name not in TARGETS:
            demand(not settings, "receiver policy also affects a nonreceiver workload")
            continue
        demand(set(settings) == FIELDS and all(isinstance(v, str) for v in settings.values()),
               "incomplete, mixed or invalid receiver policy")
        if trust is None:
            trust = settings
        demand(settings == trust and settings == {k: v for k, v in overlay["services"][name]["environment"].items() if k.startswith(P) and k != LEGACY_ENV_SUBJECTS}, "receiver policy differs across workloads")
        demand(effective.get("SUPER_ADMIN_TENANT_ID") == trust[P + "_TENANT_ID"],
               "receiver policy does not name the resolved Default tenant")
    base_services = original["services"]
    demand(TARGETS <= base_services.keys(), "receiver overlay cannot create missing workload")
    for name in TARGETS:
        base_env = base_services[name].get("environment", {})
        demand(not any(k.startswith(P) and k != LEGACY_ENV_SUBJECTS for k in base_env), "existing receiver policy requires an independent migration")
        if name != "beat" or "SUPER_ADMIN_TENANT_ID" in base_env:
            demand(base_env.get("SUPER_ADMIN_TENANT_ID") == trust[P + "_TENANT_ID"],
                   "site input conflicts with the existing receiver Default tenant")
    demand(trust[P] == "portal-managed" and UUID4.fullmatch(trust[P + "_TENANT_ID"])
           and re.fullmatch(r"([1-9]|1[0-6])", trust[P + "_REQUIRED_APPROVALS"]),
           "invalid receiver mode, tenant or approval floor")
    rp = trust[P + "_EXPECTED_RP_ID"]
    origin = trust[P + "_EXPECTED_ORIGIN"]
    u = urlsplit(origin)
    demand(host(rp) and host(u.hostname) and (u.hostname == rp or u.hostname.endswith("." + rp))
           and u.scheme == "https" and origin == "https://" + u.netloc
           and (u.netloc == u.hostname or 0 < (u.port or 0) <= 65535 and u.port != 443
                and u.netloc == f"{u.hostname}:{u.port}"),
           "invalid producer RP or exact HTTPS origin")


if __name__ == "__main__":
    try:
        if sys.argv[1] == "--site":
            print(json.dumps(read_site(sys.argv[2])))
            sys.exit(0)
        with open(sys.argv[1], encoding="utf-8") as f, open(sys.argv[2], encoding="utf-8") as base:
            resolved = json.load(f)
            with open(sys.argv[4], encoding="utf-8") as overlay:
                check(resolved, json.load(base), json.load(overlay))
        with open(sys.argv[3], encoding="utf-8") as rerender:
            demand(json.load(rerender) == resolved, "resolved graph changed on second render")
    except (ValueError, KeyError, TypeError, OSError):
        raise SystemExit("Receiver Compose scope/inputs are not valid; no services started.")
    print("Resolved portal-managed receiver scope checked; roster activation is still a separate operator action.")
