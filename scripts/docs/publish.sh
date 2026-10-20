#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
# Call from the trusted source checkout. The separate site checkout contains only output.
site=$1
shift
metadata=$(mktemp)
trap 'rm -f "$metadata"' EXIT
# Any API/auth/pagination failure stops the operation before files are removed.
gh api --paginate --slurp "repos/$GITHUB_REPOSITORY/pulls?state=open&per_page=100" > "$metadata"
python3 scripts/docs/preview_retention.py --site "$site" \
  --policy scripts/docs/preview-policy.json --open-prs "$metadata" "$@"
git -C "$site" add --all
if git -C "$site" diff --cached --quiet; then
  echo "No generated output changes"
  exit 0
fi
python3 - "$site" <<'PYTHON'
import json, os, sys, uuid
from pathlib import Path
sys.path.insert(0, 'scripts/docs')
from preview_retention import size
site = Path(sys.argv[1])
(site / '.pages-deployment.json').write_text(json.dumps({
    'source_commit': os.environ['GITHUB_SHA'], 'publication_id': uuid.uuid4().hex
}) + '\n')
limit = json.loads(Path('scripts/docs/preview-policy.json').read_text())['max_site_bytes']
if size(site) > limit:
    sys.exit('Publication marker would exceed the 900 MB guardrail; refusing push')
PYTHON
git -C "$site" add .pages-deployment.json
git -C "$site" -c user.name='github-actions[bot]' \
  -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
  -c commit.gpgsign=false commit -m 'docs: publish generated output with bounded preview retention'
# A normal fast-forward push rejects any concurrent writer; never force or retry stale cleanup.
git -C "$site" push origin HEAD:doc-previews
sha=$(git -C "$site" rev-parse HEAD)
echo "sha=$sha" >> "$GITHUB_OUTPUT"
