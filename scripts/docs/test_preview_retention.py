# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
from datetime import datetime, timedelta, timezone
from pathlib import Path
import tempfile
import unittest

from preview_retention import apply, open_prs, plan, size
from verify_pages import deployed


class RetentionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / 'site'
        self.root.mkdir()
        self.now = datetime(2026, 10, 20, tzinfo=timezone.utc)
        self.policy = dict(pinned_prs=[2335, 3949], max_unpinned_previews=2,
                           max_age_days=14, max_site_bytes=900_000_000)
        for name in ['docusaurus/main/index.html', 'docusaurus/release-preview/release/10.0/index.html',
                     'CNAME', 'other/output.txt', 'docusaurus/pr-preview/custom/keep.txt']:
            self.write(name)
        self.preview(2335)
        self.preview(3949)

    def write(self, path, data=b'x' * 10):
        p = self.root / path
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(data)

    def preview(self, number, data=b'x' * 10):
        self.write(f'docusaurus/pr-preview/pr-{number}/index.html', data)

    def snapshot(self):
        return {str(p.relative_to(self.root)): p.read_bytes() for p in self.root.rglob('*') if p.is_file()}

    def run_plan(self, prs=None, **kwargs):
        return plan(self.root, self.policy, prs or {}, self.now, **kwargs)

    def test_closed_cleanup_preserves_complete_pins_main_releases_unrelated(self):
        self.write('docusaurus/pr-preview/pr-2335/assets/nested/script.js')
        self.write('docusaurus/pr-preview/pr-3949/.well-known/security.txt')
        before = self.snapshot()
        self.preview(12)
        result = self.run_plan()
        self.assertEqual(result['remove'], [12])
        apply(self.root, result)
        self.assertEqual(self.snapshot(), before)

    def test_count_bound_prefers_most_recent_open_prs(self):
        for n in (10, 11, 12): self.preview(n)
        result = self.run_plan({n: self.now-timedelta(days=12-n) for n in (10,11,12)})
        self.assertEqual(result['remove'], [10])

    def test_expired_open_preview_removed(self):
        self.preview(10)
        self.assertEqual(self.run_plan({10:self.now-timedelta(days=15)})['remove'], [10])

    def test_new_active_preview_counts_towards_limit(self):
        self.preview(10); self.preview(11)
        source = Path(self.temp.name)/'build';source.mkdir();(source/'index.html').write_text('new')
        result = self.run_plan({10:self.now,11:self.now,12:self.now}, source=source,
                               target='docusaurus/pr-preview/pr-12', active_pr=12)
        self.assertEqual(result['remove'], [10])
        apply(self.root,result,source,'docusaurus/pr-preview/pr-12')
        self.assertEqual(size(self.root),result['projected_bytes'])

    def test_size_pressure_evicts_oldest_unpinned(self):
        baseline=size(self.root)
        self.preview(10,b'a'*30);self.preview(11,b'b'*30)
        self.policy['max_site_bytes']=baseline+30
        result=self.run_plan({10:self.now-timedelta(days=1),11:self.now})
        self.assertEqual(result['remove'],[10])

    def test_protected_content_over_budget_fails_before_any_mutation(self):
        self.preview(10);before=self.snapshot()
        self.policy['max_site_bytes']=1
        with self.assertRaisesRegex(ValueError,'protected content'):self.run_plan()
        self.assertEqual(self.snapshot(),before)

    def test_active_publication_cannot_evict_pins_to_fit(self):
        source=Path(self.temp.name)/'build';source.mkdir();(source/'index.html').write_bytes(b'x'*100)
        self.policy['max_site_bytes']=size(self.root)+10
        before=self.snapshot()
        with self.assertRaises(ValueError):
            self.run_plan({12:self.now},source=source,target='docusaurus/pr-preview/pr-12',active_pr=12)
        self.assertEqual(self.snapshot(),before)

    def test_main_publication_replaces_only_main_and_retention_candidates(self):
        source=Path(self.temp.name)/'build';source.mkdir();(source/'index.html').write_bytes(b'main update')
        before=self.snapshot();result=self.run_plan(source=source,target='docusaurus/main')
        apply(self.root,result,source,'docusaurus/main')
        before['docusaurus/main/index.html']=b'main update'
        self.assertEqual(self.snapshot(),before)

    def test_symlink_fails_closed(self):
        (self.root/'docusaurus/pr-preview/pr-10').symlink_to(Path(self.temp.name),target_is_directory=True)
        with self.assertRaisesRegex(ValueError,'Symlinks'):self.run_plan()

    def test_bad_targets_and_closed_active_pr_rejected(self):
        source=Path(self.temp.name)/'build';source.mkdir();(source/'index.html').write_text('x')
        for target in ('../other','docusaurus/main/../../other','docusaurus/pr-preview/pr-12'):
            with self.subTest(target=target),self.assertRaises(ValueError):
                self.run_plan(source=source,target=target,active_pr=12)

    def test_pins_cannot_be_dropped_or_guardrail_raised(self):
        self.policy['pinned_prs']=[2335]
        with self.assertRaises(ValueError):self.run_plan()
        self.policy['pinned_prs']=[2335,3949];self.policy['max_site_bytes']=900_000_001
        with self.assertRaises(ValueError):self.run_plan()

    def test_open_pr_metadata_requires_valid_pages(self):
        for bad in ([],{},[{}],[[{'number':1,'state':'closed','updated_at':'2026-10-20T00:00:00Z'}]]):
            with self.subTest(data=bad),self.assertRaises(ValueError):open_prs(bad)
        self.assertEqual(open_prs([[]]),{})
        self.assertEqual(set(open_prs([[dict(number=1,state='open',updated_at='2026-10-20T00:00:00Z')],
                                      [dict(number=2,state='open',updated_at='2026-10-20T00:00:00Z')]])),{1,2})

    def test_concurrent_output_mutation_refuses_commit(self):
        result=self.run_plan();self.write('unseen.txt')
        with self.assertRaisesRegex(ValueError,'changed during cleanup'):apply(self.root,result)


