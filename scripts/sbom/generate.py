#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Apply user-confirmed delivery exclusions; keep full source evidence separately."""
import os,importlib.util,json,re,hashlib,zipfile,collections
from pathlib import Path
from inventory_rules import evidence_review_status, write_platform_dependencies
OLD=Path(__file__).resolve().parent
NEW=Path(os.environ['SBOM_OUTPUT_DIR'])
OUT=NEW/'Submission-Working-Drafts';OUT.mkdir(parents=True,exist_ok=True)
INTERNAL=NEW/'Internal-Review';INTERNAL.mkdir(exist_ok=True)
spec=importlib.util.spec_from_file_location('sbom_builder',OLD/'build_documents.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
m.BASE=OUT
ledger=[]
# These are not offered runtime/deployed tools. Mixed lockfiles remain candidates,
# so shared dependencies are not blindly removed with a development consumer.
PREFIXES=('.github/','docs/','scripts/e2e/','scripts/tasks/','scripts/dev/','packages/e2e/','packages/ui-e2e/','packages/ui-test-kit/','packages/load-tool/','packages/loadtesting/','packages/delivery-orchestrator/','packages/ECIESEncryption/','packages/mock_server/','packages/election-architect/gateway/tests/')
TOKENS=('/browserstack/','/fuzz/','/tests/','/test/')
BAD_SERVICES={'Google reCAPTCHA','GitHub / package and image registries','BrowserStack','OpenAI / AI design and delivery tooling'}
BAD_NAMES={'eciesencryption','ecies-tool','webdriverio-browserstack','openai','anthropic','@anthropic-ai/sdk','e2e','mock_server','election-architect-gateway-tests'}
def excluded_path(p):
 p=p.lstrip('/').split(':',1)[0]
 return p.startswith(PREFIXES) or any(t in '/'+p for t in TOKENS)
def rejected_name(name):
 n=name.lower()
 return n in BAD_NAMES or any(t in n for t in ('browserstack','recaptcha','chatgpt'))
def filter_rows(repo,rows):
 keep=[]
 for row in rows:
  paths=row.get('Evidence paths','').split('; ')
  why=None
  if row['Type'] in ('github-action','github-action-workflow'):why='CI acquisition/build service; not supplied as OVCS runtime'
  elif rejected_name(row['Component']):why='Explicit user-confirmed unused component or optional development/testing tool'
  elif paths and all(excluded_path(p) for p in paths):why='Evidence occurs only in excluded development/test/documentation paths'
  if why:
   ledger.append(dict(repository=repo,record=row['ID'],component=row['Component'],reason=why,source_paths=row.get('Evidence paths','')))
  else:
   row=dict(row)
   paths=[p for p in paths if not excluded_path(p)]
   if paths:row['Evidence paths']='; '.join(paths)
   row['Scope']='Offered-delivery candidate; confirm against frozen release artifacts'
   keep.append(row)
 return keep
old_mods=m.scan_modules
def mods(repo):
 result=[]
 for x in old_mods(repo):
  if rejected_name(x['name']) or excluded_path(x['path']):continue
  result.append(x)
 return result
m.scan_modules=mods
old_rows=m.make_rows
def rows(repo,mods):
 rs,raw,maven,ids=old_rows(repo,mods);rs=filter_rows(repo,rs);allowed={r['ID'] for r in rs}
 return rs,raw,maven,{k:v for k,v in ids.items() if v in allowed}
m.make_rows=rows
old_assets=m.add_assets
def assets(repo,rs):old_assets(repo,rs);rs[:]=filter_rows(repo,rs);return rs
m.add_assets=assets
old_extras=m.extras
m.extras=lambda repo:[x for x in old_extras(repo) if not excluded_path(x['Evidence'])]
old_infra=m.add_infra
def infra(repo,rs,refs):old_infra(repo,rs,refs);rs[:]=filter_rows(repo,rs)
# Deployment references are a separate inventory, never fictional software packages.
m.add_infra=lambda repo,rs,refs: None
old_services=m.services
def services(repo):
 rs=[]
 for s in old_services(repo):
  if s['Component / service'] in BAD_SERVICES:
   ledger.append(dict(repository=repo,record=s['ID'],component=s['Component / service'],reason='Explicit user-confirmed service exclusion',source_paths=s['Evidence']));continue
  if s['Component / service']=='Scanovate biometric services':
   s.update({'Owner':'Scanovate Ltd., Israeli company no. 514423029','License / subscription term':'Supplied cooperation agreement is an unsigned draft. Section 4.8 grants a Project-only unrestricted, perpetual, non-exclusive license with integration/customization/modification/audit and COMELEC/JV rights; sections 3.1, 8.4, 7.4 and 12.2 qualify timing, survival, access and transfer. Obtain signed clarified terms before relying on this grant.','Usage limits':'Draft section 6.1.1: 600,000 Transactions; goodwill allowance through 660,000; above 660,000 charged at USD 0.45 per Transaction. Section 6.2 excludes processes terminated by Scanovate-system failure; counting of retries, other failed attempts and demo/test use requires clarification.','Renewal costs':'No separate periodic renewal charge stated in supplied draft for the perpetual Project license. Overage/change-of-scope charges apply; no inference of free ongoing support.','Exit costs':'No fixed exit charge stated; accrued consideration remains payable. Sections 8.4 and 9.2 restrict continued use/deletion/replacement; explicit continuity and transition permission needed.','Transferability':'Section 4.8 permits granting required COMELEC/JV rights; section 12.2 otherwise requires Scanovate consent to assignment. Obtain express advance permission covering intended parties and continuing COMELEC rights.','Account / tenant owner':'On-premises installation on COMELEC-designated servers (4.6). Activation/license-admin control, vendor access and any remote management accounts not specified; confirm COMELEC control as applicable.','Data location':'On-premises on COMELEC-designated servers. Confirm Philippines-only processing, logs, backups and vendor support/telemetry flows; no unrestricted remote transfer inferred.','Audit / source access':'Section 4.8 grants audit/customization/modification rights; 7.4 restricts deriving source, architecture and technology. Define permitted COMELEC/authorized-auditor access and practical audit/customization method. No source escrow agreement supplied.','Update obligations':'Sections 4.7-4.9 promise adaptation/capability evidence and Tier 3 support. Exhibit B has no response/resolution targets or explicit support coverage; agree patching and contract/warranty support terms.','Replacement / escrow arrangement':'6.3 permits alternative provider if pricing/scope adjustment cannot be agreed; 7.4 and 9.2 restrict replacement-related activity. Harmonize exceptions for nonperformance, continuity and lawful transition. No executed escrow arrangement supplied.','Review status':'Needs confirmation','Comment':'Draft reviewed, not executed evidence. Bid-use authorization and award-stage grant must be clear. Commercial base/implementation prices are kept in the internal review; final bid costing belongs in its appropriate submission component.','Evidence':'261025 Scanovate-Sequent COMELEC Tender Agreement (OB), sections 3.1, 4.4-4.10, 6-9, 12.2; Exhibits A-B'})
  rs.append(s)
 return rs
m.services=services

def short_front(repo,rs,mods,refs,svc,stats):
 p=m.P;t=m.tbl
 story=[p(repo.upper()+' / OVCS 2028','SubX'),p('SBOM and third-party\nrights schedule','TitleX'),p('Scoped working draft • 29 October 2026','SubX'),p('Repository: sequentech/'+repo+' • ovcs '+m.SHAS[repo],'SmallX')]
 if repo=='beyond':story.append(p('Separate supporting package — include the relevant modules only if used in the offered solution.','SubX'))
 story += [p('Declared scope: software and services supplied or relied on for the offered OVCS. Non-delivered development/testing tools and expressly unused components have been excluded. Underlying libraries incorporated into delivered code remain in scope. Shared lockfile entries are delivery candidates, not proof that each package is deployed; reconcile against the final release image/OS inventories before signing.'),p('1. Required documents','HeadX'),t([['Requirement','Submission'],['Ownership / sufficient licenses','TOR 13.3; Bulletin 3 checklist item 9. Technology-owner certification and evidence covering included modules, customization, auditability, modularity and applicable requirements.'],['COMELEC rights undertaking','Bulletin 3 checklist item 10. Signed bidder commitment to provide the required perpetual/non-exclusive rights, access and interoperability if awarded.'],['SBOM / third-party rights schedule','TOR 13.6.2; Bulletin 3 checklist item 17. This schedule and its component appendix/workbook/SPDX attachments.']],[140,385]),p('2. Ownership and release checks','HeadX'),p('Sequent Tech Inc owns its original Sequent-authored code; third-party code retains its owners and applicable licenses. Use the exact contracting/owning entity consistently. Trellis ownership has been confirmed by Sequent and its AGPL-3.0-only metadata aligned in meta#14601. Beyond contains both proprietary and AGPL files.'),p('The actual Step/Beyond revision pair, deployed images and OS/Nix closure must be recorded. This inventory does not assert a complete built-artifact SBOM. '+m.revision_pair(repo)),p('3. How to read the schedule','HeadX'),p('The complete component appendix identifies component, version, purpose, owner evidence, license declaration and rights profile. Profiles below supply the remaining TOR fields and are incorporated by reference. Every field is expanded per component in the workbook and CSV. The SPDX document is the scoped source-based inventory. Unknown fields remain explicit; a component row is not legal clearance.')]
 for key,profile in m.PROFILES.items():
  story += [p(key+' rights profile','SubX'),t([['TOR field','Applicable terms']]+[[f,profile[f]] for f in m.FIELDS[2:]],[125,400])]
 story += [m.PageBreak(),p('4. Included / candidate service terms','HeadX'),p('Only services remaining in the declared scope are listed. For unconfirmed alternatives, finalize which provider is actually used before submission; remove alternatives only following that decision. Scanovate terms below are from an unsigned draft, not executed license evidence.')]
 for s in svc:
  story += [p(s['Component / service'],'SubX'),t([['Field','Entry']]+[[f,s[f]] for f in m.FIELDS]+[['Review status',s['Review status']],['Evidence',s['Evidence']]],[125,400])]
 story += [p('5. Final approval','HeadX'),p('Before submission: sign the appropriate owner/bidder certificates; obtain the clarified vendor grant; complete included service/account/location terms; reconcile actual runtime artifacts; retain exact source/license notices. Unknown licenses, proprietary clauses and vendor limitations require explicit closure.'),p('Prepared: __________________   Authorized review / date: __________________')]
 return story
m.front=short_front
# Keep raw repository scans and exclusions INTERNAL, not embedded in scoped handoff.
def evidence(repo,dest,rs,mods):
 write_platform_dependencies(dest/'Provided-Platform-Dependencies.csv',platform,repo)
 ev=dest/'evidence';ev.mkdir(exist_ok=True)
 evidence={r['ID']:{k:r.get(k,'') for k in ('Evidence paths','Source / metadata URL','License evidence','Owner evidence')} for r in rs}
 (ev/'Component-Evidence.json').write_text(json.dumps(evidence,indent=2))
 prov=dict(repository='sequentech/'+repo,revision=m.SHAS[repo],branch='ovcs',source_tree=m.subprocess.check_output(['git','-C',str(m.WORK/repo),'rev-parse','HEAD^{tree}'],text=True).strip(),scope='Scoped source candidates; original scans precede metadata-only licensing changes; not a runtime image/OS SBOM',original_unfiltered_evidence_retained_internally=True,scanovate_agreement_status='Unsigned draft; rights subject to identified qualifications')
 (ev/'provenance.json').write_text(json.dumps(prov,indent=2));return prov
m.evidence=evidence

# Exact artifact declarations supersede scanner guesses. Evidence JSONs include
# the original URL, SHA-256 and verbatim notices; no license is invented here.
CORRECTIONS = {
    ('duckdb', '1.5.6'): 'MIT',
    ('orderly-set', '5.5.0'): 'MIT',
    ('pathspec', '1.0.4'): 'MPL-2.0',
    ('pybars3', '0.9.7'): 'LGPL-3.0-only',
    ('python-dateutil', '2.9.0.post0'): 'BSD-3-Clause',
    ('text-unidecode', '1.3'): 'Artistic-1.0 OR GPL-2.0-or-later',
    ('auto_generate_cdp', '0.4.5'): 'GPL-3.0-only',
    ('jakarta.ws.rs-api', '4.0.0'): 'EPL-2.0 OR (GPL-2.0-only WITH Classpath-exception-2.0)',
}
scoped_rows=m.make_rows
platform=[]
def corrected_rows(repo, modules):
    rs,raw,maven,ids=scoped_rows(repo,modules)
    step_modules=old_mods('step')
    result=[]
    for r in rs:
        name=r['Component']; version=r['Version / revision']
        scope=r.get('_metadata',{}).get('pomProperties',{}).get('scope')
        reason=None
        if scope=='test': reason='Maven test scope; not a runtime dependency'
        if name=='@sequent/voting-kiosk-maintenance': reason='Maintenance-only npm manifest; no runtime npm dependencies (package.json)'
        if name=='geronimo-jms_1.1_spec' and version=='1.0':
            reason='Optional dependency of log4j 1.2.17; not directly declared in the selected module. See Evidence/log4j-optional.json.'
        if reason:
            ledger.append(dict(repository=repo,record=r['ID'],component=name,reason=reason,source_paths=r['Evidence paths']))
            continue
        if scope=='provided' and version=='UNKNOWN':
            platform.append(dict(repository=repo,component=name,purl=r['PURL'],reason='Provided by Keycloak; resolve exact version from the delivered Keycloak image SBOM, not an unversioned BOM reference.',evidence=r['Evidence paths']))
            continue
        if repo=='beyond' and r['Rights profile']=='UNRESOLVED':
            own=next((x for x in step_modules if x['name']==name and x['version']==version),None)
            if own:
                r.update({'License declaration':own['license'],'Owner':own['owner'],'Classification':own['classification'],'License evidence':'Separately inventoried Step module '+own['path'],'Owner evidence':'Step manifest/SPDX at '+m.SHAS['step'],'Rights profile':'SEQUENT' if own['classification']=='Sequent-authored module' else 'OPEN'})
        correction=CORRECTIONS.get((name,version))
        if correction:
            ev=json.loads((m.EVIDENCE/'licenses'/(name+'.json')).read_text())
            r.update({'License declaration':correction,'License evidence':ev['url']+'; SHA256 '+ev['sha256'],'Rights profile':'OPEN','Comment':'Exact artifact notice reviewed; retain all bundled third-party notices. License duties still apply.'})
        r.update(m.PROFILES[r['Rights profile']])
        r['Review status']=evidence_review_status(r['Review status'],r['Rights profile'])
        result.append(r)
    allowed={r['ID'] for r in result}
    return result,raw,maven,{k:v for k,v in ids.items() if v in allowed}
m.make_rows=corrected_rows
m.main()
m.write_csv(INTERNAL/'Scope-Exclusions.csv',ledger)
(INTERNAL/'Scope-Exclusions.json').write_text(json.dumps(ledger,indent=2))
