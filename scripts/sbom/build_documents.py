#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Build printable OVCS branch SBOM and TOR 13.6.2 schedules from fixed evidence."""
from pathlib import Path
from inventory_rules import gitlink, image_version
import os,subprocess,json,hashlib,csv,re,collections,fnmatch,tomllib,shutil,datetime,xml.etree.ElementTree as ET,base64,zipfile
from urllib.parse import quote,unquote
from reportlab.platypus import SimpleDocTemplate,Paragraph,Spacer,PageBreak,Table,TableStyle,KeepTogether,LongTable
from reportlab.lib import colors
from reportlab.lib.styles import getSampleStyleSheet,ParagraphStyle
from reportlab.lib.enums import TA_LEFT
from reportlab.lib.pagesizes import A4,landscape
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from openpyxl import Workbook
from openpyxl.styles import Font,PatternFill,Alignment
from openpyxl.worksheet.datavalidation import DataValidation
from xml.sax.saxutils import escape
from license_expression import get_spdx_licensing
LICENSING=get_spdx_licensing()
EVIDENCE=Path(os.environ['SBOM_EVIDENCE_DIR'])
BASE=Path(os.environ['SBOM_OUTPUT_DIR'])
WORK=Path(os.environ['SBOM_SOURCE_DIR'])
META=json.load(open(EVIDENCE/'metadata-cache.json'))
SHAS={r:subprocess.check_output(['git','-C',str(WORK/r),'rev-parse','HEAD'],text=True).strip() for r in ('step','beyond')}
PIN={repo:(other,gitlink(WORK/repo,other)) for repo,other in [('step','beyond'),('beyond','step')]}

def revision_pair(repo):
 other,pin=PIN[repo]
 if pin is None:return repo.capitalize()+' has no committed '+other+' gitlink. Separately inventoried '+other+' checkout: '+SHAS[other]+'.'
 relation='matches' if pin==SHAS[other] else 'does not match'
 return repo.capitalize()+' pins '+other+' at '+pin+', which '+relation+' the separately inventoried checkout '+SHAS[other]+'. Reconcile the actual release pair before signing.'

DATE='29 October 2026'
CREATED=datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
PURPOSES={'admin-portal':'Election administration web portal','voting-portal':'Voter ballot selection, casting and verification web portal','ballot-verifier':'Independent ballot verification web portal','results-portal':'Election results publication web portal','@sequentech/ui-core':'Shared frontend components and platform integration','@sequentech/ui-essentials':'Shared foundational frontend UI components','@sequentech/keycloak-ui':'Voter authentication and enrollment interfaces','keycloak-extensions':'Parent build for Sequent identity-management extensions','harvest':'Backend GraphQL action and application service','windmill':'Background election jobs, notifications and reporting tasks','velvet':'Pipeline-based tally subsystem','sequent-core':'Shared election types, cryptographic/business logic and WASM bindings','trellis':'Tamper-evident append-only PostgreSQL logs and Merkle inclusion/consistency proofs','electoral-log':'Electoral log integration library','protocol-board':'Protocol bulletin-board integration','voting-crypto':'Voting cryptography library','trustee':'Trustee cryptographic operations','pgmq-broker':'PostgreSQL message-queue broker','messaging':'Multi-provider email, SMS and messaging channel delivery','messaging-webhooks':'Messaging-provider delivery webhook handling','step-cli':'Command-line platform and election configuration tools','id-capture':'Browser-compatible identity/photo capture and face detection','orare':'Document/report rendering support','doc_renderer':'Document rendering service','sequent-base':'Shared interaction-domain types and presentation helpers','voting-engine':'Reusable voting interaction engine','ivr-core':'Telephone voting core logic','ivr-cli':'Telephone voting command-line interface','ivr-lambda':'Amazon Connect telephone voting Lambda handler','ivr-emulator':'Phone I/O emulator for telephone voting','ivr-emulator-wasm':'Browser WASM interface to telephone voting emulator','architect-service':'Election Architect revisions, approvals and signed configuration packages','@sequentech/election-config-ui':'Shared election configuration validation and download components','@sequentech/election-architect':'Wizard for producing an importable election event','@sequentech/janitor-ui':'In-browser workbook-to-election import bundle preparation','e2e':'End-to-end test harness','@sequentech/ui-e2e':'Browser UI end-to-end tests','@sequentech/ui-test-kit':'Shared browser test utilities','monitoring-renderer':'dbt-charts dashboard rendering service'}
FIELDS=['Purpose','Owner','License / subscription term','Usage limits','Renewal costs','Exit costs','Transferability','Account / tenant owner','Data location','Audit / source access','Update obligations','Replacement / escrow arrangement']
PROFILES={
 'OPEN':dict(zip(FIELDS[2:],[
 'Declared license expression in component row; rights subject to that exact license and notices. No subscription term established by a package entry.',
 'No numeric limit identified from package metadata. Observe the applicable license, notices, attribution, patent and source obligations; commercial features/support may differ.',
 'No license renewal identified for the declared open-source grant. Paid support, hosting and upgrades are separate; confirm any purchased services.',
 'No termination fee identified from package metadata. Migration, data export and replacement engineering are separate costs, not priced by this inventory.',
 'Recipients receive only rights granted under the applicable upstream license. Preserve notices and source obligations; no ownership assignment is implied.',
 'Library/package: no separate tenant account identified. Any hosting or registry account is a separate service; actual deployment owner to be confirmed.',
 'Determined by the deployed application and integrations; source inventory does not establish processing region or external transfers.',
 'Package source/download reference and evidence path in the electronic row; archive the exact source, notices and build inputs for delivery. Binary audit scope requires a built-artifact SBOM.',
 'No upstream update SLA established by package metadata. Sequent must arrange supported versions and vulnerability remediation under the agreed contract; retain notices when updating.',
 'Retain source/build inputs permitted by the license; evaluate a compatible replacement if needed. No executed escrow or transition-cost agreement evidenced in the branch.'])),
 'SEQUENT':dict(zip(FIELDS[2:],[
 'Sequent-authored original code: file/module declaration in row. Step generally AGPL-3.0-only; Beyond has proprietary and AGPL portions. Contractual COMELEC grant must be documented separately.',
 'Subject to the declared license and signed bid/contract. This inventory does not create a seat, election or transaction allowance.',
 'No new pricing inferred. Any maintenance/subscription charges are to be stated in the bid or executed agreement.',
 'Transition and handover costs require the commercial schedule; no exit fee is established by repository code.',
 'Sequent retains its copyright. COMELEC use, access, interoperability and any perpetual/non-exclusive rights must be expressly covered by the agreed grant; third-party rights remain separate.',
 'No separate library tenant. Production administrative accounts must follow the agreed COMELEC ownership/control and handover arrangements; current accounts not verified.',
 'Deployment-specific; select and record the contracted region and locations of backups, logs and external processing.',
 'Exact source revision identified; authorized audit/source access and handover must be documented. A private repository is not itself an escrow agreement.',
 'Sequent support, security patching and maintenance obligations must follow the signed scope; no SLA is inferred from a source header.',
 'Document source/build handover, replacement support and any escrow triggers in the contract; no signed escrow document supplied for this package.'])),
 'UNRESOLVED':dict(zip(FIELDS[2:],[
 'NOT VERIFIED: license or sufficient grant is not established for this exact component/version; resolve before unconditional submission.',
 'Unknown until the license or agreement is obtained; do not assume unlimited use.',
 'Not established; obtain license/subscription and support terms.',
 'Not established; obtain termination, export and transition terms.',
 'Not established; obtain a grant covering intended deployment, redistribution, modifications and required COMELEC rights.',
 'Library: no tenant identified; any associated service account and its controller require confirmation.',
 'Deployment and processing locations require confirmation; source path alone establishes no residency commitment.',
 'Evidence locator in row; secure exact source/license or vendor audit/access rights. Availability of code does not establish permission to use it.',
 'Not established; identify maintenance/security-update responsibility and support life.',
 'Resolve rights or replace before release; executed escrow/replacement arrangements and costs are not evidenced.'])),
 'SERVICE':dict(zip(FIELDS[2:],[
 'Commercial/provider agreement not present in the OVCS source snapshot; obtain the exact SKU, scope, duration and permitted deployments.',
 'Confirm users, transactions, messages, throughput, geographic and environment limits, fair-use terms and overage pricing.',
 'Not established in source; attach agreed subscription, support, renewal and overage prices.',
 'Not established in source; attach termination, egress/export, transition and deletion charges.',
 'Do not assume transferability. Confirm tenant/account transfer or COMELEC-controlled provisioning and continuity rights in writing.',
 'Current and final tenant/account owners not established by the source snapshot. Identify COMELEC vs Sequent vs provider control and handover procedure.',
 'Contracted region, processing countries, subprocessors, backups/logs and support access not established by source; confirm in the provider agreement.',
 'Attach permitted audit reports, audit rights and source/escrow access if applicable; registry access or a binary digest is not source access.',
 'Obtain supported versions, end-of-life, patch/update obligations, security notification terms and service commitments.',
 'Document replacement/export format, migration support, continuity/escrow provisions and triggers; no executed agreement evidenced.']))}

