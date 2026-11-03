# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""The ER transmission sample kit for the journeys: its configuration, the
loader, and the altered copies of a downloaded package the receiver's
verifier must refuse."""

import importlib.util
import io
import json
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SAMPLE = ROOT / "scripts/transmission-sample"
KIT = SAMPLE / "kit"
P12_PASSWORD = "sample"
DESTINATION_KEY = KIT / "ccs-manila-private-key.pem"
OTHER_DESTINATION_KEY = KIT / "other-destination-private-key.pem"
SBEI_CERTIFICATES = KIT / "sbei-certificates.pem"
TALLY_SESSION_DATA = "miru:tally-session-data"


def _loader():
    spec = importlib.util.spec_from_file_location(
        "load_sample_config", SAMPLE / "load_sample_config.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


loader = _loader()
CONFIG = json.loads(loader.DEFAULT_CONFIG.read_text())
STATION_ID = CONFIG["country"]["annotations"]["miru:area-station-id"]
REQUIRED_SIGNATURES = int(CONFIG["country"]["annotations"]["miru:area-threshold"])


def ca_bundle():
    return (KIT / "root-ca.pem").read_text() + (KIT / "intermediate-ca.pem").read_text()


def sample_candidates():
    """The CCS ids of the configuration's first contest, in candidate order."""
    return [
        candidate["annotations"]["miru:candidate-id"]
        for candidate in CONFIG["post"]["contests"][0]["candidates"]
    ]


def grouped(sha256):
    """A SHA-256 as the page shows it: in groups of eight."""
    return " ".join(sha256[i : i + 8] for i in range(0, len(sha256), 8))


def _read(archive):
    with zipfile.ZipFile(io.BytesIO(archive)) as opened:
        return {
            info.filename: opened.read(info.filename)
            for info in opened.infolist()
            if not info.is_dir()
        }


def _write(members):
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, data in members.items():
            archive.writestr(name, data)
    return buffer.getvalue()


def _station_zip_name(members):
    (name,) = [
        name
        for name in members
        if name.endswith(f"/er_{STATION_ID}.zip") or name == f"er_{STATION_ID}.zip"
    ]
    return name


def _with_station(package, change):
    """`package` with its station zip's members passed through `change`."""
    members = _read(package)
    name = _station_zip_name(members)
    station = _read(members[name])
    change(station)
    members[name] = _write(station)
    return _write(members)


def _with_json(package, change):
    def edit(station):
        json_name = f"er_{STATION_ID}.json"
        document = json.loads(station[json_name])
        change(document)
        station[json_name] = json.dumps(document, indent=2).encode()

    return _with_station(package, edit)


def without_a_member_signature(package):
    return _with_json(package, lambda document: document["members"].pop())


def with_a_forged_hash(package):
    def forge(document):
        document["sha256Hash"] = "0" * 64

    return _with_json(package, forge)


def with_a_changed_byte(package):
    def flip(station):
        name = f"er_{STATION_ID}.exz"
        data = bytearray(station[name])
        data[len(data) // 2] ^= 0x01
        station[name] = bytes(data)

    return _with_station(package, flip)
