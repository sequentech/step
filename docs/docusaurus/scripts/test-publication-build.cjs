const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {execFileSync} = require('node:child_process');
const plugin = require('./publication-build.cjs');
function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'publication-build-test-'));
  const site = path.join(root, 'docs/docusaurus');
  const outDir = path.join(site, 'build');
  fs.mkdirSync(path.join(site, '.docusaurus'), {recursive:true});
  fs.writeFileSync(path.join(site, '.gitignore'), 'build/\n.docusaurus/\n');
  const pairs = [
    ['docs/06-technology/06-vulnerability-disclosure-policy.md','docs/technology/vulnerability-disclosure-policy.html'],
    ['static/security/pgp-key.txt','security/pgp-key.txt'],
    ['static/.well-known/security.txt','.well-known/security.txt'],
    ['static/.well-known/csaf/provider-metadata.json','.well-known/csaf/provider-metadata.json'],
  ];
  for (const [source, output] of pairs) {
    fs.mkdirSync(path.dirname(path.join(site, source)), {recursive:true});
    fs.writeFileSync(path.join(site, source), 'Synthetic source: '+source);
    fs.mkdirSync(path.dirname(path.join(outDir, output)), {recursive:true});
    fs.writeFileSync(path.join(outDir, output), 'Synthetic output: '+output);
  }
  const git = (...args) => execFileSync('git', ['-C', root, ...args], {encoding:'utf8'}).trim();
  git('init','-q'); git('add','.');
  git('-c','user.name=Test','-c','user.email=test@example.invalid','-c','commit.gpgSign=false','commit','-qm','Synthetic publication build');
  return {root, site, outDir, git, context:{siteDir:site,siteConfig:{baseUrl:'/docusaurus/main/'}},
    clean:() => fs.rmSync(root,{recursive:true,force:true})};
}
async function using(fn) {const f=fixture(); try {await fn(f);} finally {f.clean();}}
test('records four outputs and exact committed inputs outside the public build', () => using(async f => {
  await plugin(f.context).postBuild({outDir:f.outDir});
  const record=JSON.parse(fs.readFileSync(path.join(f.site,'.docusaurus/publication-build.json')));
  assert.equal(record.source_commit,f.git('rev-parse','HEAD'));
  assert.equal(record.source_dirty,false);
  assert.equal(Object.keys(record.artifacts).length,4);
  assert.equal(Object.keys(record.files).length,5);
  assert.equal(fs.existsSync(path.join(f.outDir,'publication-build.json')),false);
}));
test('refuses source changes while rendering', () => using(async f => {
  const p=plugin(f.context);
  fs.appendFileSync(path.join(f.site,'static/security/pgp-key.txt'),'changed');
  await assert.rejects(() => p.postBuild({outDir:f.outDir}), /inputs changed/);
}));
test('identifies a dirty public source without claiming committed output', () => using(async f => {
  fs.appendFileSync(path.join(f.site,'static/security/pgp-key.txt'),'changed');
  await plugin(f.context).postBuild({outDir:f.outDir});
  const record=JSON.parse(fs.readFileSync(path.join(f.site,'.docusaurus/publication-build.json')));
  assert.equal(record.source_dirty,true);
}));
test('hashes the generated offline-mirror routing separately from Git sources', () => using(async f => {
  fs.writeFileSync(path.join(f.site,'.yarnrc'),'yarn-offline-mirror "/accepted/vendor/yarn-offline-mirror"\n');
  const p=plugin(f.context);
  await p.postBuild({outDir:f.outDir});
  const record=JSON.parse(fs.readFileSync(path.join(f.site,'.docusaurus/publication-build.json')));
  assert.equal(record.source_dirty,false);
  assert.match(record.local_yarnrc_sha256,/^[0-9a-f]{64}$/);
  fs.appendFileSync(path.join(f.site,'.yarnrc'),'changed');
  await assert.rejects(() => p.postBuild({outDir:f.outDir}), /inputs changed/);
}));
test('binds selected-doc route identity and local raster bytes to the actual build', () => using(async f => {
  const p=plugin(f.context);
  const file=path.join(f.outDir,'assets/example.png');fs.mkdirSync(path.dirname(file),{recursive:true});fs.writeFileSync(file,'synthetic image bytes');
  await p.allContentLoaded({allContent:{'docusaurus-plugin-content-docs':{default:{loadedVersions:[{docs:[{
    source:'@site/docs/06-technology/06-vulnerability-disclosure-policy.md',title:'Synthetic policy',permalink:'/docusaurus/main/docs/technology/vulnerability-disclosure-policy',draft:false,
  }]}]}}}});
  await p.postBuild({outDir:f.outDir});
  const r=JSON.parse(fs.readFileSync(path.join(f.site,'.docusaurus/publication-build.json')));
  assert.equal(r.pages.length,1);assert.equal(r.pages[0].source,'docs/docusaurus/docs/06-technology/06-vulnerability-disclosure-policy.md');
  assert.equal(r.pages[0].sha256,r.artifacts['vulnerability-disclosure-policy'].sha256);
  assert.match(r.assets['assets/example.png'],/^[0-9a-f]{64}$/);
}));