# Exact repository provenance and source notices. Do not package configuration secrets.
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
def clean(x):return re.sub(r'\s+',' ',str(x or '')).strip()
def tags(s,key):return [clean(x).strip('"') for x in re.findall(r'SPDX-'+key+r':\s*([^\r\n]+)',s) if clean(x)]
def file_rights(root,rel):
 f=root/rel;txt=f.read_text(errors='replace')[:16000] if f.exists() and f.stat().st_size<2000000 else ''
 side=Path(str(f)+'.license')
 if side.exists():txt+='\n'+side.read_text(errors='replace')
 lic=tags(txt,'License-Identifier');owners=tags(txt,'FileCopyrightText')
 for a in tomllib.loads((root/'REUSE.toml').read_text()).get('annotations',[]):
  pats=a.get('path',[]);pats=[pats] if isinstance(pats,str) else pats
  if any(fnmatch.fnmatch(rel,p) for p in pats):
   if not lic or a.get('precedence')!='closest':
    v=a.get('SPDX-License-Identifier',[]);lic+=v if isinstance(v,list) else [v]
    v=a.get('SPDX-FileCopyrightText',[]);owners+=v if isinstance(v,list) else [v]
 return list(dict.fromkeys(filter(None,lic))),list(dict.fromkeys(filter(None,owners)))

def scan_modules(repo,root=None,revision=None):
 root=WORK/repo if root is None else root;revision=revision or SHAS[repo];mods=[]
 for f in sorted((root/'packages').rglob('*')):
  if not f.is_file() or f.name not in ('Cargo.toml','package.json','pom.xml','pyproject.toml'):continue
  rel=str(f.relative_to(root));lic,owner=file_rights(root,rel)
  try:
   if f.name=='Cargo.toml':
    d=tomllib.loads(f.read_text()).get('package',{});name=d.get('name');version=d.get('version','source');desc=d.get('description','Rust application/library module');ml=d.get('license');eco='rust-crate'
   elif f.name=='package.json':
    d=json.loads(f.read_text());name=d.get('name');version=d.get('version','source');desc=d.get('description','JavaScript/TypeScript application or workspace');ml=d.get('license');eco='npm'
   elif f.name=='pyproject.toml':
    d=tomllib.loads(f.read_text()).get('project',{});name=d.get('name');version=d.get('version','source');desc=d.get('description','Python application/module');ml=d.get('license');eco='python'
   else:
    t=ET.parse(f).getroot();ns={'m':'http://maven.apache.org/POM/4.0.0'}
    def x(n):return t.findtext('m:'+n,default='',namespaces=ns)
    name=x('artifactId');version=x('version') or 'inherited';desc=x('description') or 'Java/Keycloak extension module';ml=None;eco='java-archive'
   if not name:continue
   if isinstance(ml,str) and ml not in lic:lic.append(ml)
   # Only identify as Sequent-authored where repository copyright/license evidence supports it.
   status='Sequent-authored module' if any('Sequent' in s for s in owner) else 'Local module; verify attribution'
   if not owner and (lic and repo=='step' or str(name).startswith(('@sequentech/','@sequent/'))):owner=['Sequent Tech Inc (original code; confirm file-level exceptions)'];status='Sequent-authored module'
   if any('Free & Fair' in s for s in owner):status='Imported third-party source'
   mods.append(dict(name=name,version=clean(version),purpose=PURPOSES.get(name,clean(desc)),license=' AND '.join(lic) or 'NOASSERTION',owner='; '.join(owner) or 'Not established in module manifest',path=rel,type=eco,classification=status))
  except Exception as e:print('MODULE PARSE',rel,type(e).__name__)
 # Standard-library-only Go CLI is not emitted by Syft because it has no dependencies.
 for f in root.rglob('go.mod'):
  rel=str(f.relative_to(root));lic,owner=file_rights(root,rel)
  mods.append(dict(name='Sequent offline installation CLI',version=revision,purpose='Install, verify and report provenance of the offline OVCS release; Go standard-library-only module',license=' AND '.join(lic) or 'LicenseRef-Proprietary',owner='; '.join(owner) or 'Sequent Tech Inc (see adjacent Go source headers)',path=rel,type='go-module',classification='Sequent-authored module'))
 return mods

def purpose(p,meta):
 return clean(meta.get('description') or {'npm':'JavaScript/TypeScript dependency','rust-crate':'Rust dependency','python':'Python dependency','java-archive':'Java dependency','github-action':'Build/CI action','github-action-workflow':'Build/CI reusable workflow'}.get(p['type'],'Repository dependency'))

def make_rows(repo,mods):
 raw=json.load(open(EVIDENCE/(repo+'.syft.json')));maven=json.load(open(EVIDENCE/(repo+'-maven.syft.json')))
 pkgs=[p for p in raw['artifacts'] if p['type']!='java-archive']+maven['artifacts']; groups={};ids={}
 for p in pkgs:
  meta=p.get('metadata',{});src=meta.get('source','');salt=src if src.startswith('git+') else ''
  # Include groupId for Maven; purl distinguishes coordinates with the same artifactId.
  key=(p.get('purl') or p['type']+':'+p['name']+'@'+p['version'])+'|'+salt
  q=groups.setdefault(key,dict(p,locations=[],syft_ids=[]));q['locations']+=p.get('locations',[]);q['syft_ids'].append(p['id'])
 rows=[]
 for i,(key,p) in enumerate(sorted(groups.items(),key=lambda kv:(kv[1]['type'],kv[1]['name'].lower(),kv[1]['version'])),1):
  u=p.get('purl','');meta=META.get(u or p['type']+':'+p['name']+'@'+p['version'],{});locs=sorted(set(x['path'].lstrip('/') for x in p['locations']));source=p.get('metadata',{}).get('source','')
  if p['type']=='npm' and p.get('metadata',{}).get('resolved','').startswith('file:'):
   rel=p['metadata']['resolved'][5:].split('#')[0].removeprefix('./')
   for candidate in [WORK/repo/'packages'/rel, WORK/repo/rel]:
    if candidate.exists() and candidate.is_file():
     import tarfile
     try:
      with tarfile.open(candidate) as tar:
       manifest=next((x for x in tar.getmembers() if x.name in ('package/package.json','package.json')),None)
       if manifest:
        md=json.load(tar.extractfile(manifest));meta=dict(meta,license=md.get('license') or meta.get('license') or 'NOASSERTION',license_evidence=str(candidate.relative_to(WORK/repo))+'#package.json',declared_authors=clean(md.get('author')),description=md.get('description','Local compiled WASM package'))
     except Exception:pass
  own=next((m for m in mods if m['name']==p['name'] and m['version']==p['version'] and (p['type']!='rust-crate' or not source)),None)
  if p['type']=='java-archive':own=next((m for m in mods if m['type']=='java-archive' and m['name']==p['name'] and m['path'] in locs and m['version'] in (p['version'],'inherited')),own)
  if p['name'].startswith(('./.github/','./head/.github/')):
   path=p['name'][2:].removeprefix('head/');l,o=file_rights(WORK/repo,path if path.endswith(('.yml','.yaml')) else path+'/action.yml')
   own=dict(license=' AND '.join(l) or 'NOASSERTION',owner='; '.join(o) or 'Sequent Tech Inc (local workflow)',classification='Sequent-authored module',purpose='Local build/CI workflow')
  lic=own['license'] if own else ' AND '.join(x.get('spdxExpression') or x.get('value','') for x in p['licenses']) or meta.get('license') or 'NOASSERTION'
  if p['type'] in ('github-action','github-action-workflow') and meta.get('supplemental_review') and meta.get('copyright'):
   copyright_lines=[x.strip() for x in meta['copyright'].split(';') if re.match(r'(?i)^copyright\s*(?:\(c\))?\s*[0-9]',x.strip())]
   if copyright_lines:meta=dict(meta,copyright='; '.join(copyright_lines))
  owner=own['owner'] if own else meta.get('copyright') or ('Upstream rights holders; declared author: '+meta['declared_authors'] if meta.get('declared_authors') else 'Upstream rights holders; exact legal owner not verified')
  classification=own['classification'] if own else 'Third-party dependency'
  profile='SEQUENT' if classification=='Sequent-authored module' else 'OPEN'
  if lic=='NOASSERTION' or 'LicenseRef-Trellis' in lic or any(k in lic.lower() for k in ('unlicensed','see license','proprietary')) and profile!='SEQUENT':profile='UNRESOLVED'
  if own and 'Imported' in classification and 'Trellis' in lic:profile='UNRESOLVED'
  id=repo.upper()+'-D'+str(i).zfill(5)
  scope='Repository-declared; deployment selection to confirm'
  if all(x.startswith('.github/') for x in locs):scope='Build / CI'
  elif all(x.startswith(('docs/','scripts/tasks/')) for x in locs):scope='Documentation / tooling'
  elif all(any(k in x for k in ('/browserstack/','/fuzz/','/ui-e2e/','/e2e/','/load-tool/','/delivery-orchestrator/')) for x in locs):scope='Test / build / operations tooling; confirm bid scope'
  row={'ID':id,'Component':p['name'],'Version / revision':p['version'],'Type':p['type'],'Classification':classification,'Scope':scope,'Purpose':own.get('purpose',purpose(p,meta)) if own else purpose(p,meta),'Owner':clean(owner),'License declaration':lic,'Rights profile':profile,'Evidence paths':'; '.join(locs),'PURL':u,'Source / metadata URL':meta.get('source') or meta.get('metadata_url') or u,'License evidence':('Module manifest/header and REUSE.toml' if own else meta.get('license_evidence','Scanner declaration in source metadata')),'Owner evidence':('Source notices' if own else 'Copyright text from exact-purl GitHub export' if meta.get('copyright') else 'Registry author declaration; not verified legal ownership' if meta.get('declared_authors') else 'Not verified'),'Review status':'Needs confirmation' if profile=='UNRESOLVED' or 'not verified' in owner.lower() else 'Source evidence collected','Comment':meta.get('fetch_error',''),'_syft_ids':p['syft_ids'],'_source':source,'_metadata':p.get('metadata',{})}
  if p['type'] in ('github-action','github-action-workflow') and not p['name'].startswith('.') and not re.fullmatch('[0-9a-f]{40}',p['version']):
   row['Comment']='CI tag/branch ref is not an immutable commit; record its resolved SHA in the actual build provenance.';row['Review status']='Needs confirmation'
  if own and ' AND ' in lic:
   row['Comment']='Multiple module/header license declarations; confirm file-level license scope before release.';row['Review status']='Needs confirmation'
  row.update(PROFILES[profile]);rows.append(row)
  for sid in p['syft_ids']:ids[sid]=id
 # Add local modules not represented by lock catalogers.
 for m in mods:
  if any(r['Component']==m['name'] and (m['path'] in r['Evidence paths'] or r['Version / revision']==m['version']) and r['Classification']!='Third-party dependency' for r in rows):continue
  profile='SEQUENT' if m['classification']=='Sequent-authored module' else 'OPEN' if m['license']!='NOASSERTION' and 'LicenseRef-Trellis' not in m['license'] else 'UNRESOLVED'
  row={'ID':repo.upper()+'-M'+str(len(rows)+1).zfill(5),'Component':m['name'],'Version / revision':m['version'],'Type':m['type'],'Classification':m['classification'],'Scope':'Source module; deployment selection to confirm','Purpose':m['purpose'],'Owner':m['owner'],'License declaration':m['license'],'Rights profile':profile,'Evidence paths':m['path'],'PURL':'','Source / metadata URL':'https://github.com/sequentech/'+repo+'/blob/'+SHAS[repo]+'/'+m['path'],'License evidence':'Module manifest/header, sidecar and REUSE annotations','Owner evidence':'Source declaration','Review status':'Needs confirmation' if profile=='UNRESOLVED' else 'Source evidence collected','Comment':'','_syft_ids':[],'_metadata':{}}
  row.update(PROFILES[profile]);rows.append(row)
 return rows,raw,maven,ids

