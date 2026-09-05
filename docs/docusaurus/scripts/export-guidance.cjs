#!/usr/bin/env node
// Select the rendered consumer articles and their local images, without site code/search.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {execFileSync} = require('node:child_process');
const cheerio = require('cheerio');
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const escape = text => String(text).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
function inside(root, name) {
  if (typeof name !== 'string' || !name || path.isAbsolute(name) || name.includes('\\') || name.split('/').some(p => !p || p === '.' || p === '..'))
    throw Error('Invalid relative export path: '+name);
  const file = path.join(root, name);
  if (!fs.realpathSync(file).startsWith(fs.realpathSync(root)+path.sep) || fs.lstatSync(file).isSymbolicLink())
    throw Error('Export path leaves its source: '+name);
  return file;
}
function validateSelection(selection) {
  if (selection.schema !== 1 || !/^[0-9a-f]{40}$/.test(selection.commit) || !selection.tag || !selection.toe_reference)
    throw Error('Guidance selection needs the actual source commit, tag and TOE reference');
  const docs = selection.documents;
  if (!Array.isArray(docs) || docs.length !== 4 || docs.map(d => d.id).sort().join(',') !== 'AGD-01,AGD-02,AGD-03,AGD-04')
    throw Error('Select the four distinct AGD handbooks');
  const sets = new Set();
  for (const doc of docs) {
    if (!doc.title?.trim() || !doc.version?.trim() || !Array.isArray(doc.sources) || !doc.sources.length || new Set(doc.sources).size !== doc.sources.length)
      throw Error(doc.id+': supply the reviewed version and explicit nonempty source list');
    const key = [...doc.sources].sort().join('\n');
    if (sets.has(key)) throw Error('Two handbooks cannot have identical page selections');
    sets.add(key);
    for (const name of doc.sources)
      if (typeof name !== 'string' || !/^docs\/docusaurus\/docs\/(?:02-election_managers|03-voters|05-reference)\/.+\.mdx?$/.test(name) || name.split('/').includes('..'))
        throw Error('Guidance source is outside the consumer roots: '+name);
  }
  return docs;
}
function checkStaticSource(source, name) {
  // Fence bodies are examples, not MDX; no client-rendered component may disappear silently.
  let fence; const text = [];
  for (const line of source.split('\n')) {
    const start = line.match(/^ {0,3}(`{3,}|~{3,})(.*)$/);
    if (!fence && start) {
      if (start[2].trim() === 'mermaid') throw Error(name+': Mermaid needs an explicit offline rendering adapter');
      fence = start[1];
    } else if (fence && new RegExp(`^ {0,3}${fence[0]}{${fence.length},}\\s*$`).test(line)) fence = null;
    else if (!fence) text.push(line);
  }
  if (fence) throw Error(name+': unclosed source fence');
  const body = text.join('\n').replace(/<!--[\s\S]*?-->/g, '');
  if (/<[A-Z][A-Za-z0-9_.]*(?:\s|\/?>)/.test(body)) throw Error(name+': MDX component needs an explicit offline rendering adapter');
  if (/Content will be added here soon|\bTODO\b|\bTBD\b/.test(body)) throw Error(name+': unfinished guidance placeholder');
}
function exportGuidance({site, selection, out}) {
  site = path.resolve(site); out = path.resolve(out);
  if (fs.existsSync(out)) throw Error('Output already exists; preserve it and choose a fresh destination');
  const docs = validateSelection(selection);
  const git = (...args) => execFileSync('git',['-C',site,...args],{encoding:'utf8'}).trim();
  const repo = git('rev-parse','--show-toplevel');
  const built = JSON.parse(fs.readFileSync(path.join(site,'.docusaurus/publication-build.json')));
  if (built.schema !== 1 || built.source_dirty || built.source_commit !== selection.commit || git('rev-parse','HEAD') !== selection.commit ||
      git('status','--porcelain','--','.',':(exclude).yarnrc')) throw Error('Guidance build requires the clean selected source revision');
  for (const [name, digest] of Object.entries(built.files)) {
    if (sha(fs.readFileSync(inside(repo,name))) !== digest || sha(execFileSync('git',['-C',repo,'cat-file','blob',`${selection.commit}:${name}`])) !== digest)
      throw Error('Source or renderer changed since the documentation build: '+name);
  }
  const build = path.join(site,'build');
  const prepared = []; const routeTargets = new Map(); const output = new Map(); const images = new Map();
  for (const doc of docs) for (const [index, source] of doc.sources.entries()) {
    const matches = (built.pages || []).filter(p => p.source === source);
    if (matches.length !== 1) throw Error('Selected source has no unique built route: '+source);
    const page = matches[0];
    const sourceText = fs.readFileSync(inside(repo,source),'utf8');
    checkStaticSource(sourceText,source);
    const bytes = fs.readFileSync(inside(build,page.output));
    if (sha(bytes) !== page.sha256) throw Error('Built article changed: '+source);
    const filename = `${doc.id}/${String(index+1).padStart(3,'0')}.html`;
    const item = {doc, page, filename, bytes, source_sha256:sha(sourceText)};
    prepared.push(item);
    if (!routeTargets.has(page.permalink.replace(/\/$/,''))) routeTargets.set(page.permalink.replace(/\/$/,''),[]);
    routeTargets.get(page.permalink.replace(/\/$/,'')).push(item);
  }
  const links = [];
  const wrap = (title, content) => '<!doctype html><html lang="en"><meta charset="utf-8"><title>'+escape(title)+'</title><style>'+ 
    'body{font:11pt/1.5 sans-serif;max-width:1100px;margin:2em auto;padding:1em;color:#111;background:white}'+
    'img{max-width:100%;height:auto}table{border-collapse:collapse}th,td{border:1px solid #aaa;padding:.4em}'+
    'pre{white-space:pre-wrap;overflow-wrap:anywhere}h1,h2,h3{break-after:avoid}a{color:#153b67}'+
    '.hash-link{display:none}aside{border-left:4px solid #777;padding-left:1em}@page{margin:15mm}'+
    '</style><body>'+content+'</body></html>';
  for (const item of prepared) {
    const {doc, page, filename} = item;
    const $ = cheerio.load(item.bytes);
    const article = $('article .markdown');
    if (article.length !== 1 || article.find('h1').first().text() !== page.title) throw Error('Rendered article identity differs: '+page.source);
    if (article.find('script, iframe, object, embed, video, audio, canvas, link, form, input, select, textarea, .katex, [role="tablist"], .docusaurus-mermaid-container').length)
      throw Error(page.source+': interactive or embedded content needs an explicit offline rendering adapter');
    article.find('button').remove();
    article.find('*').each((_, element) => {
      const css = ($(element).attr('style') || '') + (element.tagName === 'style' ? $(element).text() : '');
      if (/@import|expression\s*\(/i.test(css) || [...css.matchAll(/url\(\s*['"]?([^)'"\s]+)/gi)].some(m => !m[1].startsWith('#')))
        throw Error('Unsupported style resource in '+page.source);
      for (const [name,value] of Object.entries(element.attribs || {})) {
        if (/^on/i.test(name) || /^(?:srcdoc|srcset|action|formaction|poster)$/i.test(name) ||
            /^(?:href|xlink:href|src)$/i.test(name) && /^\s*(?:javascript|data|vbscript):/i.test(value)) throw Error('Active or unresolved content in '+page.source);
        if (name === 'src' && element.tagName !== 'img' || name === 'xlink:href' && !value.startsWith('#'))
          throw Error('Unsupported resource in '+page.source);
        if (name === 'href' && element.tagName !== 'a' && !value.startsWith('#')) throw Error('Unsupported SVG resource in '+page.source);
      }
    });
    article.find('img').each((_, image) => {
      const url = new URL($(image).attr('src'), 'https://guidance.invalid'+page.permalink);
      if (url.origin !== 'https://guidance.invalid' || !url.pathname.startsWith(built.base_url) || url.search || url.hash)
        throw Error('Image must be a local built asset: '+url.href);
      const name = decodeURIComponent(url.pathname.slice(built.base_url.length));
      const bytes = fs.readFileSync(inside(build,name)); const digest = sha(bytes);
      if (!built.assets?.[name] || built.assets[name] !== digest) throw Error('Image is unrecorded or changed: '+name);
      const ext = path.extname(name).toLowerCase();
      const valid = ext === '.png' ? bytes.subarray(0,8).equals(Buffer.from([137,80,78,71,13,10,26,10])) :
        ['.jpg','.jpeg'].includes(ext) ? bytes[0] === 255 && bytes[1] === 216 && bytes[2] === 255 :
        ext === '.gif' ? /^GIF8[79]a$/.test(bytes.subarray(0,6).toString()) :
        ext === '.webp' ? bytes.subarray(0,4).toString() === 'RIFF' && bytes.subarray(8,12).toString() === 'WEBP' :
        ext === '.avif' && bytes.subarray(4,8).toString() === 'ftyp' && /avif|avis/.test(bytes.subarray(8,32).toString());
      if (!valid) throw Error('Unsupported or malformed image type: '+name);
      const target = 'assets/'+digest+ext;
      images.set(target,bytes); $(image).attr('src',path.posix.relative(path.posix.dirname(filename),target));
      $(image).removeAttr('loading');
    });
    article.find('a[href]').each((_, anchor) => {
      const href = $(anchor).attr('href');
      const url = new URL(href,'https://guidance.invalid'+page.permalink);
      if (url.origin !== 'https://guidance.invalid') {
        if (!['https:','http:','mailto:'].includes(url.protocol)) throw Error('Unsupported link protocol: '+href);
        links.push({page:filename,href}); return;
      }
      const targets = routeTargets.get(url.pathname.replace(/\/$/,''));
      const target = targets?.find(t => t.doc.id === doc.id) || targets?.[0];
      if (!target || url.search) throw Error('Internal link leaves the selected guidance: '+href+' in '+page.source);
      if (url.hash) {
        const linked = cheerio.load(target.bytes);
        if (!linked('[id]').toArray().some(el => linked(el).attr('id') === decodeURIComponent(url.hash.slice(1))))
          throw Error('Selected guidance link has a missing fragment: '+href);
      }
      $(anchor).attr('href',path.posix.relative(path.posix.dirname(filename),target.filename)+url.hash);
    });
    const identity = `<p><a href="../index.html">Guidance index</a> — ${escape(doc.id)}, version ${escape(doc.version)}. ${escape(selection.toe_reference)}; release ${escape(selection.tag)}.</p>`;
    output.set(filename,Buffer.from(wrap(doc.title,identity+article.html())));
  }
  for (const doc of docs) {
    const pages = prepared.filter(p => p.doc.id === doc.id);
    const list = pages.map(p => `<li><a href="${path.posix.basename(p.filename)}">${escape(p.page.title)}</a></li>`).join('');
    output.set(doc.id+'/index.html',Buffer.from(wrap(doc.title,`<h1>${escape(doc.id)}: ${escape(doc.title)}</h1><p>Version ${escape(doc.version)}. ${escape(selection.toe_reference)}; release ${escape(selection.tag)}.</p><ol>${list}</ol>`)));
  }
  const index = docs.map(d => `<li><a href="${d.id}/index.html">${escape(d.id)}: ${escape(d.title)} — ${escape(d.version)}</a></li>`).join('');
  output.set('index.html',Buffer.from(wrap('Consumer guidance',`<h1>Consumer guidance</h1><p>${escape(selection.toe_reference)}; release ${escape(selection.tag)}.</p><ul>${index}</ul>`)));
  for (const [name,bytes] of images) output.set(name,bytes);
  const manifest = {schema:1,tag:selection.tag,commit:selection.commit,toe_reference:selection.toe_reference,
    documents:docs.map(doc => ({...doc,pages:prepared.filter(p => p.doc.id === doc.id).map(p => ({source:p.page.source,source_sha256:p.source_sha256,html:p.filename,title:p.page.title}))})),
    external_references:links, files:Object.fromEntries([...output].sort().map(([name,bytes]) => [name,sha(bytes)]))};
  // Validate everything before creating any output. The caller's run retains failed-build logs.
  fs.mkdirSync(out,{recursive:true});
  for (const [name,bytes] of output) {fs.mkdirSync(path.dirname(path.join(out,name)),{recursive:true});fs.writeFileSync(path.join(out,name),bytes);}
  fs.writeFileSync(path.join(out,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
  return manifest;
}
if (require.main === module) {
  const opts = {};
  for (let i=2;i<process.argv.length;i+=2) {
    const key = process.argv[i];
    if (!['--site','--selection','--out'].includes(key) || opts[key] || !process.argv[i+1]) throw Error('Invalid export option: '+key);
    opts[key]=process.argv[i+1];
  }
  if (Object.keys(opts).length !== 3) throw Error('Supply --site, --selection and --out');
  const result = exportGuidance({site:opts['--site'],out:opts['--out'],selection:JSON.parse(fs.readFileSync(opts['--selection']))});
  console.log(JSON.stringify({documents:result.documents.length,pages:result.documents.reduce((n,d)=>n+d.pages.length,0),files:Object.keys(result.files).length}));
}
module.exports = {exportGuidance, validateSelection};
