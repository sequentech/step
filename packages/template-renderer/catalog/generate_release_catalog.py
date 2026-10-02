#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Rebuild the offline catalog from the pinned Step 10.0 release assets."""
import base64
import copy
import hashlib
import json
from pathlib import Path
import re
import subprocess

RELEASE = "4c97c734f67f648cb7fca93556fdb34ccdc7df38"
ROOT = Path(__file__).resolve().parents[3]
DESTINATION = Path(__file__).parent / "v1/catalog.json"
ASSETS = ".devcontainer/minio/public-assets/"
DATABASE = {
    "activity_logs", "ballot_images", "ballot_receipt", "credentials",
    "electoral_results", "initialization_report", "manual_verification",
    "participation_report",
}


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT)


def read(name):
    return git("show", f"{RELEASE}:{ASSETS}{name}")


def schema(value):
    if value is None:
        return {}
    if isinstance(value, bool):
        return {"type": "boolean"}
    if isinstance(value, (int, float)):
        return {"type": "number"}
    if isinstance(value, str):
        return {"type": "string"}
    if isinstance(value, list):
        return {"type": "array", "items": schema(value[0]) if value else {}}
    return {"type": "object", "properties": {k: schema(v) for k, v in value.items()},
            "additionalProperties": True}


def main():
    previous = {r["id"]: r for r in json.loads(DESTINATION.read_text())["reports"]}
    paths = git("ls-tree", "-r", "--name-only", RELEASE, ASSETS).decode().splitlines()
    families = sorted(p[len(ASSETS):-len("_user.hbs")] for p in paths if p.endswith("_user.hbs"))
    reports = []
    for family in families:
        source = read(f"{family}_user.hbs").decode()
        wrapper = read(f"{family}_system.hbs").decode()
        fixture_bytes = read(f"{family}.json")
        fixture = json.loads(fixture_bytes)
        release_source, release_wrapper = source, wrapper
        release_fixture = copy.deepcopy(fixture)
        compatibility_notes = []
        if family == "ballot_images":
            # The release fixture repeats 198 ballots. A representative editor
            # sample avoids embedding hundreds of logo copies in static output;
            # runtime report limits and the original fixture remain unchanged.
            original_ballot_count = len(fixture["data"]["ballot_data"])
            fixture["data"]["ballot_data"] = fixture["data"]["ballot_data"][:2]
            compatibility_notes.append(f"The Studio sample contains the first two representative ballots from the {original_ballot_count}-ballot release fixture. The release fixture repeats one Senate contest; its original path and SHA-256 are retained. Production report limits are unchanged.")
            changes = {
                '<img class="sequent-logo.svg image-url" />': '<img src="sequent-logo.svg" class="sequent-logo.svg image-url" />',
                '<img src"printing-office-logo.jpg" class="image-url circle" />': '<img src="printing-office-logo.jpg" class="printing-office-logo.jpg image-url circle" />',
                'id="qrcode-0-{{ballot_index}}"': 'id="qrcode-{{contest_index}}-{{ballot_index}}"',
                "document.querySelectorAll('.image-url').forEach(img => {": "document.querySelectorAll('.image-url').forEach(img => {\n        if (img.getAttribute('src')?.startsWith('data:')) return;",
            }
            for before, after in changes.items():
                assert source.count(before) == 1
                source = source.replace(before, after)
            compatibility_notes.append("Corrected missing logo URLs and the printing-office filename class. The native URL resolver now preserves already inlined preview images. QR element IDs include the contest index, matching the generator for ballots with multiple contests.")
        empty_header_images = {
            "ov_pre_enrolled_approved", "ov_with_voting_status", "ovcs_events",
            "ovcs_information", "ovcs_statistics", "overseas_voters",
            "pre_enrolled_ov_but_disapproved", "pre_enrolled_ov_subject_to_manual_validation",
            "statistical_report", "status", "transmission_report", "vote_receipt",
        }
        if family in empty_header_images:
            source, removed = re.subn(r'<img\s+src=""\s*/>', '', source)
            assert removed == 1
            compatibility_notes.append("Removed an unused empty right-header image placeholder while retaining its layout cell.")
        if family == "voters_turnout_percentage":
            anchor = "    <!-- End of report section -->\n    <tr class=\"report-end\">"
            assert source.count(anchor) == 1
            source = source.replace(anchor, '    <tr class="qr-codes"><td align="center"><div class="qrcode" id="qrcode"></div></td></tr>\n\n' + anchor)
            compatibility_notes.append("Restored the missing qrcode element required by the existing turnout metadata QR generator.")
        if family == "velvet_vote_receipt":
            broken_logo = '                        sequent-logo.svg\n                        <img src="" class="comissioner-logo"'
            assert source.count(broken_logo) == 1
            source = source.replace(broken_logo, '                        {{#if ../comissioner_logo}}\n                        <img src="{{../comissioner_logo}}" class="comissioner-logo"')
            broken_open = "{ { #each receipts } }"
            broken_close = "{\n                {\n                    /each}}"
            assert source.count(broken_open) == source.count(broken_close) == 1
            source = source.replace(broken_open, "{{#each receipts}}")
            source = source.replace(broken_close, "{{/each}}")
            fixture["file_qrcode_lib"] = "/assets/qrcode.min.js"
            compatibility_notes.append("Repaired the unmatched commissioner-logo conditional and its empty image URL, restored the malformed Handlebars QR loop, and supplied the bundled QR library in the sample.")
        if family == "ovcs_information":
            obsolete_start = '<script>\n  // Assume we have an array of URLs to encode'
            assert wrapper.count(obsolete_start) == 1
            begin = wrapper.index(obsolete_start)
            end = wrapper.index("</script>", begin) + len("</script>")
            wrapper = wrapper[:begin] + wrapper[end:]
            compatibility_notes.append("Removed the obsolete second QR generator that targeted a nonexistent qrcode-container. The user template already generates one metadata QR code for each sample area.")
        if family == "activity_logs":
            assert source.count('src="{{logo}}"') == 1
            source = source.replace('src="{{logo}}"', 'src="{{logo_img}}"')
            compatibility_notes.append("Corrected the logo image binding to logo_img, matching the existing conditional and release sample field.")
        config_path = f"{family}_extra_config.json"
        has_config = ASSETS + config_path in paths
        configuration = json.loads(read(config_path)) if has_config else {
            "pdf_options": {}, "report_options": {},
            "communication_templates": {"email_config": {"subject": "", "plaintext_body": "", "html_body": None}, "sms_config": {"message": ""}},
        }
        configuration["email"] = copy.deepcopy(configuration["communication_templates"]["email_config"])
        configuration["sms"] = copy.deepcopy(configuration["communication_templates"]["sms_config"])
        system_data = {}
        if "file_qrcode_lib" in wrapper:
            system_data["file_qrcode_lib"] = "/assets/qrcode.min.js"
        if family in {"ballot_receipt", "manual_verification", "credentials"}:
            system_data["file_logo"] = "/assets/sequent-logo.svg"
        if family == "ballot_receipt":
            system_data["title"] = "Ballot receipt - Sequentech"
        if family == "ballot_images":
            system_data["title"] = "Ballot Images - Sequentech"
        preview_assets = {}
        references = source + wrapper + json.dumps(fixture) + json.dumps(system_data)
        names = set(re.findall(r"[\w-]+\.(?:svg|jpe?g|png|min.js)", references))
        for name in sorted(names):
            if ASSETS + name not in paths:
                continue
            content = read(name)
            mime = "text/javascript" if name.endswith(".js") else "image/svg+xml" if name.endswith(".svg") else "image/jpeg" if name.endswith((".jpg", ".jpeg")) else "image/png"
            asset = {"mime": mime, "base64": base64.b64encode(content).decode()}
            # Release templates use both bare relative URLs and /assets URLs.
            preview_assets[name] = asset
            preview_assets["assets/" + name] = asset
        old = previous.get(family, {})
        sample_name = "Release 10.0 sample"
        if family == "ballot_images":
            sample_name += f" (2 of {original_ballot_count} ballots)"
        scenarios = [{"name": sample_name, "data": fixture}]
        if family == "electoral_results":
            scenarios += [s for s in old.get("scenarios", [])[1:] if s["name"] != "Release 10.0 sample"]
        if family == "credentials":
            for label, changes in [
                ("Structured credential", {"password": "0012-3456-7890-1234"}),
                ("Plain credential", {"password": "Plain-9!ab&<xyz>"}),
                ("Unicode and long names", {"voter_first_name": "María José & <Anne>", "voter_last_name": "Łukasiewicz O’Connor García-López", "voter_full_name": "María José & <Anne> Łukasiewicz O’Connor García-López", "issue_date": "2 de octubre de 2026"}),
                ("Empty optional names", {"voter_first_name": "", "voter_last_name": "", "voter_full_name": ""}),
            ]:
                scenarios.append({"name": label, "data": {**fixture, **changes}})
        communication_data = {
            "user": {"first_name": "María", "last_name": "O’Connor", "username": "voter-001", "email": "voter@example.org", "reference": "001234", "attributes": {"reference": ["001234"], "groups": ["North", "Community"]}},
            "tenant_id": "00000000-0000-4000-8000-000000000001",
            "election_event": {"id": "00000000-0000-4000-8000-000000000002", "name": "Community election"},
            "vote_url": "https://vote.example.org/tenant/demo/event/community/login",
        }
        report = {
            "id": family,
            "label": "Voter information letter (VIL)" if family == "credentials" else old.get("label", family.replace("_", " ").capitalize()),
            "baseName": family, "release": "10.0", "sourceCommit": RELEASE,
            "exportTarget": "database" if family in DATABASE else "public-assets",
            "sourcePaths": {"user": ASSETS + family + "_user.hbs", "system": ASSETS + family + "_system.hbs", "data": ASSETS + family + ".json", "extraConfig": ASSETS + config_path if has_config else None},
            "sourceSha256": hashlib.sha256(release_source.encode()).hexdigest(),
            "wrapperSha256": hashlib.sha256(release_wrapper.encode()).hexdigest(),
            "sampleSha256": hashlib.sha256(fixture_bytes).hexdigest(),
            "sourceExact": source == release_source,
            "wrapperExact": wrapper == release_wrapper,
            "sampleExact": fixture == release_fixture,
            "compatibilityNotes": compatibility_notes,
            "source": source, "wrapper": wrapper,
            "systemData": system_data, "communicationData": communication_data,
            "previewAssets": preview_assets,
            "configuration": configuration, "schema": schema(fixture), "scenarios": scenarios,
            "defaultLanguage": "en", "supportedLanguages": old.get("supportedLanguages", ["en"]),
            "translations": old.get("translations", {}),
        }
        if not has_config:
            report["notes"] = ["Release 10.0 ships no extra configuration file for this template; PDF and message settings start empty."]
        reports.append(report)
    result = {"version": 1, "release": "10.0", "sourceCommit": RELEASE, "reports": reports}
    DESTINATION.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(f"Wrote {len(reports)} release 10.0 template families ({len(DATABASE)} database, {len(reports) - len(DATABASE)} public-assets).")


if __name__ == "__main__":
    main()