def add_assets(repo,rows):
 root=WORK/repo;config=tomllib.loads((root/'REUSE.toml').read_text())
 for a in config.get('annotations',[]):
  owner=a.get('SPDX-FileCopyrightText',[]);owner=[owner] if isinstance(owner,str) else owner
  foreign=[o for o in owner if o and 'Sequent' not in o]
  lic=a.get('SPDX-License-Identifier','NOASSERTION')
  if not foreign and not str(lic).startswith('LicenseRef-Trellis'):continue
  paths=a.get('path',[]);paths=[paths] if isinstance(paths,str) else paths
  matched=sorted(set(str(f.relative_to(root)) for pat in paths for f in root.glob(pat) if f.is_file()))
  if not matched:continue
  profile='UNRESOLVED' if 'LicenseRef-Trellis' in lic else 'OPEN'
  label='Trellis upstream import' if profile=='UNRESOLVED' else 'Vendored asset/source: '+paths[0]
  row={'ID':repo.upper()+'-A'+str(len(rows)+1).zfill(5),'Component':label,'Version / revision':'Files at '+SHAS[repo][:12],'Type':'vendored-source/asset','Classification':'Imported third-party source/assets','Scope':'Repository files; deployment selection to confirm','Purpose':'Vendored source, UI/font/model assets identified by REUSE notices','Owner':'; '.join(foreign) or '; '.join(owner),'License declaration':lic,'Rights profile':profile,'Evidence paths':'; '.join(paths),'PURL':'','Source / metadata URL':'https://github.com/sequentech/'+repo+'/blob/'+SHAS[repo]+'/REUSE.toml','License evidence':'REUSE.toml annotation; LICENSES/'+lic+'.txt','Owner evidence':'Explicit REUSE copyright notice','Review status':'Needs confirmation' if profile=='UNRESOLVED' else 'Source evidence collected','Comment':str(len(matched))+' files; complete path/hash list in evidence/vendored-files.csv','_syft_ids':[],'_metadata':{}}
  row.update(PROFILES[profile]);rows.append(row)
 # Preserve file-level third-party notices that are not REUSE annotation groups.
 sidegroups=collections.defaultdict(list)
 for f in root.rglob('*.license'):
  t=f.read_text(errors='replace');owner=tags(t,'FileCopyrightText');lic=tags(t,'License-Identifier')
  if any('Sequent' not in o for o in owner):sidegroups[('; '.join(owner),' AND '.join(lic) or 'NOASSERTION')].append(str(f.relative_to(root)).removesuffix('.license'))
 for (owner,lic),paths in sidegroups.items():
  row={'ID':repo.upper()+'-A'+str(len(rows)+1).zfill(5),'Component':'Vendored material: '+paths[0],'Version / revision':'Files at '+SHAS[repo][:12],'Type':'vendored-source/asset','Classification':'File-level third-party notice','Scope':'Repository files; deployment selection to confirm','Purpose':'Bundled code, map data or documentation identified by file-level copyright notices','Owner':owner,'License declaration':lic,'Rights profile':'OPEN','Evidence paths':'; '.join(paths),'PURL':'','Source / metadata URL':'https://github.com/sequentech/'+repo+'/blob/'+SHAS[repo]+'/'+paths[0]+'.license','License evidence':'SPDX sidecar notices; governing license text retained by source','Owner evidence':'Explicit file copyright notice','Review status':'Source evidence collected','Comment':str(len(paths))+' files; paths retained in electronic schedule','_syft_ids':[],'_metadata':{}}
  row.update(PROFILES['OPEN']);rows.append(row)
 return rows

def extras(repo):
 """Inventory deployment references without treating templates as installed artifacts."""
 root=WORK/repo;out=[]
 for f in sorted(root.rglob('*')):
  if not f.is_file() or f.stat().st_size>2000000:continue
  rel=str(f.relative_to(root))
  if f.name in ('flake.lock','devenv.lock'):
   try:
    for name,node in json.loads(f.read_text()).get('nodes',{}).items():
     v=node.get('locked',{});
     if not v:continue
     out.append(dict(Kind='Nix input',Name=name,Version=v.get('rev',v.get('narHash','unresolved')),Reference=v.get('url') or (('https://github.com/'+v['owner']+'/'+v['repo']) if v.get('owner') and v.get('repo') else str(v.get('type',''))),Evidence=rel,Hash=v.get('narHash',''),Note='Locked input; not a resolved NixOS package closure or license determination'))
   except Exception:pass
  if f.name=='Chart.yaml':
   import yaml
   try:
    d=yaml.safe_load(f.read_text())
    for v in (d or {}).get('dependencies',[]):
     if not v.get('repository','').startswith('file:'):out.append(dict(Kind='Helm dependency',Name=v['name'],Version=str(v.get('version','')),Reference=v.get('repository',''),Evidence=rel,Hash='',Note='Declared chart dependency; chart/image licenses and installed digest require verification'))
   except Exception:pass
  if f.suffix in ('.yml','.yaml','.nix','.sh') or 'Dockerfile' in f.name:
   txt=f.read_text(errors='replace');stages=set()
   for n,l in enumerate(txt.splitlines(),1):
    s=l.strip()
    if s.startswith('#'):continue
    from inventory_rules import image_reference
    ref=image_reference(rel,s)
    if ref:
     previous_stage=ref.lower() in stages
     alias=re.search(r'\sAS\s+(\S+)',s,re.I)
     if alias:stages.add(alias.group(1).lower())
     if ref in ('scratch',) or ref.startswith('${') or previous_stage:continue
     out.append(dict(Kind='Container reference',Name=ref.split('@')[0],Version=image_version(ref),Reference=ref,Evidence=rel+':'+str(n),Hash=ref.split('@',1)[1] if '@' in ref else '',Note='Source reference only; may be a dev/test/example/template. No image filesystem scanned.'))
   if rel=='airgap/prepare.sh':
    for m in re.finditer(r'"([^"\s]+@sha256:[0-9a-f]{64})"',txt):
     ref=m.group(1);out.append(dict(Kind='Pinned image input',Name=ref.split('@')[0],Version='digest pinned',Reference=ref,Evidence=rel+':'+str(txt[:m.start()].count('\n')+1),Hash=ref.split('@')[1],Note='Pinned external binary input; vendor/image SBOM and applicable rights still required.'))
 return out

