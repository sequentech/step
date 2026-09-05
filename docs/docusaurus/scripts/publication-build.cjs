// Identify the inputs and outputs used for the served/public-master comparison.
// This record stays in .docusaurus; it is not an additional public website page.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {execFileSync} = require('node:child_process');
const sha = data => crypto.createHash('sha256').update(data).digest('hex');
module.exports = function publicationBuild(context) {
  const site = context.siteDir;
  const git = (...args) => execFileSync('git', ['-C', site, ...args], {encoding:'utf8'}).trim();
  const repo = git('rev-parse', '--show-toplevel');
  const prefix = path.relative(repo, site);
  const snapshot = () => ({
    source_commit:git('rev-parse', 'HEAD'),
    source_dirty:!!git('status', '--porcelain', '--', '.', ':(exclude).yarnrc'),
    local_yarnrc_sha256:fs.existsSync(path.join(site, '.yarnrc')) ? sha(fs.readFileSync(path.join(site, '.yarnrc'))) : null,
    files:Object.fromEntries(git('ls-files', '-z').split('\0').filter(Boolean).sort().map(name => [
      `${prefix}/${name}`, sha(fs.readFileSync(path.join(site, name))),
    ])),
  });
  const before = snapshot();
  return {
    name:'publication-build',
    async postBuild({outDir}) {
      if (JSON.stringify(before) !== JSON.stringify(snapshot()))
        throw new Error('Public documentation inputs changed during the build');
      const paths = {
        'vulnerability-disclosure-policy': ['docs/technology/vulnerability-disclosure-policy.html', 'docs/06-technology/06-vulnerability-disclosure-policy.md'],
        'pgp-key': ['security/pgp-key.txt', 'static/security/pgp-key.txt'],
        'security.txt': ['.well-known/security.txt', 'static/.well-known/security.txt'],
        'csaf-provider-metadata': ['.well-known/csaf/provider-metadata.json', 'static/.well-known/csaf/provider-metadata.json'],
      };
      const artifacts = {};
      for (const [name, [output, source]] of Object.entries(paths)) {
        let file = output;
        if (!fs.existsSync(path.join(outDir, file)) && file.endsWith('.html'))
          file = file.slice(0, -5) + '/index.html';
        artifacts[name] = {source:`${prefix}/${source}`, output:file,
          sha256:sha(fs.readFileSync(path.join(outDir, file)))};
      }
      fs.writeFileSync(path.join(site, '.docusaurus/publication-build.json'), JSON.stringify({
        schema:1, base_url:context.siteConfig.baseUrl, ...before, artifacts,
      }, null, 2) + '\n');
    },
  };
};
