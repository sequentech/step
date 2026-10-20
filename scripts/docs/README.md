<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Pages publication and preview retention

The Pages site has a 900,000,000-byte uncompressed output guardrail. Every
publication applies retention before committing output to `doc-previews`.
The actual Pages deployment must succeed, and HTTP responses must match the
published marker, target page and protected policy/security files. A successful
Docusaurus build or branch push alone does not count as publication success.

`preview-policy.json` pins the 16 previews protected during the October 2026
Pages repair. PRs **2335 and 3949 must remain pinned**, even after closure.
Main, all releases, pinned preview trees and unrelated files are never removed
by retention. Changes to the pinned list require a reviewed maintenance PR.

Up to four additional previews are retained, prioritizing the currently
publishing PR and then recently updated open PRs. Unpinned previews for closed
PRs, or PRs with no activity for 14 days, are removed. The oldest eligible
unpinned previews are also removed when needed to stay under the size cap.
If protected content plus the new publication exceeds the cap, publication
fails before any push; it never sacrifices protected content to meet the cap.

Cleanup runs daily, manually, and when any PR closes. It uses trusted tooling
from the default branch and does not build the closing PR. Both workflows use
the same concurrency group; all writes are normal fast-forward commits.
A concurrent external writer causes the push to fail so the inventory can be
reassessed, rather than force-pushed or automatically replayed.

For a dry run against an isolated `doc-previews` checkout:

```bash
gh api --paginate --slurp 'repos/sequentech/step/pulls?state=open&per_page=100' > /tmp/open-prs.json
python3 scripts/docs/preview_retention.py --site /path/to/generated-checkout \
  --policy scripts/docs/preview-policy.json --open-prs /tmp/open-prs.json --dry-run
python3 -m unittest discover -s scripts/docs -p 'test_*.py' -v
```

GitHub Pages must retain its existing branch source (`doc-previews`, root).
The existing PAT is used only for the output checkout/push so the Pages
workflow is triggered. API reads use the job token. No new permissions, token
scopes, Git history rewriting or production deployment settings are needed.
