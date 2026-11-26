# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Offline regression for the original-generation selector and Portal wires."""
import re
import sqlite3
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REC = ROOT / "hasura/felix3-local-initialize/1794700002270_original_enrollment"
SQL = (REC / "up.sql").read_text()

class OriginalEnrollment(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        self.db.create_function("now_s", 0, lambda: 1050)
        self.db.create_function("hsm_required", 1, lambda t: t == "tenant1")
        self.db.create_function("payload_matches", 4, lambda bound, subject, canonical, exp:
            bound == "orig" and subject == "publication1" and canonical == "original-canonical" and exp > 0)
        self.db.create_function("octet_length", 1, lambda v: len(v) if v else None)
        self.db.create_function("is_sig_hash", 1, lambda v: bool(v and re.match(r"^[a-f0-9]{64}$",v)))
        self.db.create_function("greatest", -1, max)
        self.db.executescript("""
        CREATE TABLE bindings(installation,tenant,event,post,publication,official,request,payload_sha256);
        CREATE TABLE permits(officer,until_at);
        CREATE TABLE attempts(id,installation,tenant,event,post,publication,official,request,payload_sha256,officer,
            bound,portal_session_sha256,abandoned_at,exchange_at,auth_time,gold_until,released_at,claimed_until);
        CREATE TABLE generations(attempt,tenant_id,event_id,official,prepared_by,device_profile,step,
            fence,evidence_sha256,spki_sha256,public_csr_pem,completed_at,enrollment_id,certificate_id);
        CREATE TABLE enrollments(id,tenant_id,actor,spki,device_profile,key_version,device_evidence_hash,
            nonexportable_confirmed,revoked_at,key_id);
        CREATE TABLE req(id,tenant_id,election_event_id,election_id,payload_sha256,action,scope_key,created_at,
            document_id,document_sha256,expires_at,status,executed_at,completed_at,subject,canonical_payload);
        CREATE TABLE approvals(tenant_id,election_event_id,request_id,user_id);
        INSERT INTO bindings VALUES ('install1','tenant1','event1','post1','publication1','nominee','next','new-hash');
        INSERT INTO permits VALUES ('officer',1200);
        INSERT INTO attempts VALUES ('attempt','install1','tenant1','event1','post1','publication1','nominee','old','old-hash',
             'officer','orig','original-sid',NULL,720,730,780,740,760);
        INSERT INTO req VALUES
           ('old','tenant1','event1','post1','old-hash','initialize-voting','scope1',700,NULL,NULL,1000,'expired',NULL,NULL,
              'publication1','original-canonical'),
           ('next','tenant1','event1','post1','new-hash','initialize-voting','scope1',1050,NULL,NULL,1200,'waiting',NULL,NULL,
              'publication1','new-canonical');
        """)
        spki="a"*64
        self.db.execute("INSERT INTO generations VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            ("attempt","tenant1","event1","nominee","officer","softhsm-test","completed","fence","evidence",
             spki,"public-csr"*8,760,"enrollment",None))
        self.db.execute("INSERT INTO enrollments VALUES (?,?,?,?,?,?,?,?,?,?)",
            ("enrollment","tenant1","nominee",spki,"softhsm-test",1,"evidence",1,None,"official/tenant1/nominee"))
        body = re.search(r"FROM felix3_initialize_setup\.eligible\(b,false\) p(.*?);",SQL,re.S).group(1)
        for old,new in (
            ("felix3_initialize_setup.attempt","attempts"),
            ("sequent_backend.operational_setup_generation","generations"),
            ("sequent_backend.operational_signing_official_enrollment","enrollments"),
            ("sequent_backend.operational_signing_hsm_required","hsm_required"),
            ("sequent_backend.signing_request","req"),
            ("sequent_backend.signing_approval","approvals"),
            ("felix3_initialize_setup.payload_matches","payload_matches"),
            ("SELECT FROM approvals","SELECT 1 FROM approvals"),
            ("(a.bound).","a."),
            ("::text",""),
            ("clock_timestamp()","now_s()"),
            ("date_trunc('second',old.expires_at)","old.expires_at"),
            ("g.spki_sha256 ~ '^[0-9a-f]{64}$'","is_sig_hash(g.spki_sha256)"),
            ("'00000000-0000-0000-0000-000000000000'","'nil'"),
            ("AND isfinite(old.expires_at)",""),
        ):
            body=body.replace(old,new)
        # Current request eligibility below simulates the independent 2200
        # canonical/marker lookup. The shipped resolver must still match both rows.
        self.query = ("SELECT a.id,p.officer,g.enrollment_id,g.spki_sha256 FROM bindings b CROSS JOIN permits p "
          +body+" AND live.status='waiting' AND live.expires_at > now_s() AND live.executed_at IS NULL")
    def result(self):
        return self.db.execute(self.query).fetchall()
    def test_new_waiting_request_preserves_exact_attempt_and_enrollment(self):
        self.assertEqual(self.result(),[("attempt","officer","enrollment","a"*64)])
        self.db.execute("UPDATE req SET status='cancelled' WHERE id='old'")
        self.assertEqual(self.result()[0][:3],("attempt","officer","enrollment"))
    def test_refuses_altered_scope_actor_request_or_hardware(self):
        for change in (
            "UPDATE bindings SET installation='other'",
            "UPDATE bindings SET tenant='other'",
            "UPDATE bindings SET event='other'",
            "UPDATE bindings SET post='other'",
            "UPDATE bindings SET publication='other'",
            "UPDATE bindings SET official='stranger'",
            "UPDATE permits SET officer='different'",
            "UPDATE permits SET until_at=1050",
            "UPDATE attempts SET released_at=NULL",
            "UPDATE attempts SET claimed_until=NULL",
            "UPDATE attempts SET exchange_at=NULL",
            "UPDATE attempts SET auth_time=NULL",
            "UPDATE attempts SET gold_until=730",
            "UPDATE attempts SET portal_session_sha256=NULL",
            "UPDATE attempts SET abandoned_at=900",
            "UPDATE req SET scope_key='new-scope' WHERE id='next'",
            "UPDATE req SET status='waiting',expires_at=1150 WHERE id='old'",
            "UPDATE req SET status='expired' WHERE id='next'",
            "UPDATE req SET expires_at=1050 WHERE id='next'",
            "UPDATE req SET executed_at=1001 WHERE id='old'",
            "UPDATE req SET completed_at=1001 WHERE id='old'",
            "UPDATE req SET created_at=699 WHERE id='next'",
            "UPDATE req SET payload_sha256='changed' WHERE id='old'",
            "UPDATE req SET canonical_payload='changed' WHERE id='old'",
            "UPDATE generations SET evidence_sha256='not-original'",
            "UPDATE generations SET step='claimed'",
            "UPDATE generations SET completed_at=700",
            "UPDATE enrollments SET key_version=2",
            "UPDATE enrollments SET revoked_at=1010",
            "INSERT INTO approvals VALUES ('tenant1','event1','old','nominee')",
            "INSERT INTO approvals VALUES ('tenant1','event1','next','nominee')",
        ):
            with self.subTest(change=change):
                self.db.execute("SAVEPOINT trial")
                self.db.execute(change)
                self.assertEqual(self.result(),[])
                self.db.execute("ROLLBACK TO trial")
                self.db.execute("RELEASE trial")
    def test_exact_original_request_is_not_rebound(self):
        self.db.execute("UPDATE bindings SET request='old',payload_sha256='old-hash'")
        self.db.execute("UPDATE req SET status='waiting',expires_at=1200 WHERE id='old'")
        self.assertEqual(self.result()[0][:3],("attempt","officer","enrollment"))
    def test_read_only_selector_and_no_setup_side_effect(self):
        source = SQL.split("CREATE OR REPLACE FUNCTION ",1)[0]
        for token in ("operational_setup_complete_generation", "operational_signing_enroll_official",
                      "take_delivery", "confirm_gold", "bind_browser", "begin_setup"):
            self.assertNotIn(token, SQL)
        self.assertNotRegex(source, r"\b(?:INSERT|UPDATE|DELETE)\s+(?:INTO|FROM|felix3_)")
        self.assertIn("sequent_backend.operational_setup_pin_for_attachment", SQL)
        self.assertIn("felix3_initialize_setup.eligible(b,true)", SQL)
        self.assertIn("felix3_initialize_attachment.post_access", SQL)

