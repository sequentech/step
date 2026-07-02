<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# NVD API Key Setup for Security Audits

The OWASP dependency-check Maven plugin uses the National Vulnerability Database (NVD) API to check for vulnerabilities. Without an API key, the update process can take **hours**. With an API key, it completes in **minutes**.

## Get an API Key

1. Go to https://nvd.nist.gov/developers/request-an-api-key
2. Fill out the form with your email
3. Check your email for the API key (usually arrives within minutes)

## Setup (Choose ONE method)

### Method 1: Environment Variable (Recommended)

Add to your shell configuration file on your **host machine** (not in the container):

**Linux/Mac** - Add to `~/.bashrc` or `~/.zshrc`:
```bash
export NVD_API_KEY="your-api-key-here"
```

**Windows** - Add to your user environment variables:
1. Search for "Environment Variables" in Windows
2. Click "New" under User variables
3. Variable name: `NVD_API_KEY`
4. Variable value: `your-api-key-here`

Then **rebuild your devcontainer** or restart VS Code.

### Method 2: Secret File

Create a file on your **host machine**:

**Linux/Mac**:
```bash
mkdir -p ~/.secrets
echo "your-api-key-here" > ~/.secrets/nvd-api-key
chmod 600 ~/.secrets/nvd-api-key
```

**Windows**:
```powershell
New-Item -ItemType Directory -Force -Path $env:USERPROFILE\.secrets
Set-Content -Path $env:USERPROFILE\.secrets\nvd-api-key -Value "your-api-key-here"
```

Then **rebuild your devcontainer** or restart VS Code.

### Method 3: Direct File (Alternative)

Create `~/.nvd-api-key` on your **host machine**:

**Linux/Mac**:
```bash
echo "your-api-key-here" > ~/.nvd-api-key
chmod 600 ~/.nvd-api-key
```

**Windows**:
```powershell
Set-Content -Path $env:USERPROFILE\.nvd-api-key -Value "your-api-key-here"
```

## Using Devcontainers

This project uses [Visual Studio Code Dev Containers](https://code.visualstudio.com/docs/devcontainers/containers) to provide a consistent development environment. The devcontainer configuration automatically handles the NVD API key setup for you.

### How Devcontainers Work with NVD API Key

The `.devcontainer/devcontainer.json` configuration includes settings that:

1. **Mount your home directory** - This gives the container access to your API key files
2. **Pass environment variables** - If you set `NVD_API_KEY` on your host, it's automatically available in the container
3. **Preserve permissions** - File-based API keys maintain their security settings

### Rebuilding the Devcontainer

After setting up your NVD API key, you need to rebuild the devcontainer for changes to take effect:

1. Open the Command Palette in VS Code (`Ctrl+Shift+P` or `Cmd+Shift+P`)
2. Search for and select: **Dev Containers: Rebuild Container**
3. Wait for the container to rebuild and reopen

Alternatively, you can:
- Close and reopen VS Code
- Use the command: **Dev Containers: Reopen in Container**

### Troubleshooting Devcontainers

If your API key isn't available in the devcontainer:

1. **Verify on host machine** - Check that the key exists outside the container
2. **Check mounts** - Ensure your home directory is mounted (see `.devcontainer/devcontainer.json`)
3. **Rebuild completely** - Try **Dev Containers: Rebuild Container Without Cache**
4. **Check environment** - Inside the container, run `echo $NVD_API_KEY` or `cat ~/.nvd-api-key`

## Verify Setup

After setting up the API key and rebuilding your devcontainer, verify it's working:

```bash
echo $NVD_API_KEY
# Should output your API key if using environment variable method

# Or check for the file
cat ~/.nvd-api-key
# or
cat ~/.secrets/nvd-api-key
```

## How It Works

The security audit script (`scripts/tasks/security/audit-dependencies.sh`) automatically detects your API key from these sources (in order):

1. `$NVD_API_KEY` environment variable
2. `~/.nvd-api-key` file
3. `~/.secrets/nvd-api-key` file

The devcontainer configuration mounts your home directory, so these files/variables are automatically available inside the container.

## Security Note

⚠️ **NEVER commit your API key to the repository!**

The methods above keep your key on your local machine only. The key is:
- Not added to any git-tracked files
- Only available in your personal devcontainer
- Automatically available when you rebuild/restart

## Usage

Once configured, the security audit will automatically use your API key:

```bash
./scripts/tasks/security/audit-dependencies.sh
```

Or run the VS Code task: `security.audit.dependencies`

You should see:
```
☕ Auditing: keycloak-extensions
  Using NVD API key for faster updates...
```

Instead of:
```
⚠ No NVD API key found - this will be SLOW
```
