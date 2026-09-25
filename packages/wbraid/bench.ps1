# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# Local benchmark run for wbraid's cryptography (PowerShell; bash twin: bench.sh,
# same grids, same output).
#
# What it measures (the two measurement programs in crates/vsc/examples/):
#
#   targets  The five cryptographic stages of a tally, each timed alone, for one
#            cell "N:W" -- N ciphertexts of width W: shuffle prove, shuffle
#            verify, one trustee's partial decryption, combine (verify every
#            partial and interpolate), and the first mix's Naor-Yung
#            verify-and-strip. One CSV line per run, milliseconds:
#              count,width,prove,verify,partial_decrypt,combine,ny_strip,sizeof,ser
#   tally    The global target: one whole tally replayed with real data flow for
#            a quorum of Q trustees, cell "N:W:Q", composed into its
#            critical-path latency T (what the tally takes end to end, each
#            trustee on its own machine) and V (the external verifier's path).
#            One CSV line per run, milliseconds, stage breakdown on stderr:
#              count,width,quorum,ser,strip_prod,strip_ver,prove,verify,partial,combine,ser_ms,t,v
#
# Each cell runs REPS times; read the median. Results are appended to a
# timestamped file under bench-results/ (git-ignored). Everything is built
# first, untimed; run with the machine otherwise idle.
#
# These are the local tools. Authoritative numbers come from a fixed EC2
# reference machine instead -- bench-ec2.sh, documented in BENCH-EC2.md -- and
# PERFORMANCE.md (Status) is generated from such a session.
#
# Usage, from packages/wbraid in PowerShell:
#
#   .\bench.ps1 -Guidance 0                            # targets + tally over the default grids
#   .\bench.ps1 -Guidance 0 -Cells '100000:2' -TallyCells '100000:2:3' -Reps 5
#   .\bench.ps1 -Guidance 0 -TallyCells @()            # targets only
#   .\bench.ps1 -Guidance 0 -Cells @() -TallyCells '1000000:1:2'   # tally only
#   .\bench.ps1                                        # also the criterion guidance benches (slow)
#
# Or run the programs directly, once built (cargo build --release -p vsc --examples):
#
#   .\target\release\examples\targets.exe 100000 2          # N W
#   .\target\release\examples\tally.exe 100000 2 3          # N W Q
#   .\target\release\examples\tally.exe 100000 2 3 --ser    # + encoding/decoding of every posted message
#
# Supported widths W: 1 2 3 5 10; quorums Q (tally): 2 3 4 5 7.
#
# -Guidance 1 (the default) first runs the criterion benches parallel_tradeoff
# and msm_strategy, which steer implementation choices and are recorded in
# PERFORMANCE.md; they are not part of a snapshot.

[CmdletBinding()]
param(
    [int]$Reps = 3,
    [string[]]$Cells = @('1000:2', '10000:2', '10000:5', '100000:2', '100000:5'),
    # The global target's "N:W:Q" cells (bench.sh's TALLY_CELLS); @() skips it.
    [string[]]$TallyCells = @('10000:2:3', '100000:2:3'),
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

# --- The global target: one tally's critical path, per N:W:Q cell -----------
if ($TallyCells.Count -gt 0) {
    $tallyExe = Join-Path $PSScriptRoot 'target\release\examples\tally.exe'
    Log '## tally  (count,width,quorum,ser,strip_prod,strip_ver,prove,verify,partial,combine,ser_ms,t,v)'
    foreach ($cell in $TallyCells) {
        $n, $w, $q = $cell -split ':'
        for ($r = 1; $r -le $Reps; $r++) {
            & $tallyExe $n $w $q 2>$null | Tee-Object -FilePath $out -Append
        }
    }
    Log ''
}

Log "# done -> $out"