def add_infra(repo,rows,refs):
 groups={}
 for e in refs:
  key=(e['Kind'],e['Name'],e['Version'],e['Reference'])
  groups.setdefault(key,[]).append(e)
 for (_,name,version,ref),es in sorted(groups.items()):
  row={'ID':repo.upper()+'-I'+str(len(rows)+1).zfill(5),'Component':name,'Version / revision':version,'Type':es[0]['Kind'],'Classification':'Declared infrastructure/build input; rights to verify','Scope':'Source reference; not a scanned or confirmed deployed artifact','Purpose':('Source/build dependency input' if es[0]['Kind']=='Nix input' else 'Chart-driven deployment input' if es[0]['Kind']=='Helm dependency' else 'Container build or deployment reference'),'Owner':'Upstream image/input rights holders; exact owner not verified','License declaration':'NOASSERTION','Rights profile':'UNRESOLVED','Evidence paths':'; '.join(sorted(set(e['Evidence'] for e in es))),'PURL':'','Source / metadata URL':ref,'License evidence':'No component license grant established by the source reference; obtain exact artifact license/SBOM','Owner evidence':'Not verified','Review status':'Needs confirmation','Comment':es[0]['Note']+' Source hash/digest: '+es[0]['Hash'],'_syft_ids':[],'_metadata':{}}
  row.update(PROFILES['UNRESOLVED']);rows.append(row)


def services(repo):
 records=[
 ('Scanovate biometric services','OCR, facial matching, liveness and presentation-attack detection','Scanovate; contracting legal entity to confirm','step: packages/keycloak-extensions/scanovate-authenticator; beyond: airgap/prepare.sh:323-334','Four vendor images pinned by SHA-256 in Beyond; vendor terms, internal SBOM, audit/source rights and license limits not in branch. Byte identity is not a rights grant.'),
 ('AWS SES','Email/OTP delivery integration','Amazon Web Services; contracting entity and tenant to confirm','step: packages/messaging/src/providers/aws.rs; packages/keycloak-extensions/aws-ses-email-sender-provider','Supported integration; confirm actual bid scope, sending quotas, region, per-message and renewal pricing.'),
 ('AWS SNS','SMS/OTP delivery integration','Amazon Web Services; contracting entity and tenant to confirm','step: packages/messaging/src/providers/aws.rs; packages/messaging/Cargo.toml','Supported integration; confirm destination coverage, sender registration, throughput and message/overage charges.'),
 ('Twilio Verify','Optional SMS verification/OTP provider','Twilio; contracting legal entity and tenant to confirm','step: packages/keycloak-extensions/message-otp-authenticator/src/main/java/sequent/keycloak/authenticator/gateway/TwilioVerifySenderProvider.java','Supported provider in code; inclusion, geographic allowances, verification pricing and transferability require confirmation.'),
 ('Infobip','Optional messaging/OTP provider','Infobip; contracting legal entity and tenant to confirm','step: packages/messaging/src/providers/infobip.rs; src/webhooks/infobip.rs','Supported provider; confirm contracted channels, countries, throughput, templates and per-message pricing.'),
 ('Meta WhatsApp','Optional WhatsApp delivery channel','Meta/WhatsApp; contracting entity, business account and any BSP to confirm','step: packages/messaging/src/providers/whatsapp.rs','Supported channel; confirm business/account ownership, template approval, consent scope, country availability and conversation/message fees.'),
 ('Meta Messenger','Optional Messenger delivery channel','Meta; contracting entity and page/business account owner to confirm','step: packages/messaging/src/providers/messenger.rs','Supported channel; final bid inclusion and account/permissions/policy restrictions require confirmation.'),
 ('Amazon Connect','Optional telephone voting contact-center integration','Amazon Web Services; contracting entity and tenant to confirm','beyond: packages/ivr-lambda/README.md; aws/Voting.json','Supported telephone voting integration; confirm phone-number provisioning, countries, concurrent calls, minutes, recording/data region and all service/telecom charges.'),
 ('AWS Lambda','Optional serverless runtime for IVR and document rendering','Amazon Web Services; contracting entity and tenant to confirm','beyond: packages/ivr-lambda/Cargo.toml; step: packages/orare/Cargo.toml','Optional runtime; confirm inclusion, concurrency/time limits, region and invocation/compute pricing.'),
 ('AWS DynamoDB','Optional IVR state persistence','Amazon Web Services; contracting entity and tenant to confirm','beyond: packages/ivr-core/Cargo.toml (optional aws-sdk-dynamodb)','Feature-gated integration; confirm deployment selection, retention, capacity, backups, region and request/storage prices.'),
 ('AWS S3 or selected S3-compatible service','Object storage for application, board and report artifacts','AWS or selected storage operator; not established for final bid','step: packages/sequent-core/Cargo.toml; packages/harvest/Cargo.toml; beyond: packages/ivr-core/Cargo.toml','SDK compatibility does not establish an AWS purchase. Confirm storage provider, account, buckets, regions, retention, request/egress charges and export procedure.'),
 ('AWS Secrets Manager or selected secrets service','Credential/secret storage integration','AWS or selected operator; final service to confirm','step: packages/windmill/Cargo.toml; beyond: packages/ivr-lambda/Cargo.toml','Confirm whether used, tenant access, key custody, region, rotation obligations, request/storage pricing and handover.'),
 ('Google reCAPTCHA','Optional anti-automation checks during authentication/recovery','Google; contracting legal entity and project owner to confirm','step: packages/keycloak-extensions/message-otp-authenticator/src/main/java/sequent/keycloak/authenticator/forgot_password/Utils.java','Source contains integration; confirm enabled scope, product tier, external connectivity/data processing, quotas and account ownership.'),
 ('SMTP relay / alternative message provider','Configurable outbound email service','Provider selected for the bid; not established','step: packages/messaging/Cargo.toml (lettre SMTP transport)','An SMTP implementation is not a contracted relay subscription. Select final provider or record as not applicable with the final deployment.'),
 ('Cloud infrastructure / managed services','Hosting, object storage, databases, secrets and networking where contracted','AWS / Google Cloud options in source; final provider to confirm','beyond: k8s/charts/cluster-setup/values.yaml; infrastructure/; airgap/','Do not treat example cloud configurations or registry mirrors as proof of contracted hosting or data residency. Offline NixOS/K3s demo and cloud production are different scopes.'),
 ('Cloudflare DNS / edge services','DNS and optional proxy/firewall integrations','Cloudflare; final account/SKU to confirm','beyond: k8s/charts/client-setup/values.yaml; k8s/charts/cluster-setup/values.yaml','Conditional integration; confirm whether included in the bid and which services are actually enabled.'),
 ('External identity providers / federation','Optional SAML/OIDC identity federation','Selected identity-provider operator; not established','step: packages/keycloak-extensions/idp-linking-authenticator; .devcontainer/simplesamlphp/','Development/sample IdP is not proof of a production identity-provider contract. Mark not applicable if excluded from the offered deployment.'),
 ('GitHub / package and image registries','Source hosting, CI and release dependency acquisition','GitHub and individual registry operators','step and beyond: .github/; .gitmodules; Dockerfiles and lockfiles','Build/acquisition services, not automatically a runtime dependency. Archive source, notices and offline inputs; confirm commercial-account continuity if relied upon.'),
 ('BrowserStack','Cross-browser testing service','BrowserStack; account owner to confirm','step: packages/sequent-core/browserstack; packages/ballot-verifier/browserstack','Testing/tooling only based on paths; confirm exclusion from delivered runtime and whether any subscription is part of the bid.'),
 ('OpenAI / AI design and delivery tooling','AI processing in development/operations tools where enabled','OpenAI; selected account and contract to confirm','beyond: packages/delivery-orchestrator/ (optional tooling)','Do not assume voter data is sent or that this is included in OVCS runtime. Confirm scope; exclude only with an explicit deployment decision.'),
 ]
 out=[]
 for i,(name,purpose_,owner,evidence,note) in enumerate(records,1):
  row={'ID':repo.upper()+'-S'+str(i).zfill(2),'Component / service':name,'Purpose':purpose_,'Owner':owner,'Scope':'Potential service/integration; bid inclusion to confirm','Evidence':evidence,'Review status':'Needs confirmation','Comment':note};row.update(PROFILES['SERVICE']);out.append(row)
 return out

