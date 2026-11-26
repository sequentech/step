#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Offline operator fixture. Never run on hosted/receiver election data."""

import argparse
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import time
import uuid

DATABASE = "operational_demo_14990"
URL = "http://127.0.0.1:18889/disposable-officials.crl"
CA_NAME = "Disposable Operational Signing Fixture CA"
P256_SPKI_PREFIX = bytes.fromhex("3059301306072a8648ce3d020106082a8648ce3d03010703420004")
CONFIG = """
[ ca ]
default_ca = issuer
[ issuer ]
database = index.txt
new_certs_dir = issued
certificate = ca.pem
private_key = ca.key
serial = serial
crlnumber = crlnumber
default_md = sha256
default_days = 1
default_crl_days = 1
policy = person
copy_extensions = none
unique_subject = no
x509_extensions = leaf
[ person ]
commonName = supplied
organizationalUnitName = supplied
[ leaf ]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature,nonRepudiation
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always
crlDistributionPoints = URI:""" + URL + "\n"

CHECK = """
SELECT current_database() = 'operational_demo_14990' AND inet_client_addr() IS NULL
   AND current_user = session_user
   AND coalesce(sequent_backend.operational_signing_demo_fixture_id(
       :'tenant'::uuid, :'event'::uuid, 'initialize-voting', :'backend') = :'run'::uuid,false) AS allowed
\\gset
\\if :allowed
\\else
\\quit 9
\\endif
"""


def parse_uuid(text):
    value = uuid.UUID(text)
    if value.int == 0 or str(value) != text:
        raise ValueError("noncanonical fixture UUID")
    return text


def sha(data):
    return hashlib.sha256(data).hexdigest()


def asn1date(text):
    return datetime.datetime.strptime(text.strip(), "%b %d %H:%M:%S %Y GMT").replace(
        tzinfo=datetime.timezone.utc
    ).isoformat()


