#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""Source-only. Exercise the shipped predicates with inert scalar rows."""
import base64
import hashlib
import json
import re
import sqlite3
import unittest
from pathlib import Path

STEP = Path(__file__).resolve().parents[3]
MIGRATIONS = STEP / "hasura/migrations/backend-db"
MIG = MIGRATIONS / "1794700002250_local_initialize_attachment"
SQL = (MIG / "up.sql").read_text()
RUST = (STEP / "packages/windmill/src/services/signing/local_initialize_attachment.rs").read_text()
OLD = (MIGRATIONS / "1794700001200_operational_setup_generation/up.sql").read_text()
LOCAL = (MIGRATIONS / "1794700002200_local_initialize_self_setup/up.sql").read_text()


def body(name):
    return SQL.split("CREATE FUNCTION felix3_initialize_attachment." + name + "(", 1)[1].split("$$;", 1)[0]


def scalar(query):
    """Translate PG spelling only. Leave the joins and predicates intact."""
    return (query.replace("sequent_backend.", "").replace("::text", "")
            .replace("SELECT FROM", "SELECT 1 FROM").replace("clock_timestamp()", ":now")
            .replace("interval '24 hours'", "86400"))


def compact(source):
    return "".join(source.split())


class Predicates(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        self.db.create_function("octet_length", 1, lambda x: None if x is None else len(x))
        self.now = {"now": 100000}

    def prove(self, query, params, changes):
        self.assertEqual(self.db.execute(query, params).fetchall(), [(1,)])
        for change in changes:
            with self.subTest(change=change):
                self.db.execute("SAVEPOINT negative")
                self.db.execute(change)
                self.assertEqual(self.db.execute(query, params).fetchall(), [])
                self.db.execute("ROLLBACK TO negative")
                self.db.execute("RELEASE negative")

    def test_installed_trust_predicate(self):
        self.db.executescript("""
            CREATE TABLE b(tenant,event);
            CREATE TABLE certificate_authority(id,tenant_id,election_event_id,fingerprint_sha256,
                purpose,not_before,not_after,pem);
            CREATE TABLE staff_crl(tenant_id,election_event_id,issuer_id,issuer_fingerprint,status,
                this_update,next_update,last_ok_at,der);
            INSERT INTO b VALUES (1,2);
            INSERT INTO certificate_authority VALUES ('3',1,2,'root','staff-signatures',1,200000,'root pem');
            INSERT INTO staff_crl VALUES (1,2,'3','root','ok',1000,200000,99999,'public crl');
        """)
        source = body("trust").split("FROM sequent_backend.certificate_authority ca", 1)[1].split(";", 1)[0]
        query = scalar("SELECT 1 FROM b CROSS JOIN certificate_authority ca " + source)
        query = (query.replace("(b).", "b.").replace("pins->>'issuer'", ":issuer")
                 .replace("pins->>'root_sha256'", ":root").replace("pins->>'crl_sha256'", ":crl")
                 .replace("encode(sha256(l.der),'hex')", "l.der")
                 .replace("felix3_initialize_attachment.certificate_hash", "certificate_hash")
                 .replace("(other.tableoid,other.ctid) <> (l.tableoid,l.ctid)", "other.rowid <> l.rowid"))
        self.db.create_function("certificate_hash", 1, lambda x: "root" if x == "root pem" else None)
        self.prove(query, {**self.now, "issuer": "3", "root": "root", "crl": "public crl"}, [
            "UPDATE b SET tenant=8", "UPDATE b SET event=8", "UPDATE certificate_authority SET id='8'",
            "UPDATE certificate_authority SET purpose='other'", "UPDATE certificate_authority SET fingerprint_sha256='other'",
            "UPDATE certificate_authority SET pem='other material'",
            "UPDATE certificate_authority SET not_after=100000", "UPDATE certificate_authority SET not_before=100001",
            "UPDATE staff_crl SET issuer_id='8'", "UPDATE staff_crl SET issuer_fingerprint='other'",
            "UPDATE staff_crl SET status='error'", "UPDATE staff_crl SET this_update=100001",
            "UPDATE staff_crl SET next_update=100000", "UPDATE staff_crl SET last_ok_at=13600",
            "UPDATE staff_crl SET last_ok_at=100001", "UPDATE staff_crl SET der='different'",
            "DELETE FROM staff_crl", "INSERT INTO staff_crl SELECT * FROM staff_crl",
        ])

    def test_registry_requires_new_row_in_this_transaction(self):
        self.db.executescript("""
            CREATE TABLE b(tenant,event,post,official,request);
            CREATE TABLE d(officer); CREATE TABLE g(spki);
            CREATE TABLE signing_approval(tenant_id,election_event_id,request_id,user_id);
            CREATE TABLE staff_certificate(tenant_id,election_event_id,election_id,id,user_id,registered_by,
                xmin,status,registration,spki_sha256,fingerprint_sha256,pem,not_before,not_after,holder_sha256);
            INSERT INTO b VALUES (1,2,3,'4',11); INSERT INTO d VALUES ('5'); INSERT INTO g VALUES ('key');
            INSERT INTO staff_certificate VALUES (1,2,3,6,'4','5',77,'active','security-officer','key','leaf',
                'public pem',1,200000,'holder');
        """)
        source = body("finish").split("INTO c FROM sequent_backend.staff_certificate s", 1)[1].split(";", 1)[0]
        query = scalar("SELECT 1 FROM b CROSS JOIN d CROSS JOIN g CROSS JOIN staff_certificate s " + source)
        query = (query.replace("registered,", ":registered,").replace("pg_current_xact_id()::xid", ":xid")
                 .replace("d.pins->>'leaf_sha256'", ":leaf")
                 .replace("felix3_initialize_attachment.certificate_hash(", "certificate_hash("))
        self.db.create_function("certificate_hash", 1, lambda pem: "leaf" if pem == "public pem" else None)
        self.prove(query, {**self.now, "registered": 6, "xid": 77, "leaf": "leaf"}, [
            "UPDATE b SET tenant=8", "UPDATE b SET event=8", "UPDATE b SET post=8", "UPDATE b SET official='8'",
            "UPDATE d SET officer='4'", "UPDATE staff_certificate SET election_id=NULL",
            "UPDATE staff_certificate SET id=8", "UPDATE staff_certificate SET xmin=76",
            "UPDATE staff_certificate SET registration='first-use'", "UPDATE staff_certificate SET status='revoked'",
            "UPDATE staff_certificate SET not_after=100000", "UPDATE staff_certificate SET not_before=100001",
            "UPDATE staff_certificate SET spki_sha256='other'", "UPDATE staff_certificate SET fingerprint_sha256='other'",
            "UPDATE staff_certificate SET pem='swapped'", "INSERT INTO signing_approval VALUES (1,2,11,'4')",
            """INSERT INTO staff_certificate SELECT tenant_id,
                election_event_id,election_id,10,'other',registered_by,xmin,status,registration,spki_sha256,
                fingerprint_sha256,pem,not_before,not_after,holder_sha256 FROM staff_certificate""",
        ])

    def test_transaction_admission(self):
        self.db.executescript("""CREATE TABLE decision(attempt,writer,backend,xid,certificate,until_at);
            INSERT INTO decision VALUES ('original','registry',567,89,NULL,100001);""")
        where = body("finish").split("FROM felix3_initialize_attachment.decision v WHERE ", 1)[1].split(";", 1)[0]
        query = scalar("SELECT 1 FROM decision v WHERE " + where)
        query = (query.replace("= ticket", "= :ticket").replace("session_user", ":writer")
                 .replace("pg_backend_pid()", ":backend").replace("pg_current_xact_id()", ":xid"))
        self.prove(query, {**self.now, "ticket": "original", "writer": "registry", "backend": 567, "xid": 89}, [
            "UPDATE decision SET attempt='another'", "UPDATE decision SET writer='portal'",
            "UPDATE decision SET backend=566", "UPDATE decision SET xid=88", "UPDATE decision SET certificate=6",
            "UPDATE decision SET until_at=100000",
        ])

    def test_pending_inspection_ignores_stale_public_pins_but_never_a_sealed_decision(self):
        self.db.executescript("""
            CREATE TABLE b(tenant,event); CREATE TABLE g(certificate,spki);
            CREATE TABLE decision(attempt,certificate,pins,trust);
            CREATE TABLE staff_certificate(tenant_id,election_event_id,spki_sha256,fingerprint_sha256);
            INSERT INTO b VALUES (1,2); INSERT INTO g VALUES (NULL,'key');
            INSERT INTO decision VALUES ('attempt',NULL,'old leaf and CRL','old observation');
        """)
        pending = body("inspect").split("AND ((g.certificate IS NULL", 1)[1].split(
            "OR EXISTS (SELECT FROM felix3_initialize_attachment.decision d", 1)[0]
        query = scalar("SELECT 1 FROM b CROSS JOIN g WHERE (g.certificate IS NULL" + pending)
        query = (query.replace("felix3_initialize_attachment.decision", "decision")
                 .replace("= ticket", "= :ticket").replace("expected->>'leaf_sha256'", ":leaf"))
        self.prove(query, {"ticket":"attempt","leaf":"new leaf"}, [
            "UPDATE decision SET certificate='sealed'", "UPDATE g SET certificate='sealed'",
            "INSERT INTO staff_certificate VALUES (1,2,'key','other leaf')",
            "INSERT INTO staff_certificate VALUES (1,2,'other key','new leaf')",
        ])

    def test_original_released_completed_generation(self):
        self.db.executescript("""
            CREATE TABLE b(tenant,event,official,request);
            CREATE TABLE attempt(id,bound,officer,abandoned_at,released_at,claimed_until);
            CREATE TABLE operational_setup_generation(attempt,tenant_id,event_id,official,prepared_by,device_profile,
                enrollment_id,spki_sha256,step,fence,evidence_sha256);
            CREATE TABLE operational_signing_official_enrollment(tenant_id,actor,id,spki,device_profile,
                nonexportable_confirmed,revoked_at);
            CREATE TABLE operational_setup_installation(tenant_id,device_profile);
            CREATE TABLE signing_approval(tenant_id,election_event_id,request_id,user_id);
            INSERT INTO b VALUES (1,2,'3',4);
            INSERT INTO attempt VALUES (5,'signed scope','6',NULL,10,20);
            INSERT INTO operational_setup_generation VALUES (5,1,2,'3','6','softhsm-test',7,'key','completed',8,'proof');
            INSERT INTO operational_signing_official_enrollment VALUES (1,'3',7,'key','softhsm-test',1,NULL);
            INSERT INTO operational_setup_installation VALUES (1,'softhsm-test');
        """)
        self.db.create_function("operational_signing_hsm_required", 1, lambda tenant: tenant == 1)
        join = body("generation").split("SELECT s.* INTO g FROM ", 1)[1].split("FOR UPDATE OF s", 1)[0]
        query = scalar("SELECT 1 FROM b CROSS JOIN " + join).replace("felix3_initialize_setup.attempt", "attempt")
        query = query.replace("(b,who)", "(:bound,:who)").replace("(ticket,", "(:ticket,").replace(",who,", ",:who,")
        self.prove(query, {"bound": "signed scope", "who": "6", "ticket": 5}, [
            "UPDATE attempt SET bound='different publication or payload'", "UPDATE attempt SET officer='3'",
            "UPDATE attempt SET claimed_until=NULL", "UPDATE attempt SET released_at=NULL",
            "UPDATE attempt SET abandoned_at=8", "UPDATE operational_setup_generation SET step='claimed'",
            "UPDATE operational_setup_generation SET fence=NULL", "UPDATE operational_setup_generation SET evidence_sha256=NULL",
            "UPDATE operational_setup_generation SET prepared_by='9'", "UPDATE operational_setup_generation SET event_id=9",
            "UPDATE operational_setup_generation SET enrollment_id=9", "UPDATE operational_setup_generation SET spki_sha256='9'",
            "UPDATE operational_setup_installation SET device_profile='yubihsm'",
            "UPDATE operational_signing_official_enrollment SET nonexportable_confirmed=0",
            "UPDATE operational_signing_official_enrollment SET revoked_at=10",
            "INSERT INTO signing_approval VALUES (1,2,4,'3')",
        ])

    def test_both_sessions_must_see_both_labels(self):
        source = body("post_access").split("AND (r.permission_label", 1)[1].split("FOR SHARE", 1)[0]
        query = "SELECT 1 FROM req r CROSS JOIN post p WHERE (r.permission_label" + source
        query = query.replace("= ANY(ol)", "IN ('officer','shared')").replace("= ANY(nl)", "IN ('nominee','shared')")
        self.db.executescript("CREATE TABLE req(permission_label); CREATE TABLE post(permission_label);"
                              "INSERT INTO req VALUES ('shared'); INSERT INTO post VALUES (NULL);")
        self.prove(query, {}, ["UPDATE req SET permission_label='officer'", "UPDATE req SET permission_label='nominee'",
                               "UPDATE post SET permission_label='officer'", "UPDATE post SET permission_label='nominee'"])

    def test_replay_needs_the_officers_original_sealed_receipt(self):
        self.db.executescript("""
            CREATE TABLE decision(attempt,bound,officer,pins,trust,certificate,attached_at);
            CREATE TABLE g(certificate,attached_at,enrollment,fingerprint,spki);
            CREATE TABLE b(tenant,event,post,request,official,payload_sha256);
            CREATE TABLE t(stamp);
            CREATE TABLE staff_certificate(id,tenant_id,election_event_id,election_id,user_id,registered_by,
                status,registration,fingerprint_sha256,spki_sha256,pem,not_before,not_after,holder_sha256);
            CREATE TABLE operational_signing_official_certificate(tenant_id,event_id,certificate_id,actor,
                enrollment_id,certificate_fingerprint,retired_at);
            INSERT INTO decision VALUES (5,'binding','6','pins','trust',7,88);
            INSERT INTO g VALUES (7,88,8,'leaf','key');
            INSERT INTO b VALUES (1,2,3,4,'9','payload');
            INSERT INTO t VALUES ('trust');
            INSERT INTO staff_certificate VALUES (7,1,2,3,'9','6','active','security-officer',
                'leaf','key','public pem',1,200000,'holder');
            INSERT INTO operational_signing_official_certificate VALUES (1,2,7,'9',8,'leaf',NULL);
        """)
        branch = body("inspect").split("OR EXISTS (SELECT FROM felix3_initialize_attachment.decision d", 1)[1]
        branch = branch.split("AND EXISTS (SELECT FROM sequent_backend.operational_initialize_current(", 1)[0]
        self.db.create_function("certificate_hash", 1, lambda pem: "leaf" if pem == "public pem" else None)
        query = scalar("SELECT 1 FROM g CROSS JOIN b CROSS JOIN t CROSS JOIN decision d" + branch)
        query = (query.replace("(ticket,b,who,expected,", "(:ticket,:bound,:who,:pins,")
                 .replace(",who)", ",:who)").replace("expected->>'leaf_sha256'", ":leaf")
                 .replace("felix3_initialize_attachment.certificate_hash", "certificate_hash"))
        self.prove(query, {**self.now, "ticket": 5, "bound": "binding", "who": "6", "pins": "pins", "leaf": "leaf"}, [
            "UPDATE decision SET bound='other'", "UPDATE decision SET officer='9'", "UPDATE decision SET pins='different'",
            "UPDATE decision SET certificate=NULL", "UPDATE g SET attached_at=89", "UPDATE g SET fingerprint='other'",
            "UPDATE t SET stamp='new trust'", "UPDATE g SET enrollment=10",
            "UPDATE operational_signing_official_certificate SET retired_at=9",
            "UPDATE staff_certificate SET status='revoked'", "UPDATE staff_certificate SET pem='different'",
        ])


class AdmissionRecovery(unittest.TestCase):
    """Run the shipped UPSERT predicate with explicit transaction outcomes and public pins."""

    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        self.status = {81: "aborted", 82: "in progress"}
        self.db.create_function("pg_xact_status", 1, self.status.get)
        self.db.execute("""CREATE TABLE decision(attempt PRIMARY KEY,bound,scope,officer,pins,fixed_pins,trust,
            officer_labels,nominee_labels,writer,backend,xid,until_at,certificate,attached_at)""")
        self.bound = dict(installation="i",tenant="t",event="e",post="p",publication="pub",request="r1",
                          official="nominee",payload_sha256="payload1")
        self.pins = dict(enrollment="enrollment",spki_sha256="key",csr_sha256="csr",leaf_sha256="leaf",
                         issuer="issuer",root_sha256="root",crl_sha256="crl1")
        self.old = self.candidate(backend=101,xid=81,until_at=900)
        self.new = self.candidate(backend=102,xid=82,until_at=1100)
        # Strip comments before finding the end of the actual SQL statement.
        admit = re.sub(r"--[^\n]*", "", body("admit"))
        branch = admit.split("ON CONFLICT (attempt)", 1)[1].split(";", 1)[0]
        self.assertIn("pg_xact_status(d.xid)", branch)
        names = list(self.old)
        self.insert = ("INSERT INTO decision AS d (" + ",".join(names) + ") VALUES ("
                       + ",".join(":" + col for col in names) + ") ON CONFLICT (attempt)" + scalar(branch))
        self.assertEqual(self.apply(self.old), 1)

    def candidate(self, bound=None, pins=None, **overrides):
        b = {**self.bound, **(bound or {})}
        p = {**self.pins, **(pins or {})}
        dump = lambda obj: json.dumps(obj, sort_keys=True)
        return dict(attempt="attempt", bound=dump(b), scope=dump({k:v for k,v in b.items()
                    if k not in ("request","payload_sha256")}), officer="officer", pins=dump(p),
                    fixed_pins=dump({k:v for k,v in p.items() if k not in ("leaf_sha256","crl_sha256")}),
                    trust="stamp",officer_labels="ol",nominee_labels="nl",writer="registry",backend=102,
                    xid=82,until_at=1100) | overrides

    def apply(self, row):
        return self.db.execute(self.insert, {**row, "now":1000}).rowcount

    def snapshot(self):
        return self.db.execute("SELECT * FROM decision").fetchone()

    def test_aborted_writer_can_refresh_current_crl_or_genuine_reissue(self):
        for changed in ({"crl_sha256":"crl2"}, {"leaf_sha256":"leaf2","crl_sha256":"crl2"}):
            with self.subTest(changed=changed):
                self.db.execute("SAVEPOINT before_retry")
                next_row = self.candidate(pins=changed,trust="current observation")
                self.assertEqual(self.apply(next_row), 1)
                self.assertEqual(self.db.execute("SELECT pins,trust,xid,bound FROM decision").fetchone(),
                                 (next_row["pins"],next_row["trust"],next_row["xid"],self.old["bound"]))
                self.db.execute("ROLLBACK TO before_retry")
                self.db.execute("RELEASE before_retry")

    def test_expiry_never_steals_live_or_indeterminate_writer(self):
        for state in ("in progress", None):
            self.status[81] = state
            for candidate in (self.new,self.candidate(pins={"crl_sha256":"crl2"})):
                before = self.snapshot()
                with self.subTest(state=state,pins=candidate["pins"]):
                    self.assertEqual(self.apply(candidate), 0)
                    self.assertEqual(self.snapshot(), before)

    def test_terminal_unsealed_request_rebind_does_not_cross_scope_or_key(self):
        b = dict(request="authorized-current-request",payload_sha256="authorized-current-payload")
        for fields in ({"pins":{k:"different"}} for k in
                       ("enrollment","spki_sha256","csr_sha256","issuer","root_sha256")):
            with self.subTest(fields=fields):
                self.assertEqual(self.apply(self.candidate(bound=b,**fields)), 0)
        for key in ("installation","tenant","event","post","publication","official"):
            with self.subTest(key=key):
                self.assertEqual(self.apply(self.candidate(bound={**b,key:"different"})), 0)
        for fields in ({"officer":"nominee"},{"writer":"other registry"},{"xid":81}):
            with self.subTest(fields=fields):
                self.assertEqual(self.apply(self.candidate(bound=b,**fields)), 0)
        self.status[81] = "committed"
        current = self.candidate(bound=b,trust="new observation")
        self.assertEqual(self.apply(current), 1)
        self.assertEqual(self.db.execute("SELECT bound,pins,xid FROM decision").fetchone(),
                         (current["bound"],self.old["pins"],82))

    def test_sealed_decision_is_never_rebound(self):
        self.db.execute("UPDATE decision SET certificate=17,attached_at=800")
        before = self.snapshot()
        for candidate in (self.new,self.candidate(pins={"leaf_sha256":"leaf2"}),
                          self.candidate(bound={"request":"r2","payload_sha256":"payload2"})):
            self.assertEqual(self.apply(candidate), 0)
            self.assertEqual(self.snapshot(), before)

    def test_exact_live_writer_repeats_read_only_without_extending_its_deadline(self):
        self.status[81] = "in progress"
        self.db.execute("UPDATE decision SET until_at=1050")
        # Execute the read-only answer after a conflicting UPSERT affected zero rows.
        tail = body("admit").split("RETURN EXISTS (SELECT FROM felix3_initialize_attachment.decision d",1)[1]
        q = scalar("SELECT EXISTS (SELECT 1 FROM decision d" + tail.split(";",1)[0])
        for name in ("ticket","b","who","expected","checked_stamp","ol","nl","writer_login","writer_pid","tx","cutoff"):
            q = re.sub(r"(?<![.:])\b" + name + r"\b", ":" + name, q)
        proof = dict(ticket="attempt",b=self.old["bound"],who="officer",expected=self.old["pins"],
                     checked_stamp="stamp",ol="ol",nl="nl",writer_login="registry",writer_pid=101,tx=81,cutoff=1090,now=1000)
        before = self.snapshot()
        self.assertEqual(self.apply(self.old), 0)
        self.assertEqual(self.db.execute(q,proof).fetchone(), (1,))
        for change in ({"ticket":"other"},{"b":self.candidate(bound={"request":"r2"})["bound"]},{"who":"nominee"},
                       {"expected":self.candidate(pins={"crl_sha256":"crl2"})["pins"]},{"checked_stamp":"other"},
                       {"ol":"other"},{"nl":"other"},{"writer_login":"other"},{"writer_pid":102},{"tx":82},
                       {"cutoff":1049},{"now":1050}):
            with self.subTest(change=change):
                self.assertEqual(self.db.execute(q,{**proof,**change}).fetchone(), (0,))
        self.assertEqual(self.snapshot(), before)


class Contracts(unittest.TestCase):
    def test_exact_pem_envelope(self):
        expression = body("certificate_hash").split("substring(pem FROM E'", 1)[1].split("')", 1)[0]
        envelope = re.compile(expression.replace(chr(92) * 2, chr(92)))
        public = b"synthetic scalar input"
        wrapped = "-----BEGIN CERTIFICATE-----\n" + base64.b64encode(public).decode() + "\n-----END CERTIFICATE-----"
        for pem in (wrapped, wrapped + "\n", wrapped.replace("\n", "\r\n") + "\r\n"):
            match = envelope.fullmatch(pem)
            self.assertIsNotNone(match)
            self.assertEqual(hashlib.sha256(base64.b64decode(match[1])).digest(), hashlib.sha256(public).digest())
        for pem in (wrapped + "\n" + wrapped, wrapped + "\nextra", "extra\n" + wrapped,
                    wrapped.replace("BEGIN CERTIFICATE", "BEGIN CERTIFICATE REQUEST")):
            self.assertIsNone(envelope.fullmatch(pem))

    def test_original_scope_and_primitives(self):
        fn = body("generation")
        for text in ("(a.id,a.bound,a.officer) = (ticket,b,who)", "a.released_at IS NOT NULL", "a.claimed_until IS NOT NULL",
                     "felix3_initialize_setup.eligible(b,true)", "permit.officer <> who OR who = b.official",
                     "(ticket,b.tenant,b.event,b.official,who,'softhsm-test')", "e.nonexportable_confirmed AND e.revoked_at IS NULL",
                     "IS DISTINCT FROM (g.public_csr_pem,g.enrollment_id,g.spki_sha256)", "signing_approval v"):
            self.assertIn(text, fn)
        for text in ("operational_signing_local_initialize_installation_id(", "r.action = 'initialize-voting'",
                     "felix3_initialize_setup.payload_matches(b,r.subject,r.canonical_payload,r.expires_at)"):
            self.assertIn(text, LOCAL)
        for suffix in ("public_csr", "pin_for_attachment", "attach_certificate"):
            name = "sequent_backend.operational_setup_" + suffix
            self.assertIn("CREATE FUNCTION " + name, OLD)
            self.assertEqual(SQL.count(name + "("), 1)

    def test_atomic_registry_order_and_replay(self):
        fn = RUST.split("pub async fn register_and_attach(", 1)[1]
        sequence = ["self.officer(", "attachment.inspect(", "self.check_csr(", "wire.public_key().raw == csr_spki",
                    "key_usage_check(", "verified_path(", "validity_check(", "operational_revocation_until(",
                    "if let Some(certificate)", "return Ok(receipt);", "pg_current_xact_id()",
                    "attachment.admit(", "check.commit().await?;\n        self.officer(",
                    "audit_log::act_for(&tx", "register_staff_certificate(&tx", "attachment.finish(",
                    "settled.context(", "tx.commit().await?", "kick_audit_log_outbox()"]
        offsets = [compact(fn).index(compact(p)) for p in sequence]
        self.assertEqual(offsets, sorted(offsets))
        self.assertIn("election_id: Some(self.bound.post)", fn)
        self.assertIn("linked_to: None", fn)
        self.assertIn("d.certificate,d.attached_at)", body("inspect"))
        self.assertIn("d.certificate IS NULL", body("admit"))
        self.assertIn("pg_xact_status(d.xid) IN ('aborted','committed')", body("admit"))
        self.assertNotRegex(SQL, r"(?i)(INSERT INTO|UPDATE|DELETE FROM) sequent_backend\.staff_certificate")

    def test_recovery_cannot_rewrite_fixed_coordinates_or_skip_current_verification(self):
        grant = SQL.split("GRANT SELECT, INSERT, UPDATE (", 1)[1].split(")", 1)[0].split(",")
        self.assertFalse(set(grant) & {"attempt","scope","fixed_pins","officer","writer"})
        self.assertIn("ON felix3_initialize_attachment.decision TO sequent_os_ledger_api_owner;", SQL)
        for projection in ("scope = (to_jsonb(bound) - ARRAY['request','payload_sha256'])",
                           "fixed_pins = (pins - ARRAY['leaf_sha256','crl_sha256'])"):
            self.assertIn(projection, SQL)
        fn = body("admit")
        for check in ("officer_service() IS NOT TRUE", "registry_writer(writer_login)",
                      "pg_xact_status(tx) IS DISTINCT FROM 'in progress'", "attachment.inspect(",
                      "m.certificate IS NOT NULL OR m.stamp <> checked_stamp", "cutoff <= clock_timestamp()"):
            self.assertLess(fn.index(check), fn.index("INSERT INTO felix3_initialize_attachment.decision"))
        self.assertIn("registry_writer(session_user)", body("finish"))
        self.assertIn("FOR UPDATE OF s FOR SHARE OF a NOWAIT", body("generation"))
        self.assertIn("FOR UPDATE;", body("finish"))

    def test_compilation_authority_and_crypto_boundaries(self):
        for p in ('feature = "felix3-local-initialize"', 'not(feature = "operational-signing-demo-fixture")',
                  'portal_origin == "http://localhost:3002"', 'device_profile == "softhsm-test"',
                  "prepare_initialize_signing(tx", "verify_local_post(open).await?", "Permissions::SIGN_INITIALIZE_VOTING",
                  "Permissions::SIGNING_CERTIFICATES_REGISTER", "Permissions::GOLD", "id != self.bound.official",
                  "request.verify(&public)?", "X9_62_PRIME256V1", "hash(spki) == self.spki_sha256",
                  "hash(&issued.der) == issued.leaf_sha256", "hash(&der) == issued.crl_sha256",
                  "hash(&path[1].to_der()?) == issued.root_sha256", "signed_list.verify(&root_key)?",
                  "signed_list.get_by_serial(leaf.serial_number()), CrlStatus::NotRevoked"):
            self.assertIn(compact(p), compact(RUST))
        self.assertIn("felix3_initialize_setup.principal('felix3_local_attachment_officer')", body("officer_service"))
        self.assertIn("system_user = 'peer:felix3_local_attachment_officer'", body("officer_service"))
        self.assertNotRegex(SQL, r"(?im)^\s*(?:CREATE (?:ROLE|USER)|GRANT EXECUTE ON ALL)")
        self.assertNotRegex(RUST, r"fn main\(|HttpCrlFetcher|require_legacy_signing|generate_claimed_official")
        self.assertIn("IF EXISTS (SELECT FROM felix3_initialize_attachment.decision)", (MIG / "down.sql").read_text())

    def test_every_private_function_has_an_exact_acl_and_rollback_signature(self):
        down = compact((MIG / "down.sql").read_text())
        declarations = re.findall(r"CREATE FUNCTION (felix3_initialize_attachment\.\w+)\(([^)]*)\)", SQL)
        self.assertTrue(declarations)
        for name, args in declarations:
            signature = name + "(" + ",".join(arg.split()[1] for arg in args.split(",") if arg.strip()) + ")"
            with self.subTest(signature=signature):
                self.assertIn(signature, down)
                self.assertTrue(any(signature in compact(stmt) for stmt in
                                    re.findall(r"REVOKE ALL ON FUNCTION\s+(.+?) FROM PUBLIC;", SQL, re.S)))
                self.assertIn("SET search_path = pg_catalog, pg_temp", body(name.split(".")[1]))


if __name__ == "__main__":
    unittest.main()
