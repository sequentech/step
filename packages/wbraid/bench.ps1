# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# Controlled benchmark run for the MSM optimization campaign (Windows twin of
# bench.sh).
#
# Run this with the machine otherwise quiesced (no other heavy processes) to
# get authoritative numbers. It builds everything FIRST (untimed), then runs
# the timed benchmarks, teeing everything to a timestamped results file under
# bench-results/.
#
# Usage (from packages/wbraid, in PowerShell):
#   .\bench.ps1
#   .\bench.ps1 -Reps 5
#
# The criterion benches (parallel_tradeoff, msm_strategy) self-calibrate; the
# targets example runs over a fixed cell grid, -Reps times each, so the median
# can be taken. Override the grid with -Cells ("N:W").

[CmdletBinding()]
param(
    [int]$Reps = 3,
    [string[]]$Cells = @('1000:2', '10000:2', '10000:5', '100000:2', '100000:5'),
    # 0 skips the criterion guidance benches and runs only the targets grid
    # (bench.sh's GUIDANCE=0).
    [int]$Guidance = 1
)

# cargo writes progress and a harmless "patch not used" warning to stderr with
# exit 0. Do NOT let native-command stderr become a terminating error (that is
# what `$ErrorActionPreference = 'Stop'` + `2>&1` would do); gate on the exit
# code instead.
$ErrorActionPreference = 'Continue'
$PSNativeCommandUseErrorActionPreference = $false

Set-Location $PSScriptRoot

$resultsDir = Join-Path $PSScriptRoot 'bench-results'
New-Item -ItemType Directory -Force -Path $resultsDir | Out-Null
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$out = Join-Path $resultsDir "bench-$stamp.txt"

function Log([string]$msg) { $msg | Tee-Object -FilePath $out -Append }

# Run a build/prep step untimed: discard its output unless it actually fails,
# in which case surface the output and stop.
function Build-Step([string[]]$cargoArgs) {
    $log = & cargo @cargoArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        $log | Out-Host
        throw "build step failed (exit $LASTEXITCODE): cargo $($cargoArgs -join ' ')"
    }
}

# Criterion prints results to stdout; keep only the benchmark/time lines,
# dropping the "Warming up / Collecting / Analyzing" progress chatter (and
# cargo's stderr, via 2>$null).
function Run-Criterion([string]$name) {
    cargo bench -p vsc --bench $name 2>$null |
        Where-Object { $_ -match 'Benchmarking|time:' -and $_ -notmatch 'Warming|Collecting|Analyzing' } |
        Tee-Object -FilePath $out -Append
}

$commit = (git rev-parse --short HEAD 2>$null)
Log "# wbraid benchmark run $stamp"
Log "# commit: $commit"
Log "# host cores: $env:NUMBER_OF_PROCESSORS"
Log ''

# --- Build everything first (NOT timed) -------------------------------------
Write-Host 'building (untimed)...'
Build-Step @('build', '--release', '-p', 'vsc', '--examples')
if ($Guidance -eq 1) {
    Build-Step @('bench', '-p', 'vsc', '--bench', 'parallel_tradeoff', '--no-run')
    Build-Step @('bench', '-p', 'vsc', '--bench', 'msm_strategy', '--no-run')
}
Write-Host 'build done; starting timed run.'

# --- Criterion micro-benches (statistical; guidance, not snapshot) ------------
if ($Guidance -eq 1) {
    Log '## parallel_tradeoff (criterion)'
    Run-Criterion 'parallel_tradeoff'
    Log ''

    Log '## msm_strategy (criterion)'
    Run-Criterion 'msm_strategy'
    Log ''
} else {
    Log '# guidance benches skipped (-Guidance 0)'
    Log ''
}

# --- Top-level target snapshot (absolute, current tree) ---------------------
$targetsExe = Join-Path $PSScriptRoot 'target\release\examples\targets.exe'

Log '## targets  (count,width,prove,verify,partial_decrypt,combine,ny_strip,sizeof,ser)'
foreach ($cell in $Cells) {
    $n, $w = $cell -split ':'
    for ($r = 1; $r -le $Reps; $r++) {
        & $targetsExe $n $w 2>$null | Tee-Object -FilePath $out -Append
    }
}
Log ''

Log "# done -> $out"