class Fixture:
    def __init__(self, args):
        self.args = args
        self.work = args.state.resolve(strict=True)
        m = self.work.stat()
        if self.work != args.state or m.st_uid != os.getuid() or stat.S_IMODE(m.st_mode) != 0o700:
            raise ValueError("use a canonical, private (0700) operator state directory")
        self.socket = args.socket.resolve(strict=True)
        if self.socket != args.socket or not self.socket.is_dir():
            raise ValueError("use an existing absolute local PostgreSQL socket directory")
        if not stat.S_ISSOCK((self.socket / f".s.PGSQL.{args.port}").stat().st_mode):
            raise ValueError("fixture socket is not a Unix socket")
        self.environment = dict(os.environ)
        self.environment = {k: v for k, v in self.environment.items() if not k.startswith("PG")}
        self.environment.update({"LC_ALL": "C", "PGCONNECT_TIMEOUT": "3", "PGPASSFILE": "/dev/null"})
        self.vars = dict(tenant=args.tenant, event=args.event, backend=args.backend, run=args.run)
        self.openssl = shutil.which("openssl")
        self.psql = shutil.which("psql")
        if not self.openssl or not self.psql:
            raise ValueError("openssl and psql must be available")
        # PostgreSQL 16's psql may not support numeric \\quit; require an explicit
        # sentinel as well, independently of the client exit status.
        if self.sql("SELECT 'isolated';").strip() != "isolated":
            raise ValueError("refused an unsealed or wrong fixture database")

    def sql(self, body, **extra):
        # This fixture-only command always uses the fixed database, a local
        # Unix socket, and the immutable installation marker. No ambient PG routing.
        variables = self.vars | extra
        command = [self.psql, "-X", "-A", "-t", "-q", "-v", "ON_ERROR_STOP=1",
                   "-h", str(self.socket), "-p", str(self.args.port), "-d", DATABASE, "-w"]
        for key, value in variables.items():
            command += ["-v", f"{key}={value}"]
        result = subprocess.run(command, input=CHECK + "\n" + body, text=True,
                                capture_output=True, check=False, env=self.environment)
        if result.returncode:
            raise ValueError("fixture database operation refused; inspect operator database logs")
        return result.stdout

    def crypto(self, *args, data=None, include_stderr=False):
        result = subprocess.run([self.openssl, *args], input=data, cwd=self.work,
                                capture_output=True, check=False, env=self.environment)
        if result.returncode:
            raise ValueError(f"fixture cryptographic check failed: {args[0]}")
        return result.stdout + result.stderr if include_stderr else result.stdout

    def certificate_info(self, filename):
        details = self.crypto("x509", "-in", filename, "-noout", "-startdate", "-enddate", "-serial")
        values = dict(line.decode().split("=", 1) for line in details.splitlines())
        return dict(before=asn1date(values["notBefore"]), after=asn1date(values["notAfter"]),
                    serial=values["serial"], fp=sha(self.crypto("x509", "-in", filename, "-outform", "DER")),
                    pem=(self.work / filename).read_text())

    def initialize_ca(self):
        if any(self.work.iterdir()):
            raise ValueError("CA creation needs an empty operator state directory")
        (self.work / "issued").mkdir()
        for name, value in {"index.txt": "", "serial": "01\n", "crlnumber": "01\n",
                            "ca.cnf": CONFIG}.items():
            (self.work / name).write_text(value)
        self.crypto("req", "-new", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256",
                    "-nodes", "-keyout", "ca.key", "-out", "ca.pem", "-days", "2",
                    "-subj", f"/CN={CA_NAME}",
                    "-addext", "basicConstraints=critical,CA:TRUE,pathlen:0",
                    "-addext", "keyUsage=critical,keyCertSign,cRLSign",
                    "-addext", "subjectKeyIdentifier=hash")
        self.publish_crl()

    def publish_crl(self):
        self.crypto("ca", "-config", "ca.cnf", "-gencrl", "-out", "crl.pem")
        self.crypto("crl", "-in", "crl.pem", "-CAfile", "ca.pem", "-verify", "-noout")
        number = self.crypto("crl", "-in", "crl.pem", "-crlnumber", "-noout").strip()
        if not number.startswith(b"crlNumber=0x") or int(number.split(b"=", 1)[1], 16) < 1:
            raise ValueError("fixture CRL is not numbered")
        crl = self.crypto("crl", "-in", "crl.pem", "-outform", "DER").hex()
        dates = dict(line.decode().split("=", 1) for line in self.crypto(
            "crl", "-in", "crl.pem", "-lastupdate", "-nextupdate", "-noout"
        ).splitlines())
        response = self.sql("""
BEGIN;
INSERT INTO sequent_backend.certificate_authority
    (tenant_id,election_event_id,common_name,subject,issuer_common_name,issuer,
     not_before,not_after,fingerprint_sha256,serial_number,pem,purpose)
VALUES (:'tenant',:'event',:'common',:'subject',:'common',:'subject',
        :'before',:'after',:'fp',:'serial',:'pem','staff-signatures')
ON CONFLICT ON CONSTRAINT certificate_authority_one_per_purpose DO NOTHING;
WITH changed AS (
INSERT INTO sequent_backend.staff_crl
    (tenant_id,election_event_id,issuer_id,issuer_fingerprint,url,der,this_update,next_update,status,last_ok_at)
SELECT :'tenant',:'event',id,:'fp',:'url',decode(:'crl','hex'),:'crl_before',:'crl_after','ok',clock_timestamp()
  FROM sequent_backend.certificate_authority
 WHERE (tenant_id,election_event_id,fingerprint_sha256,purpose) = (:'tenant',:'event',:'fp','staff-signatures')
ON CONFLICT (tenant_id,election_event_id,url) DO UPDATE
SET der=EXCLUDED.der,this_update=EXCLUDED.this_update,next_update=EXCLUDED.next_update,
    last_ok_at=EXCLUDED.last_ok_at,fetched_at=clock_timestamp(),status='ok'
WHERE staff_crl.issuer_id=EXCLUDED.issuer_id AND staff_crl.this_update <= EXCLUDED.this_update
RETURNING 1)
SELECT EXISTS (SELECT FROM changed) AS installed
\\gset
\\if :installed
COMMIT;
SELECT 'installed';
\\else
ROLLBACK;
\\endif
""", common=CA_NAME, subject=f"CN={CA_NAME}", url=URL, crl=crl,
            crl_before=asn1date(dates["lastUpdate"]), crl_after=asn1date(dates["nextUpdate"]),
            **self.certificate_info("ca.pem"))
        if response.strip() != "installed":
            raise ValueError("CA or CRL did not match the installed fixture issuer")

    def allow(self):
        values = dict(officer=self.args.officer, first=self.args.officials[0], second=self.args.officials[1])
        if len(set(values.values())) != 3:
            raise ValueError("the separate officer and two approvers must be distinct")
        if self.sql("""
INSERT INTO sequent_backend.operational_initialize_setup_roster
VALUES (:'tenant',:'event',:'first',:'officer',clock_timestamp()+interval '8 hours'),
       (:'tenant',:'event',:'second',:'officer',clock_timestamp()+interval '8 hours')
RETURNING 'authorized';
""", **values).split() != ["authorized", "authorized"]:
            raise ValueError("the pair was not preauthorized")

    def issue(self, official=None):
        official = official or self.args.official
        values = dict(officer=self.args.officer, official=official)
        data = self.sql("""
SELECT json_build_object('attempt', s.attempt, 'post', s.post_id, 'pin', p.expected_spki,
    'enrollment', p.enrollment_id,
    'csr', sequent_backend.operational_setup_public_csr(s.tenant_id,s.event_id,s.official,a.officer,s.attempt))
FROM sequent_backend.operational_initialize_self_setup s
JOIN sequent_backend.operational_initialize_setup_roster a USING (tenant_id,event_id,official)
JOIN sequent_backend.signing_request r
  ON (r.tenant_id,r.election_event_id,r.id,r.election_id,r.payload_sha256) =
     (s.tenant_id,s.event_id,s.request_id,s.post_id,s.payload_sha256)
CROSS JOIN LATERAL sequent_backend.operational_setup_pin_for_attachment(
    s.tenant_id,s.event_id,s.official,a.officer,s.attempt) p
WHERE (s.tenant_id,s.event_id,s.official,a.officer) = (:'tenant',:'event',:'official',:'officer')
  AND a.eligible_until > clock_timestamp() AND r.action='initialize-voting'
  AND r.status='waiting' AND r.expires_at > clock_timestamp()
  AND encode(sha256(convert_to(r.canonical_payload,'UTF8')),'hex') = s.payload_sha256;
""", **values)
        if not data.strip():
            raise ValueError("no current exact officer-preauthorized device CSR to sign")
        material = json.loads(data)
        csr_name, leaf_name = material["attempt"] + ".csr", material["attempt"] + ".pem"
        parse_uuid(material["attempt"])
        (self.work / csr_name).write_text(material["csr"])
        proof = self.crypto("req", "-in", csr_name, "-verify", "-noout", include_stderr=True)
        if b"verify OK" not in proof:
            raise ValueError("CSR proof of possession was not confirmed")
        subject = self.crypto("req", "-in", csr_name, "-noout", "-subject", "-nameopt", "RFC2253").decode().strip()
        if subject != f"subject=OU={self.args.tenant},CN={official}":
            raise ValueError("CSR identity does not match the official/tenant")
        key_pem = self.crypto("req", "-in", csr_name, "-pubkey", "-noout")
        key_der = self.crypto("pkey", "-pubin", "-outform", "DER", data=key_pem)
        if len(key_der) != 91 or not key_der.startswith(P256_SPKI_PREFIX):
            raise ValueError("CSR key is not the required uncompressed P-256 key")
        spki = sha(key_der)
        if spki != material["pin"]:
            raise ValueError("CSR SPKI does not match the persisted nonexportable device enrollment")
        if not (self.work / leaf_name).is_file():
            self.crypto("ca", "-config", "ca.cnf", "-batch", "-notext", "-in", csr_name, "-out", leaf_name)
        leaf_key = self.crypto("x509", "-in", leaf_name, "-pubkey", "-noout")
        if sha(self.crypto("pkey", "-pubin", "-outform", "DER", data=leaf_key)) != spki:
            raise ValueError("issued certificate SPKI does not match the device enrollment")
        # Existing current signed CRL covers newly issued, unrevoked leaves; do not
        # rotate the revocation snapshot under the other official's in-flight approval.
        self.crypto("verify", "-CAfile", "ca.pem", "-CRLfile", "crl.pem", "-crl_check", leaf_name)
        subject = self.crypto("x509", "-in", leaf_name, "-noout", "-subject", "-nameopt", "compat").decode()
        expected = f"/CN={official}/OU={self.args.tenant}"
        if subject.strip() != f"subject={expected}":
            raise ValueError("issued staff certificate has the wrong subject")
        canonical = f"/2.5.4.3={official}/2.5.4.11={self.args.tenant}".encode()
        response = self.sql("""
BEGIN;
SELECT pg_advisory_xact_lock(hashtextextended('signing-event:' || :'tenant' || ':' || :'event',0));
SELECT set_config('hasura.user', json_build_object('x-hasura-user-id', :'officer',
     'x-hasura-tenant-id', :'tenant', 'x-hasura-role', 'admin')::text, true);
INSERT INTO sequent_backend.staff_certificate
    (tenant_id,election_event_id,user_id,username,election_id,fingerprint_sha256,spki_sha256,
     holder_sha256,serial,subject,issuer,not_before,not_after,pem,registration,registered_by)
SELECT :'tenant',:'event',:'official',:'official',:'post',:'fp',:'spki',:'holder',:'serial',
       :'subject',:'issuer',:'before',:'after',:'pem','security-officer',:'officer'
  FROM sequent_backend.operational_initialize_setup_roster a
  JOIN sequent_backend.operational_initialize_self_setup s USING (tenant_id,event_id,official)
  JOIN sequent_backend.signing_request r
    ON (r.tenant_id,r.election_event_id,r.id,r.election_id,r.payload_sha256) =
       (s.tenant_id,s.event_id,s.request_id,s.post_id,s.payload_sha256)
  CROSS JOIN LATERAL sequent_backend.operational_setup_pin_for_attachment(
      s.tenant_id,s.event_id,s.official,a.officer,s.attempt) p
 WHERE (s.tenant_id,s.event_id,s.official,a.officer,s.attempt,s.post_id)
       = (:'tenant',:'event',:'official',:'officer',:'attempt',:'post')
   AND p.enrollment_id=:'enrollment' AND p.expected_spki=:'spki'
   AND a.eligible_until > clock_timestamp() AND r.status='waiting'
   AND r.action='initialize-voting' AND r.expires_at > clock_timestamp()
   AND encode(sha256(convert_to(r.canonical_payload,'UTF8')),'hex') = s.payload_sha256
 RETURNING id AS certificate
\\gset
SELECT sequent_backend.operational_setup_attach_certificate(
    :'tenant',:'event',:'official',:'officer',:'attempt',:'enrollment',
    :'certificate',:'fp',ARRAY[:'pem']) AS attached
\\gset
\\if :attached
COMMIT;
SELECT 'attached';
\\else
ROLLBACK;
\\endif
""", **values, attempt=material["attempt"], post=material["post"], spki=spki,
            enrollment=material["enrollment"], holder=sha(canonical), subject=expected,
            issuer=f"CN={CA_NAME}", **self.certificate_info(leaf_name))
        if "attached" not in response.splitlines():
            raise ValueError("officer did not attach certificate to the exact fixture enrollment")


    def serve_pair(self):
        """One bounded operator run for two already authorized synthetic officials.

        Polls only public enrollment state. Each issuance uses the same separate
        CA/roster/CSR checks as an explicit operator command; no signer, role or
        quorum write is reachable here. Never retry an ambiguous attachment.
        """
        if self.args.backend != "softhsm-test":
            raise ValueError("automatic fixture issuance is for the disposable software token only")
        officer = self.args.officer
        outstanding = set(self.args.officials)
        if len(outstanding | {officer}) != 3 or len(outstanding) != 2:
            raise ValueError("the two named officials and their separately designated officer must differ")
        deadline = time.monotonic() + self.args.timeout
        while outstanding:
            for official in sorted(outstanding):
                phase = self.sql("""
SELECT coalesce(g.step::text,'awaiting-first-use')
  FROM sequent_backend.operational_initialize_setup_roster a
  LEFT JOIN sequent_backend.operational_initialize_self_setup s USING (tenant_id,event_id,official)
  LEFT JOIN sequent_backend.operational_setup_generation g
    ON (a.tenant_id,a.event_id,a.official,a.officer,s.attempt) =
       (g.tenant_id,g.event_id,g.official,g.prepared_by,g.attempt)
 WHERE (a.tenant_id,a.event_id,a.official,a.officer) = (:'tenant',:'event',:'official',:'officer')
   AND a.eligible_until > clock_timestamp();
""", officer=officer, official=official).strip()
                if phase == 'completed':
                    # issue() rechecks the pending canonical request, generated key,
                    # CA and live roster transactionally; a failure halts the supervisor.
                    self.issue(official)
                    outstanding.remove(official)
                elif phase == 'attached':
                    outstanding.remove(official)
                elif phase not in ('awaiting-first-use','prepared','claimed'):
                    raise ValueError("named official is not eligible for this exact fixture")
            if outstanding:
                if time.monotonic() >= deadline:
                    raise ValueError("timed out waiting for named synthetic officials; no enrollment was retried")
                time.sleep(min(2, max(0, deadline-time.monotonic())))


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--state", type=Path, required=True)
    parser.add_argument("--socket", type=Path, required=True)
    parser.add_argument("--port", type=int, default=5432)
    for arg in ("run", "tenant", "event"):
        parser.add_argument("--" + arg, type=parse_uuid, required=True)
    parser.add_argument("--backend", choices=("softhsm-test", "yubihsm"), required=True)
    actions = parser.add_subparsers(dest="action", required=True)
    actions.add_parser("create-ca")
    actions.add_parser("publish-crl")
    allow = actions.add_parser("preauthorize")
    allow.add_argument("officer", type=parse_uuid)
    allow.add_argument("officials", type=parse_uuid, nargs=2)
    issue = actions.add_parser("issue-and-attach")
    issue.add_argument("officer", type=parse_uuid)
    issue.add_argument("official", type=parse_uuid)
    pair = actions.add_parser("serve-preauthorized-pair", help="bounded disposable-SoftHSM fixture CA for two already preauthorized subjects")
    pair.add_argument("officer", type=parse_uuid)
    pair.add_argument("officials", type=parse_uuid, nargs=2)
    pair.add_argument("--timeout", type=int, default=900)
    args = parser.parse_args()
    if args.action == "serve-preauthorized-pair" and not 1 <= args.timeout <= 3600:
        parser.error("timeout must be in 1..3600 seconds")
    try:
        fixture = Fixture(args)
        # OpenSSL ca uses mutable per-authority serial and index files. Keep a
        # single writer even while the two-official supervisor is waiting.
        lock = os.open(fixture.work, os.O_RDONLY | os.O_DIRECTORY)
        try:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as error:
                raise ValueError("the fixture authority already has an active operator") from error
            {"create-ca": fixture.initialize_ca, "publish-crl": fixture.publish_crl,
             "preauthorize": fixture.allow, "issue-and-attach": fixture.issue,
             "serve-preauthorized-pair": fixture.serve_pair}[args.action]()
        finally:
            os.close(lock)
    except (ValueError, OSError, KeyError) as failure:
        print(f"Fixture authority refused: {failure}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