class PortalWire(unittest.TestCase):
    def test_new_status_is_read_only_and_only_click_may_renew_presentation(self):
        routes = ROOT / "packages/harvest/src/routes"
        rust = (routes / "local_officer_bridge.rs").read_text()
        mount = (routes / "local_initialize_authority.rs").read_text()
        ui = (ROOT / "packages/admin-portal/src/components/signing/LocalInitializeEnrollment.tsx").read_text()
        meta = (ROOT / "hasura/felix3-local-initialize/actions.yaml").read_text()
        status = rust.split("pub(crate) async fn nominee_status(",1)[1].split("pub(crate) async fn nominee_resume(",1)[0]
        renew = rust.split("pub(crate) async fn nominee_resume(",1)[1].split("pub(crate) async fn presence(",1)[0]
        presence = rust.split("pub(crate) async fn presence(",1)[1].split("async fn scoped_row(",1)[0]
        for action, endpoint, method in (
            ("operationalInitializeNomineeReviewStatus","nominee-review-status","type: query"),
            ("operationalInitializeNomineeResume","nominee-resume","kind: synchronous"),
        ):
            spec = meta.split("- name: "+action,1)[1].split("- name:",1)[0]
            self.assertIn(method,spec)
            self.assertIn("/attachment/"+endpoint,spec)
            self.assertIn("role: sign-initialize-voting",spec)
        self.assertIn("attachment::nominee_status",mount)
        self.assertIn("attachment::nominee_resume",mount)
        self.assertIn("checked_scope",status)
        self.assertNotIn(".put(",status)
        self.assertIn("checked_scope",renew)
        self.assertIn("recovery_gold",renew)
        self.assertIn(".put(",renew)
        self.assertNotIn("current_nominee",renew)
        self.assertIn("current_nominee",presence)
        self.assertGreaterEqual(presence.count("presentation_is_current"),2)
        self.assertEqual(ui.count("await api.nomineeResume"),1)
        self.assertIn("onClick={() => void resume()}",ui)
        self.assertIn('current !== "resume-ready"',ui)
    def test_only_verified_fresh_gold_can_renew_after_same_scope_resolution(self):
        source = (ROOT / "packages/harvest/src/routes/local_officer_bridge.rs").read_text()
        signed = source.split("fn recovery_gold(",1)[1].split("#[rocket::post",1)[0]
        for check in ("claims.acr == Permissions::GOLD", "after.timestamp() < auth",
                      "now.saturating_sub(60) < auth", "auth <= claims.iat",
                      "claims.iat <= now", "now < claims.exp"):
            self.assertIn(check,signed)
        self.assertIn("bridge_identity()",source)
        self.assertIn("(k.clone(), sid.to_owned())",source)
        self.assertIn("(origin.officer, origin.pins)",source)

if __name__ == "__main__":
    unittest.main()
