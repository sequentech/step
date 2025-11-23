<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# CLA (Contributor License Agreement) Setup

This directory contains the Contributor License Agreement (CLA) setup for the Sequent Voting Platform project.

## Files

- **CLA.md** - The full text of the Contributor License Agreement
- **signatures.json** - Storage for CLA signatures (managed automatically by the CLA bot)
- **README.md** - This file

## How It Works

The CLA Assistant GitHub Action automatically:

1. Checks if a PR author has signed the CLA
2. Comments on the PR if signature is needed
3. Updates the PR status based on CLA signature
4. Stores signatures in `signatures.json` on the `cla-signatures` branch

## Setup Instructions for Maintainers

### 1. Create a Personal Access Token (PAT)

The CLA bot needs a GitHub Personal Access Token with `repo` scope to:
- Create and update the `cla-signatures` branch
- Update the signatures.json file
- Comment on pull requests

**To create the PAT:**

1. Go to GitHub Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Click "Generate new token (classic)"
3. Give it a descriptive name like "CLA Bot Token"
4. Select the `repo` scope (full control of private repositories)
5. Set expiration as appropriate (1 year recommended, with calendar reminder to renew)
6. Generate the token and copy it

### 2. Add the PAT as a Repository Secret

1. Go to the repository Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Name: `CLA_PAT`
4. Value: Paste the PAT you created
5. Click "Add secret"

### 3. Create the CLA Signatures Branch

The bot will automatically create the `cla-signatures` branch on first use, but you can create it manually:

```bash
git checkout --orphan cla-signatures
git rm -rf .
cp .github/cla/signatures.json .
git add signatures.json
git commit -m "Initialize CLA signatures"
git push origin cla-signatures
git checkout main
```

## For Contributors

Contributors will see a comment on their first PR asking them to sign the CLA. They simply need to comment:

```
I have read the CLA Document and I hereby sign the CLA
```

The bot will then record their signature and update the PR status.

## Customization

To customize the CLA workflow:

1. Edit `.github/workflows/cla.yml` to change messages or behavior
2. Edit `.github/cla/CLA.md` to modify the CLA terms (consult legal counsel first!)
3. Update the allowlist in the workflow to exclude specific accounts

## Troubleshooting

### Bot not responding
- Check that the `CLA_PAT` secret is set correctly
- Verify the PAT has `repo` scope
- Check workflow runs in the Actions tab

### Signatures not saving
- Ensure the `cla-signatures` branch exists
- Verify the PAT has push access to the repository
- Check the signatures.json file format is valid JSON

### Contributors can't sign
- Make sure they comment the exact phrase (case-sensitive)
- Check that the PR is from a fork, not a branch in the same repo
- Verify they're commenting on the PR, not on individual commits

## Support

For questions or issues with the CLA setup:
- Open an issue in the repository
- Contact legal@sequentech.io
- Join our [Discord community](https://discord.gg/WfvSTmcdY8)