# PDF typography and layout.
pdfmetrics.registerFont(TTFont('Lato','/usr/share/fonts/truetype/lato/Lato-Regular.ttf'))
pdfmetrics.registerFont(TTFont('LatoBold','/usr/share/fonts/truetype/lato/Lato-Bold.ttf'))
pdfmetrics.registerFontFamily('Lato',normal='Lato',bold='LatoBold',italic='Lato',boldItalic='LatoBold')
NAVY=colors.HexColor('#143344');TEAL=colors.HexColor('#007B80');LIGHT=colors.HexColor('#EEF4F5');GRAY=colors.HexColor('#53646C')
ST=getSampleStyleSheet()
for s in ST.byName.values():s.fontName='Lato'
ST.add(ParagraphStyle(name='BodyX',fontName='Lato',fontSize=10,leading=14,spaceAfter=8,textColor=NAVY,splitLongWords=True))
ST.add(ParagraphStyle(name='SmallX',parent=ST['BodyX'],fontSize=8,leading=10,spaceAfter=4))
ST.add(ParagraphStyle(name='TinyX',parent=ST['BodyX'],fontSize=7,leading=8.7,spaceAfter=0))
ST.add(ParagraphStyle(name='TitleX',fontName='LatoBold',fontSize=27,leading=31,spaceAfter=15,textColor=NAVY))
ST.add(ParagraphStyle(name='HeadX',fontName='LatoBold',fontSize=16,leading=20,spaceBefore=14,spaceAfter=9,textColor=NAVY))
ST.add(ParagraphStyle(name='SubX',fontName='LatoBold',fontSize=11,leading=15,spaceBefore=9,spaceAfter=7,textColor=TEAL))
def P(x,style='BodyX'):
 s=escape(str(x)).replace('\n','<br/>')
 return Paragraph(s,ST[style])
def tbl(rows,widths,small=True):
 style='SmallX' if small else 'BodyX'
 data=[[P(x,style) for x in row] for row in rows]
 data[0]=[Paragraph('<b><font color="#FFFFFF">'+escape(str(x)).replace('\n','<br/>')+'</font></b>',ST[style]) for x in rows[0]]
 t=Table(data,colWidths=widths,repeatRows=1,hAlign='LEFT')
 t.setStyle(TableStyle([('BACKGROUND',(0,0),(-1,0),NAVY),('TEXTCOLOR',(0,0),(-1,0),colors.white),('VALIGN',(0,0),(-1,-1),'TOP'),('BOTTOMPADDING',(0,0),(-1,-1),7),('TOPPADDING',(0,0),(-1,-1),6),('LEFTPADDING',(0,0),(-1,-1),7),('RIGHTPADDING',(0,0),(-1,-1),7),('ROWBACKGROUNDS',(0,1),(-1,-1),[colors.white,LIGHT]),('LINEBELOW',(0,0),(-1,0),.5,TEAL)]))
 for cell in data[0]:cell.style=ParagraphStyle('wh',parent=ST[style],textColor=colors.white,fontName='LatoBold')
 return t

def pdf(path,story,repo,wide=False):
 size=landscape(A4) if wide else A4
 def page(c,d):
  c.setStrokeColor(TEAL);c.setLineWidth(.7);c.line(35,size[1]-31,size[0]-35,size[1]-31)
  c.setFont('Lato',8);c.setFillColor(GRAY)
  c.drawString(35,22,'SEQUENT  /  '+repo.upper()+'  /  OVCS '+SHAS[repo][:12]+'  /  '+DATE)
  c.drawRightString(size[0]-35,22,str(d.page))
 doc=SimpleDocTemplate(str(path),pagesize=size,rightMargin=35,leftMargin=35,topMargin=43,bottomMargin=40,title=path.stem,author='Sequent Tech Inc',pageCompression=1)
 doc.build(story,onFirstPage=page,onLaterPages=page)