class DeploymentTests(unittest.TestCase):
    def run_record(self, **kwargs):
        return dict(id=1,name='pages build and deployment',head_sha='expected',
                    status='completed',conclusion='success',html_url='https://example.invalid/run',**kwargs)

    def test_build_success_is_not_pages_deployment_success(self):
        r=self.run_record();r['name']='Documentation Preview'
        self.assertFalse(deployed([r],'expected'))

    def test_wrong_commit_or_running_deployment_not_success(self):
        r=self.run_record()
        self.assertFalse(deployed([r],'other'))
        r['status']='in_progress';self.assertFalse(deployed([r],'expected'))

    def test_failed_deployment_errors(self):
        r=self.run_record();r['conclusion']='failure'
        with self.assertRaises(RuntimeError):deployed([r],'expected')

    def test_latest_attempt_must_succeed(self):
        a=self.run_record(run_attempt=1);b=self.run_record(run_attempt=2);b['conclusion']='failure'
        with self.assertRaises(RuntimeError):deployed([a,b],'expected')
        self.assertTrue(deployed([a],'expected'))



class PublicationTests(unittest.TestCase):
    def setUp(self):
        import os
        import subprocess
        self.os, self.subprocess = os, subprocess
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.base=Path(self.temp.name);self.remote=self.base/'remote.git';self.seed=self.base/'seed';self.site=self.base/'site'
        self.source=Path(__file__).resolve().parents[2]
        self.git('init','--bare',str(self.remote))
        self.git('clone',str(self.remote),str(self.seed))
        self.git('-C',str(self.seed),'checkout','-b','doc-previews')
        for number in (2335,3949,12):
            p=self.seed/f'docusaurus/pr-preview/pr-{number}/index.html';p.parent.mkdir(parents=True);p.write_text(f'preview {number}')
        (self.seed/'CNAME').write_text('docs.example.invalid')
        self.commit(self.seed,'initial')
        self.git('-C',str(self.seed),'push','origin','HEAD:doc-previews')
        self.git('clone','--branch','doc-previews',str(self.remote),str(self.site))
        binpath=self.base/'bin';binpath.mkdir();gh=binpath/'gh'
        gh.write_text('#!/bin/sh\nprintf \'[[]]\\n\'\n');gh.chmod(0o755)
        self.env=dict(os.environ,PATH=str(binpath)+os.pathsep+os.environ['PATH'],
                      GITHUB_REPOSITORY='example/test',GITHUB_SHA='test-source',GITHUB_OUTPUT=str(self.base/'output'))

    def git(self,*args):
        return self.subprocess.check_output(['git',*args],stderr=self.subprocess.DEVNULL).decode().strip()

    def commit(self,path,message):
        self.git('-C',str(path),'add','--all')
        self.git('-C',str(path),'-c','user.name=Test','-c','user.email=test@example.invalid','-c','commit.gpgsign=false','commit','-m',message)

    def publish(self):
        return self.subprocess.run(['bash','scripts/docs/publish.sh',str(self.site)],cwd=self.source,
                                   env=self.env,capture_output=True,text=True)

    def test_normal_commit_preserves_history_and_protected_output(self):
        before=self.git('-C',str(self.remote),'rev-parse','doc-previews')
        result=self.publish();self.assertEqual(result.returncode,0,result.stderr)
        after=self.git('-C',str(self.remote),'rev-parse','doc-previews')
        self.assertNotEqual(before,after)
        self.assertEqual(self.git('-C',str(self.remote),'rev-parse',after+'^'),before)
        paths=self.git('-C',str(self.remote),'ls-tree','-r','--name-only',after)
        self.assertNotIn('pr-12/',paths)
        self.assertIn('pr-2335/index.html',paths);self.assertIn('pr-3949/index.html',paths)
        self.assertIn('.pages-deployment.json',paths)

    def test_concurrent_writer_rejects_stale_cleanup_without_force(self):
        (self.seed/'new-output.txt').write_text('unseen output')
        self.commit(self.seed,'concurrent output');self.git('-C',str(self.seed),'push','origin','HEAD:doc-previews')
        expected=self.git('-C',str(self.remote),'rev-parse','doc-previews')
        result=self.publish();self.assertNotEqual(result.returncode,0)
        self.assertEqual(self.git('-C',str(self.remote),'rev-parse','doc-previews'),expected)
        paths=self.git('-C',str(self.remote),'ls-tree','-r','--name-only',expected)
        self.assertIn('new-output.txt',paths);self.assertIn('pr-12/index.html',paths)

if __name__ == '__main__': unittest.main()
