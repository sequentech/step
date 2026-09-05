const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const crypto = require('node:crypto');
const {execFileSync} = require('node:child_process');
const {exportGuidance} = require('./export-guidance.cjs');
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(),'guidance-test-'));
  const site = path.join(root,'docs/docusaurus'); const build = path.join(site,'build');
  const write = (name, bytes) => {fs.mkdirSync(path.dirname(path.join(root,name)),{recursive:true});fs.writeFileSync(path.join(root,name),bytes);};
  write('.gitignore','docs/docusaurus/build/\ndocs/docusaurus/.docusaurus/\n');
  const docs = [1,2,3,4].map(n => ({id:`AGD-0${n}`,title:`Synthetic handbook ${n}`,version:'test-only',sources:[`docs/docusaurus/docs/03-voters/page${n}.md`]}));
  for (const [i,doc] of docs.entries()) {
    write(doc.sources[0],`# Page ${i+1}\n\nSynthetic fixture only.`);
    write(`docs/docusaurus/build/docs/page${i+1}.html`,`<html><nav>PRIVATE NAVIGATION</nav><article><div class="markdown"><h1>Page ${i+1}</h1><h2 id="detail">Details</h2><img src="/manual/assets/image.png"><a href="/manual/docs/page${(i+1)%4+1}#detail">Next</a><a href="https://example.invalid/reference">Reference</a><pre><button>Copy</button>Example</pre></div></article><script>SECRET SEARCH PAYLOAD</script></html>`);
  }
  write('docs/docusaurus/build/assets/image.png',Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl6tXsAAAAASUVORK5CYII=','base64'));
  write('docs/docusaurus/build/internal.html','NEVER INCLUDE');
  const git = (...args) => execFileSync('git',['-C',root,...args],{encoding:'utf8'}).trim();
  git('init','-q');git('add','.');git('-c','user.name=Test','-c','user.email=test@example.invalid','-c','commit.gpgSign=false','commit','-qm','Synthetic selection');
  const commit = git('rev-parse','HEAD');
  const manifest = {schema:1,base_url:'/manual/',source_dirty:false,source_commit:commit,
    files:Object.fromEntries(docs.map(d => [d.sources[0],sha(fs.readFileSync(path.join(root,d.sources[0])))])),
    pages:docs.map((d,i)=>({source:d.sources[0],title:`Page ${i+1}`,permalink:`/manual/docs/page${i+1}`,output:`docs/page${i+1}.html`,sha256:sha(fs.readFileSync(path.join(build,`docs/page${i+1}.html`)))})),
    assets:{'assets/image.png':sha(fs.readFileSync(path.join(build,'assets/image.png')))}};
  const record = () => write('docs/docusaurus/.docusaurus/publication-build.json',JSON.stringify(manifest));record();
  const selection={schema:1,tag:'synthetic-v0',toe_reference:'Synthetic TOE',commit,documents:docs};
  const out=path.join(root,'export');
  return {root,site,build,write,git,manifest,record,selection,out,run:()=>exportGuidance({site,selection,out}),clean:()=>fs.rmSync(root,{recursive:true,force:true})};
}
const using = fn => {const f=fixture();try {return fn(f);} finally {f.clean();}};
test('exports exactly selected articles and images, with offline links and identities',()=>using(f=>{
  const m=f.run();assert.equal(m.documents.length,4);assert.equal(Object.keys(m.files).length,10);
  const html=fs.readFileSync(path.join(f.out,'AGD-01/001.html'),'utf8');
  assert.match(html,/\.\.\/AGD-02\/001.html#detail/);assert.match(html,/test-only/);assert.match(html,/synthetic-v0/);
  assert.doesNotMatch(html,/PRIVATE|SECRET|<script|<button|loading=/);assert.equal(fs.existsSync(path.join(f.out,'internal.html')),false);
  for (const [name,digest] of Object.entries(m.files)) assert.equal(sha(fs.readFileSync(path.join(f.out,name))),digest);
  assert.equal(m.external_references.length,4);assert.throws(f.run,/already exists/);
}));
test('refuses ambiguous or missing handbook selection and internal source trees',()=>using(f=>{
  f.selection.documents[0].sources=[];assert.throws(f.run,/nonempty/);
  f.selection.documents[0].sources=['evidence/bsi-certification/2_alc/9_Internal_To_Do.md'];assert.throws(f.run,/outside/);
  f.selection.documents[0].sources=f.selection.documents[1].sources;assert.throws(f.run,/identical/);
  assert.equal(fs.existsSync(f.out),false);
}));
test('refuses changed rendered articles and changed image bytes',()=>using(f=>{
  const p=path.join(f.build,'docs/page1.html');const old=fs.readFileSync(p);fs.appendFileSync(p,'changed');assert.throws(f.run,/article changed/);fs.writeFileSync(p,old);
  fs.appendFileSync(path.join(f.build,'assets/image.png'),'changed');assert.throws(f.run,/Image is unrecorded or changed/);
}));
test('refuses dirty source even when timestamps are restored',()=>using(f=>{
  const p=path.join(f.root,f.selection.documents[0].sources[0]);const stat=fs.statSync(p);fs.appendFileSync(p,'changed');fs.utimesSync(p,stat.atime,stat.mtime);assert.throws(f.run,/clean selected source/);
}));
function changeArticle(f, content) {
  const p=path.join(f.build,'docs/page1.html');fs.writeFileSync(p,`<article><div class="markdown"><h1>Page 1</h1>${content}</div></article>`);
  f.manifest.pages[0].sha256=sha(fs.readFileSync(p));f.record();
}
test('refuses unselected local links, absent fragments and remote images',()=>using(f=>{
  changeArticle(f,'<a href="/manual/docs/internal">Unselected</a>');assert.throws(f.run,/leaves the selected/);
  changeArticle(f,'<a href="/manual/docs/page2#absent">Missing</a>');assert.throws(f.run,/missing fragment/);
  changeArticle(f,'<img src="https://example.invalid/remote.png">');assert.throws(f.run,/local built asset/);
}));
test('refuses embedded, client-only and active content without writing a package',()=>using(f=>{
  for (const html of ['<iframe src="https://example.invalid/video"></iframe>','<div class="docusaurus-mermaid-container"></div>','<p onclick="evil()">Text</p>','<img src="/manual/assets/image.png" srcset="https://example.invalid/tracker 2x">','<svg><style>@import "https://example.invalid/style"</style></svg>']) {
    changeArticle(f,html);assert.throws(f.run,/adapter|content|style resource/);assert.equal(fs.existsSync(f.out),false);
  }
}));
test('refuses path traversal or symlink substitution in recorded sources',()=>using(f=>{
  f.manifest.files['../outside']='0'.repeat(64);f.record();assert.throws(f.run,/Invalid relative/);delete f.manifest.files['../outside'];f.record();
  const p=path.join(f.root,f.selection.documents[0].sources[0]);const text=fs.readFileSync(p);fs.unlinkSync(p);fs.writeFileSync(path.join(f.root,'outside.md'),text);fs.symlinkSync(path.join(f.root,'outside.md'),p);assert.throws(f.run,/clean selected|leaves its source/);
}));

test('refuses unfinished source, unrendered diagrams and MDX components',()=>using(f=>{
  const source=f.selection.documents[0].sources[0];
  for (const body of ['Content will be added here soon.','```mermaid\ngraph LR; A-->B\n```','<GoogleVideo id="synthetic" />']) {
    f.write(source,'# Page 1\n\n'+body);
    f.git('add',source);f.git('-c','user.name=Test','-c','user.email=test@example.invalid','-c','commit.gpgSign=false','commit','-qm','Synthetic unsupported source');
    f.selection.commit=f.git('rev-parse','HEAD');f.manifest.source_commit=f.selection.commit;
    f.manifest.files[source]=sha(fs.readFileSync(path.join(f.root,source)));f.record();
    assert.throws(f.run,/placeholder|rendering adapter/);assert.equal(fs.existsSync(f.out),false);
  }
}));