def front(repo,rows,mods,extra,svc,stats):
 optional=repo=='beyond';story=[]
 story+=[P('OVCS 2028','SubX'),P(repo.upper()+'\nSoftware Bill of Materials\nand Third-Party Rights Schedule','TitleX'),P('Prepared from the ovcs branch • '+DATE,'SubX')]
 if optional:story+=[P('SEPARATE SUPPORTING PACKAGE — BID INCLUSION TO BE CONFIRMED','SubX')]
 story+=[P('Prepared for authorized review and signature. Source evidence has been populated; unresolved license grants, provider contracts and deployment evidence are listed explicitly. This document does not certify those unresolved matters as complete.')]
 story+=[tbl([['Repository','Fixed source revision'],['sequentech/'+repo,SHAS[repo]],['Branch','ovcs (snapshot; not a moving branch reference)'],['Source archive SHA-256',sha(WORK/(repo+'.tar.gz'))],['Prepared inventory',str(len(rows))+' component records; '+str(stats['scanner_occurrences'])+' scanner occurrences before deduplication and supplemental module/asset rows']], [125,400])]
 story+=[Spacer(1,13),P('Printing and electronic attachments','SubX'),P('Print this document together with 02-Complete-Component-Inventory.pdf. The component appendix incorporates the common rights profiles in this document. The workbook and CSV contain every TOR field expanded for each row; SPDX JSON provides the machine-readable source inventory. Keep all attachments associated with the exact revision above.')]
 story+=[P('Ownership','SubX'),P('Sequent Tech Inc owns the copyright in the original '+repo.capitalize()+' code authored by Sequent. This ownership is distinct from the licenses granted to recipients. Third-party code, imported source, fonts, models and service rights remain with their respective rights holders and are identified separately. The ownership declaration is provided as a separate signature-ready document.')]
 story+=[PageBreak(),P('1. COMELEC requirement and submission boundary','HeadX')]
 quote_='The bidder shall submit a Software Bill of Materials and a complete third-party rights and dependency schedule identifying, for each component or service: purpose, owner, license or subscription term, usage limits, renewal and exit costs, transferability, account/tenant owner, data location, audit or source access, update obligations, and replacement or escrow arrangement.'
 story+=[P(quote_),P('Source: TOR 13.6.2, opening-of-bids requirements; Bid Bulletin No. 3 (Final 28 October 2026), PDF page 23 item (o), and revised checklist PDF page 34 item 17. “Document 22” may be an internal submission label; this package cites the operative clause and latest checklist.','SmallX')]
 story+=[P('This is a repository source inventory, not a claim that every dependency is deployed. Development, CI, documentation and test packages are retained and marked; mixed workspace lockfiles do not by themselves distinguish shipped runtime code. Container references, Helm dependencies and locked Nix inputs are listed separately. Actual image filesystems, the resolved NixOS package closure, provider contracts and deployment regions have not been supplied to this preparation process.')]
 story+=[P('Repository boundary','SubX')]
 if optional:
  story+=[P(revision_pair(repo)+' Beyond contains private modules, infrastructure and installation tools; private repository status alone is not a reason to omit a module or dependency that is actually used in the offered OVCS system. Include this package or its relevant rows if those components are offered.')]
 else:
  story+=[P(revision_pair(repo)+' GitHub source archives do not include submodule contents; this Step scan does not silently include Beyond.')]
 story+=[P('TOR 13.3.1 and 13.3.2.5','SubX'),P('The TOR requires IP ownership or unrestricted perpetual licenses for all OVCS modules, and adequate COMELEC rights such as perpetual/non-exclusive license, access and interoperability commitments. Source availability and package-license declarations are evidence inputs; they do not replace the authorized COMELEC grant or any necessary third-party agreements.')]
 story+=[P('2. Matters to close before unconditional submission','HeadX')]
 gaps=[['Priority / matter','Required closure'],['High — commercial services / Scanovate','Confirm actual bid scope and attach signed terms covering all TOR fields, including limits, price, renewal, exit, tenant control, location, audit/access and replacement/escrow. Obtain vendor SBOMs or record the vendor’s stated disclosure limits.'],['High — actual delivered artifacts','Freeze the actual Step/Beyond pair, deployment config, image digests and NixOS closure. Add per-image and system SBOMs plus provenance/manifests from the actual release; source references alone are insufficient to inventory OS/image contents.'],['Review — unknown/nonstandard licenses and owners',str(stats['unresolved'])+' component rows use UNRESOLVED rights profile; '+str(stats['owner_unverified'])+' rows have no verified legal-owner evidence. Review the workbook’s Review queue and preserve complete upstream notices. Registry author names are not proof of copyright title.'],['Review — ownership / contract grant','Authorized Sequent signatory to confirm the exact legal entity, its original-code ownership and any chain-of-title evidence; execute the required COMELEC rights grant. Beyond’s proprietary notice grants no rights by possession alone.']]
 story+=[tbl(gaps,[145,380])]
 story+=[PageBreak(),P('3. How to read the complete rights schedule','HeadX'),P('Each component appendix row identifies the component, exact version/revision, purpose, rights-holder evidence, license declaration and rights profile. The profile supplies the remaining TOR fields and is incorporated by reference. Per-row materialized fields, source paths, PURLs and review notes are in 03-Rights-and-Dependencies.xlsx and 04-Rights-and-Dependencies.csv. A license declaration is recorded as evidence, not an assertion that legal compatibility has been approved.')]
 for key,profile in PROFILES.items():
  story+=[P(key+' — '+{'OPEN':'third-party declared open-source license','SEQUENT':'Sequent original modules','UNRESOLVED':'rights evidence incomplete','SERVICE':'commercial or account-based service'}[key],'SubX')]
  story+=[tbl([['TOR field','Applicable schedule entry']]+[[f,profile[f]] for f in FIELDS[2:]],[125,400])]
 story+=[PageBreak(),P('4. Services and external rights','HeadX'),P('These rows are supported integrations or infrastructure options found in source. None is represented as a confirmed purchase. Final bid inclusion/exclusion and contractual values must be decided explicitly. Every row uses the SERVICE profile above, with its purpose, owner and exception notes below.')]
 for s in svc:
  story+=[P(s['ID']+' • '+s['Component / service'],'SubX'),P(s['Purpose']+'. Owner/provider: '+s['Owner']+'.'),P(s['Comment']),P('Evidence: '+s['Evidence'],'SmallX')]
 story+=[PageBreak(),P('5. Sequent modules and imported source','HeadX'),P('The following module declarations identify the original product scope and local exceptions. Exact source files and REUSE notices govern each file. This list includes tooling and tests where present; it is not a deployment manifest.')]
 story+=[tbl([['Module / manifest','License declaration / classification']]+[[m['name']+'\n'+m['path'],m['license']+'\n'+m['classification']] for m in mods],[295,230])]
 story+=[P('6. Inventory method and evidence','HeadX'),P('Source archives were obtained by commit SHA through GitHub. Syft v1.55.0 scanned lockfiles, requirements and Maven manifests. A separate Maven pass enabled network-backed parent/property and transitive-dependency resolution against Maven Central. Declared dependencies remain distinct from the resolved production runtime; unresolved Maven/BOM/build-plugin or unpinned Python references still require an actual build inventory.')]
 story+=[P('License metadata was enriched against exact versions from npm, the crates.io index, PyPI and Maven Central, supplemented by exact local Cargo manifests and exact-PURL matches to the separately dated GitHub export. No package was resolved to “latest” for licensing. Registry license/author declarations are not a legal conclusion. Full license and ownership evidence references are retained per row. Unknown fields remain explicit rather than inferred.')]
 story+=[P('Supplemental analysis records first-party/local modules, REUSE-declared imported assets, container references, Helm dependencies and pinned Nix inputs. No source code, private keys or runtime credentials are included in this submission package. Selected licensing notices and evidence hashes are included; access to proprietary source is governed separately.')]
 story+=[tbl([['Inventory check','Result'],['Scanner occurrences (language / CI packages)',stats['scanner_occurrences']],['Deduplicated + supplemental component rows',len(rows)],['Rights profiles',', '.join(k+': '+str(v) for k,v in stats['profiles'].items())],['Container / Helm / Nix source references',len(extra)],['External service review entries',len(svc)],['SPDX document scope','Source inventory; package licenseDeclared carries evidence; licenseConcluded is NOASSERTION unless reviewed separately. File-level deployment/OS contents not claimed.']],[270,255])]
 story+=[P('7. Release and submission checklist','HeadX')]
 for t in ['Confirm the exact offered release and Step/Beyond revision pairing; retain source archive and build/input hashes.','Resolve Trellis/imported-source rights, unknown/nonstandard license declarations and legal-owner entries. Confirm any copyleft, notice, corresponding-source and combination obligations for the delivered distribution.','Fill and approve every included service’s commercial terms and complete COMELEC tenant ownership/control, region and handover fields. Mark genuinely excluded services as not applicable with reasons.','Attach actual image/OS/Nix closure inventories and vendor disclosures. Reconcile all deployed packages against this source inventory; retain required license texts and source/build-access materials.','Have an authorized Sequent representative review and sign the separate ownership declaration and the final rights grant. Print this document and the complete component appendix, and supply the machine-readable/workbook attachments.']:
  story+=[P('☐  '+t)]
 story+=[P('Prepared by: ____________________   Reviewed by: ____________________'),P('Authorized approval / date: ________________________________________')]
 return story


def declaration(repo):
 story=[P('SEQUENT TECH INC','SubX'),P('Declaration of ownership\nof original software','TitleX'),P('For authorized review and signature • OVCS 2028 • '+DATE,'SubX')]
 if repo=='beyond':story+=[P('Separate supporting declaration — inclusion in the bid is not yet confirmed.','SubX')]
 story+=[P('Software: '+repo.upper()+' repository (sequentech/'+repo+'), ovcs branch snapshot '+SHAS[repo]+'.'),P('Sequent Tech Inc declares that it owns the copyright and intellectual property rights in the original software code authored and developed by Sequent in the above repository. The applicable Sequent source notices identify Sequent Tech Inc / Sequent Tech and legal@sequentech.io as the copyright holder/contact.')]
 story+=[P('Sequent’s ownership of its original code is retained when that code is licensed to others. A reference to the GNU Affero General Public License or another license does not transfer Sequent’s copyright to the author or publisher of that license.')]
 story+=[P('This declaration distinguishes Sequent’s original code from third-party and imported materials. Such materials retain their respective owners and applicable rights, as recorded in the accompanying Software Bill of Materials and Third-Party Rights Schedule. This declaration does not assert that Sequent owns third-party code, fonts, models, vendor binaries or services.')]
 if repo=='step':
  story+=[P('The Step repository generally declares AGPL-3.0-only for Sequent code, with file-level exceptions. Trellis is Sequent-owned code under AGPL-3.0-only, as confirmed by Sequent on 29 October 2026 and recorded in meta#14601; its original author and development provenance are retained. This statement must not be used as a blanket certification that all third-party rights are already complete.')]
 else:
  story+=[P('Beyond contains both proprietary Sequent files and AGPL-3.0-only files. LICENSES/LicenseRef-Proprietary.txt reserves Sequent’s rights and states that possession of the code grants no license. Any rights granted to COMELEC for proprietary portions must be documented in the authorized agreement; the AGPL portions remain governed by their applicable license.')]
 story+=[P('The rights to be granted to COMELEC, including any perpetual/non-exclusive license, access and interoperability commitments required by TOR 13.3, are to be stated in the duly authorized bid and agreement. This declaration is evidence of original-code ownership and does not replace those grants or third-party agreements.')]
 story+=[Spacer(1,20),P('For and on behalf of Sequent Tech Inc:'),P('Authorized representative: _______________________________________'),P('Position / title: __________________________________________________'),P('Signature: ______________________________________________________'),P('Date and place: __________________________________________________'),Spacer(1,8),P('Supporting evidence: repository README/SPDX headers, REUSE.toml, selected LICENSES notices, module schedule, exact revision and archive hash in the accompanying package. Signatory to confirm the contracting entity and authority before execution.','SmallX')]
 return story


def append_pdf(repo,rows):
 story=[P(repo.upper()+' / Complete component inventory','TitleX'),P(str(len(rows))+' component records • ovcs '+SHAS[repo]+' • '+DATE,'SmallX'),P('Use with 01-Submission-and-Rights-Schedule.pdf: its OPEN, SEQUENT, UNRESOLVED and SERVICE profiles supply the remaining TOR fields for each component. Full expanded fields and unabridged evidence are in the workbook/CSV. “Upstream” refers to the package’s rights holders; author declarations are not verified ownership. Source occurrences may include build/test/documentation dependencies.','SmallX')]
 data=[[P(x,'TinyX') for x in ['Record','Component / version','Purpose / scope','Owner evidence','License declaration / rights profile']]]
 for r in rows:
  # Purpose is a concise description; preserve full license and owner statements.
  pur=r['Purpose'];pur=pur if len(pur)<190 else pur[:187]+'…'
  own=r['Owner'];own=own if len(own)<550 else 'Full upstream copyright statement in electronic record '+r['ID']+'. '+('Registry author declaration available; see workbook.' if r['Owner evidence'].startswith('Registry') else 'See package source notices and unabridged rights schedule.')
  vals=[r['ID'],r['Component']+'\n'+r['Version / revision'],pur+'\n'+r['Scope'],own,r['License declaration']+'\n['+r['Rights profile']+']']
  data.append([P(x,'TinyX') for x in vals])
 t=LongTable(data,colWidths=[72,165,195,180,160],repeatRows=1,hAlign='LEFT')
 t.setStyle(TableStyle([('VALIGN',(0,0),(-1,-1),'TOP'),('BACKGROUND',(0,0),(-1,0),LIGHT),('ROWBACKGROUNDS',(0,1),(-1,-1),[colors.white,colors.HexColor('#F6F8F8')]),('LEFTPADDING',(0,0),(-1,-1),5),('RIGHTPADDING',(0,0),(-1,-1),5),('TOPPADDING',(0,0),(-1,-1),4),('BOTTOMPADDING',(0,0),(-1,-1),4),('LINEBELOW',(0,0),(-1,0),.8,TEAL)]))
 story.append(t);return story


def workbook(path,rows,mods,extra,svc,stats):
 wb=Workbook();wb.remove(wb.active)
 tabs={'Read me':[['Field','Value'],['Purpose','TOR 13.6.2 branch-source SBOM + rights schedule. Prepared for review; see blockers.'],['Release',path.parent.name],['Revision',SHAS[path.parent.name.split('-')[0].lower()]],['Print','01 main + 02 full appendix. 05 ownership declaration requires authorized signature.'],['Status','Source evidence collected does not mean complete legal/deployment compliance.'],['Complete TOR fields',', '.join(FIELDS)],['Unresolved rights rows',stats['unresolved']],['Owner evidence unverified rows',stats['owner_unverified']]],'Components':rows,'Local modules':mods,'Services':svc,'Infra references':extra,'Review queue':[r for r in rows if r['Review status']=='Needs confirmation' or r['License declaration']=='NOASSERTION' or r['Version / revision'] in ('UNKNOWN','inherited')],'Rights profiles':[dict(Profile=k,**v) for k,v in PROFILES.items()]}
 for name,data in tabs.items():
  ws=wb.create_sheet(name)
  if data and isinstance(data[0],dict):
   keys=[k for k in data[0] if not k.startswith('_')];ws.append(keys)
   for r in data:ws.append([str(r.get(k,''))[:32767] for k in keys])
  else:
   for r in data:ws.append(r)
  ws.freeze_panes='B2';ws.auto_filter.ref=ws.dimensions
  ws.sheet_properties.pageSetUpPr.fitToPage=True;ws.page_setup.orientation='landscape';ws.page_setup.paperSize=ws.PAPERSIZE_A3;ws.page_setup.fitToHeight=0;ws.page_setup.fitToWidth=1;ws.print_title_rows='1:1'
  for c in ws[1]:c.font=Font(name='Calibri',bold=True,color='FFFFFF');c.fill=PatternFill('solid',fgColor='143344');c.alignment=Alignment(wrap_text=True,vertical='top')
  for col in ws.columns:
   header=str(col[0].value);ws.column_dimensions[col[0].column_letter].width=24 if header in ('ID','Version / revision','Type','Rights profile','Review status') else 52
   for c in col[1:]:
    c.alignment=Alignment(wrap_text=True,vertical='top');c.font=Font(name='Calibri',size=10)
    # Keep all imported metadata as literal data, never Excel formulas.
    if isinstance(c.value,str) and c.value.startswith(('=','+','-','@')):c.data_type='s'
  if name in ('Components','Services','Review queue'):
   hs=[c.value for c in ws[1]]
   if 'Review status' in hs:
    from openpyxl.utils import get_column_letter
    letter=get_column_letter(hs.index('Review status')+1);dv=DataValidation(type='list',formula1='"Source evidence collected,Source license evidence recorded,Needs confirmation,Approved,Excluded from bid"');ws.add_data_validation(dv);dv.add(letter+'2:'+letter+str(ws.max_row))
 wb.save(path)


def spdx(repo,rows,raw,maven,idmap):
 root='SPDXRef-'+repo+'-source';packages=[];rels=[{'spdxElementId':'SPDXRef-DOCUMENT','relationshipType':'DESCRIBES','relatedSpdxElement':root}];extracted={}
 def valid_license(s):
  # Preserve unrecognized declarations as LicenseRef records instead of invalid SPDX expressions.
  try:
   LICENSING.parse(s,validate=True,strict=True)
   return s
  except Exception:
   if s in ('NOASSERTION','NONE'):return s
   if re.fullmatch(r'LicenseRef-[A-Za-z0-9.-]+',s):return s
   key='LicenseRef-Declaration-'+hashlib.sha256(s.encode()).hexdigest()[:14];extracted.setdefault(key,dict(licenseId=key,extractedText='Upstream declaration as recorded: '+s+'\nNot a substituted license grant; consult the exact upstream license and evidence.'));return key
 packages.append(dict(name='sequentech/'+repo,SPDXID=root,versionInfo=SHAS[repo],downloadLocation='git+https://github.com/sequentech/'+repo+'.git@'+SHAS[repo],filesAnalyzed=False,licenseConcluded='NOASSERTION',licenseDeclared='AGPL-3.0-only' if repo=='step' else 'NOASSERTION',copyrightText='Sequent Tech Inc (original Sequent-authored code only); third-party copyrights retained',supplier='Organization: Sequent Tech Inc',comment='OVCS branch source inventory. Includes development/tooling declarations and separately identified exceptions. Not a built-image/OS SBOM. Submodule content not traversed.'))
 for r in rows:
  sid='SPDXRef-'+r['ID'];lic=valid_license(r['License declaration'])
  p=dict(name=r['Component'],SPDXID=sid,versionInfo=r['Version / revision'],downloadLocation='NOASSERTION',filesAnalyzed=False,licenseConcluded='NOASSERTION',licenseDeclared=lic,copyrightText=r['Owner'] if r['Owner evidence']=='Source notices' or r['Owner evidence'].startswith('Copyright') else 'NOASSERTION',summary=r['Purpose'],comment='Rights schedule: '+r['ID']+'; profile '+r['Rights profile']+'. Scope: '+r['Scope']+'. Evidence: '+r['Evidence paths']+'. License evidence: '+r['License evidence']+'. Owner evidence: '+r['Owner evidence']+'.')
  if r['Classification']=='Sequent-authored module':p['supplier']='Organization: Sequent Tech Inc'
  if r.get('PURL'):p['externalRefs']=[dict(referenceCategory='PACKAGE-MANAGER',referenceType='purl',referenceLocator=r['PURL'])]
  md=r.get('_metadata',{});checks=[]
  if md.get('checksum') and re.fullmatch('[0-9a-f]{64}',md['checksum']):checks.append(dict(algorithm='SHA256',checksumValue=md['checksum']))
  integ=md.get('integrity','')
  for term in integ.split():
   try:
    algo,val=term.split('-',1)
    if algo in ('sha1','sha256','sha384','sha512'):checks.append(dict(algorithm=algo.upper(),checksumValue=base64.b64decode(val,validate=True).hex()))
   except Exception:pass
  if checks:p['checksums']=checks
  packages.append(p);rels.append(dict(spdxElementId=root,relationshipType='CONTAINS',relatedSpdxElement=sid))
  for custom in re.findall(r'LicenseRef-[A-Za-z0-9.-]+',lic):
   if custom not in extracted:
    f=WORK/repo/'LICENSES'/(custom+'.txt');extracted[custom]=dict(licenseId=custom,extractedText=f.read_text() if f.exists() else 'Declaration recorded in source: '+r['License declaration']+'. Obtain governing terms; this record grants no additional rights.')
 seen=set()
 for rel in raw.get('artifactRelationships',[])+maven.get('artifactRelationships',[]):
  a=idmap.get(rel.get('parent'));b=idmap.get(rel.get('child'))
  if a and b and a!=b and rel.get('type') in ('dependency-of',):
   key=(a,b)
   if key not in seen:rels.append(dict(spdxElementId='SPDXRef-'+a,relationshipType='DEPENDENCY_OF',relatedSpdxElement='SPDXRef-'+b));seen.add(key)
 doc=dict(spdxVersion='SPDX-2.3',dataLicense='CC0-1.0',SPDXID='SPDXRef-DOCUMENT',name=repo+'-ovcs-'+SHAS[repo],documentNamespace='https://sequentech.io/spdx/ovcs/'+repo+'/'+SHAS[repo]+'/'+hashlib.sha256((CREATED+repo).encode()).hexdigest()[:12],creationInfo=dict(creators=['Organization: Sequent Tech Inc','Tool: syft-1.55.0','Tool: ovcs-submission-builder-1.0'],created=CREATED),packages=packages,relationships=rels)
 if extracted:doc['hasExtractedLicensingInfos']=list(extracted.values())
 return doc


def write_csv(path,records):
 if not records:return
 keys=[k for k in records[0] if not k.startswith('_')]
 with open(path,'w',newline='',encoding='utf-8-sig') as f:
  w=csv.DictWriter(f,fieldnames=keys,extrasaction='ignore');w.writeheader();w.writerows(records)

def evidence(repo,dest,rows,mods):
 root=WORK/repo;ev=dest/'evidence';ev.mkdir(exist_ok=True);items=[]
 selected=['README.md','REUSE.toml','.gitmodules']
 selected +=[str(p.relative_to(root)) for p in (root/'LICENSES').glob('*') if p.is_file()]
 for rel in selected:
  p=root/rel
  if p.exists():out=ev/'source-notices'/rel;out.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,out);items.append(dict(Path=rel,SHA256=sha(p),Purpose='Selected source notice / submodule declaration'))
 for p in root.rglob('*'):
  if p.is_file() and (p.name in ('Cargo.lock','yarn.lock','package-lock.json','uv.lock','pom.xml','requirements.txt','flake.lock','devenv.lock','Dockerfile.prod','prepare.sh') or p.suffix=='.license'):
   items.append(dict(Path=str(p.relative_to(root)),SHA256=sha(p),Purpose='Manifest / lock / notice evidence; source content not included'))
 write_csv(ev/'source-evidence-hashes.csv',items)
 vendored=[]
 for a in tomllib.loads((root/'REUSE.toml').read_text()).get('annotations',[]):
  owner=a.get('SPDX-FileCopyrightText',[]);owner=[owner] if isinstance(owner,str) else owner
  if not any(o and 'Sequent' not in o for o in owner):continue
  pats=a.get('path',[]);pats=[pats] if isinstance(pats,str) else pats
  for pat in pats:
   for p in root.glob(pat):
    if p.is_file():vendored.append(dict(Path=str(p.relative_to(root)),SHA256=sha(p),Owner='; '.join(owner),License=a.get('SPDX-License-Identifier','NOASSERTION')))
 write_csv(ev/'vendored-files.csv',vendored)
 # Scanner output is source metadata, not source code. Remove local absolute source location.
 for label,src in [('scanner-source',EVIDENCE/(repo+'.syft.json')),('scanner-maven',EVIDENCE/(repo+'-maven.syft.json'))]:
  d=json.load(open(src))
  d['source']['name']='sequentech/'+repo+'@'+SHAS[repo]
  d['source']['metadata']={'path':'repository-source'}
  d.get('descriptor',{}).get('configuration',{}).pop('registry',None)
  (ev/(label+'.syft.json')).write_text(json.dumps(d,ensure_ascii=False,indent=2))
 used={r['PURL']:META.get(r['PURL'],{}) for r in rows if r.get('PURL')}
 (ev/'exact-version-license-evidence.json').write_text(json.dumps(used,ensure_ascii=False,indent=2))
 prov=dict(repository='sequentech/'+repo,branch='ovcs',commit=SHAS[repo],source_archive_sha256=sha(WORK/(repo+'.tar.gz')),submodule=(dict(repository='sequentech/'+PIN[repo][0],commit=PIN[repo][1],contents_scanned=False) if PIN[repo][1] else None),prepared_utc=CREATED,scanner='Syft v1.55.0',scanner_source=dict(release=json.load(open(WORK/'scan-provenance.json'))['syft_release'],asset_url=json.load(open(WORK/'scan-provenance.json'))['syft_asset_url'],binary_sha256=sha(WORK/'syft')),scope='Source-declared inventory; not built image/OS closure',license_metadata='Exact versions only; evidence per component; no legal compatibility conclusion')
 (ev/'provenance.json').write_text(json.dumps(prov,indent=2))
 return prov


def main():
 overview={}
 for repo in ('step','beyond'):
  dest=BASE/(repo.upper()+'-OVCS');dest.mkdir(exist_ok=True)
  mods=scan_modules(repo);rows,raw,maven,idmap=make_rows(repo,mods);add_assets(repo,rows);extra=extras(repo);svc=services(repo)
  add_infra(repo,rows,extra)
  stats=dict(scanner_occurrences=sum(p['type']!='java-archive' for p in raw['artifacts'])+len(maven['artifacts']),profiles=dict(collections.Counter(r['Rights profile'] for r in rows)),unresolved=sum(r['Rights profile']=='UNRESOLVED' for r in rows),owner_unverified=sum(r['Owner evidence'].startswith(('Registry','Not verified')) for r in rows),components=len(rows),modules=len(mods),infra_references=len(extra))
  print(repo,stats,flush=True)
  write_csv(dest/'04-Rights-and-Dependencies.csv',rows);write_csv(dest/'Services.csv',svc);write_csv(dest/'Infrastructure-References.csv',extra)
  jsonrows=[{k:v for k,v in r.items() if not k.startswith('_')} for r in rows]
  (dest/'Rights-and-Dependencies.json').write_text(json.dumps(dict(repository=repo,revision=SHAS[repo],components=jsonrows,services=svc,infrastructure_references=extra,rights_profiles=PROFILES,statistics=stats),ensure_ascii=False,indent=2))
  (dest/'06-Source-SBOM.spdx.json').write_text(json.dumps(spdx(repo,rows,raw,maven,idmap),ensure_ascii=False,indent=2))
  workbook(dest/'03-Rights-and-Dependencies.xlsx',rows,mods,extra,svc,stats)
  pdf(dest/'01-Submission-and-Rights-Schedule.pdf',front(repo,rows,mods,extra,svc,stats),repo)
  pdf(dest/'02-Complete-Component-Inventory.pdf',append_pdf(repo,rows),repo,True)
  pdf(dest/'05-Ownership-Declaration.pdf',declaration(repo),repo)
  from pypdf import PdfWriter
  combined=PdfWriter()
  for name,title in [('01-Submission-and-Rights-Schedule.pdf','Submission and rights schedule'),('05-Ownership-Declaration.pdf','Original-code ownership declaration'),('02-Complete-Component-Inventory.pdf','Complete component inventory')]:combined.append(str(dest/name),outline_item=title)
  with open(dest/(repo.upper()+'-OVCS-Complete-Print.pdf'),'wb') as combined_file:combined.write(combined_file)
  combined.close()
  provenance=evidence(repo,dest,rows,mods)
  (dest/'README.txt').write_text(repo.upper()+' OVCS branch submission preparation\nRevision: '+SHAS[repo]+'\n\nComplete-Print.pdf combines 01, 05 and 02 for printing. Alternatively print 01 plus 02 and review/sign 05. Provide 03, 04 and 06 electronically. Services.csv and Infrastructure-References.csv supplement the source SBOM.\n\nPrepared for review: complete commercial service terms and owner/notice evidence and actual image/OS release inventory before unconditional submission.\n'+('Beyond inclusion is unconfirmed; include relevant modules/dependencies if actually used by the offered OVCS, regardless of private repository status.\n' if repo=='beyond' else revision_pair(repo)+'\n'))
  overview[repo]=dict(stats,revision=SHAS[repo],provenance=provenance)
 (BASE/'Preparation-Summary.json').write_text(json.dumps(overview,indent=2))
 (BASE/'READ-FIRST.txt').write_text('OVCS 2028 — SBOM and third-party rights preparation — 29 October 2026\n\nSTEP-OVCS is the main package. BEYOND-OVCS is separate, inclusion to be confirmed. All inventories are exact ovcs snapshots, not GitHub default-branch exports.\n\nPDFs: 01 narrative, rights profiles and services; 02 complete component inventory; 05 signature-ready original-code ownership declaration. XLSX/CSV expands every TOR field per component; SPDX JSON supplies source inventory.\n\nBefore final submission: close the highlighted licensing/service/deployment items. Sequent owns its original code; imported and vendor materials retain their own rights. Trellis is Sequent-owned and declares AGPL-3.0-only; see meta#14601. Beyond includes proprietary and AGPL portions.\n\nSource archives and the scanner executable remain local under work/ and are intentionally excluded from the handoff ZIP. No deployment secrets or source-code archive is included in the ZIP.\n')
 # Seal only deliverables, not the workspace/cache or executables.
 files=[p for r in ('STEP-OVCS','BEYOND-OVCS') for p in sorted((BASE/r).rglob('*')) if p.is_file()]
 files +=[BASE/'READ-FIRST.txt',BASE/'Preparation-Summary.json']
 (BASE/'SHA256SUMS').write_text(''.join(sha(p)+'  '+str(p.relative_to(BASE))+'\n' for p in files))
 with zipfile.ZipFile(BASE/'OVCS-SBOM-and-Rights-Packages.zip','w',compression=zipfile.ZIP_DEFLATED,compresslevel=6) as z:
  for p in files+[BASE/'SHA256SUMS']:z.write(p,str(p.relative_to(BASE)))
 print('Built',BASE/'OVCS-SBOM-and-Rights-Packages.zip',flush=True)
if __name__=='__main__':main()
