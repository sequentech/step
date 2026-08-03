# Run Verificatum's verifier against proofs braid produced.
#
# The interop tests are `#[ignore]`d because they shell out to a JVM, and they
# need four environment variables. This script works out three of them from the
# repo, creates the fourth (an initialised random source, which `vmnv` refuses to
# start without even though verification consumes no randomness), and runs the
# tests.
#
# Prerequisites: a JDK on PATH, or -Java pointing at one. Nothing else — the
# Verificatum jars are in the tree and no Java build is required.
#
#   .\vmnv.ps1                     # the three-party chain (the default)
#   .\vmnv.ps1 -All                # every interop test, including -mix
#   .\vmnv.ps1 -Test vmnv_accepts_a_braid_mixing_proof
#
# See testdata/verificatum/README.md for what the tests check and how the
# reference corpus they use was generated.

[CmdletBinding()]
param(
    # Substring filter passed to the test binary. The default is the multi-party
    # shuffle chain, which needs no reference corpus — it emits its own.
    [string] $Test = "vmnv_accepts_a_three_party_chain",

    # Run every test in the file instead of a single one.
    [switch] $All,

    # Path to java.exe. Defaults to `java` from PATH.
    [string] $Java,

    # Where to keep the generated random source and seed. These are throwaway;
    # delete the directory to regenerate them.
    [string] $StateDir = (Join-Path $env:TEMP "braid-vmnv")
)

# Native tools here write ordinary progress to stderr -- `java -version` puts its
# whole banner there, and so does cargo. Under `$ErrorActionPreference = "Stop"`
# (which a caller's profile may well set, and which this script inherits)
# PowerShell promotes that into a terminating NativeCommandError, so the script
# dies on a successful `java -version`. Both settings below turn that off; every
# real failure is checked explicitly via Test-Path or $LASTEXITCODE instead.
$ErrorActionPreference = "Continue"
# Only meaningful on PowerShell 7.2+, where it defaults to true from 7.4.
# Assigning it on Windows PowerShell 5.1 is harmless.
$PSNativeCommandUseErrorActionPreference = $false

$repoRoot = $PSScriptRoot
$jarDir = Join-Path $repoRoot "crates\braid\verificatum"
$vmnJar = Join-Path $jarDir "verificatum-vmn\verificatum-vmn-3.1.0.jar"
$vcrJar = Join-Path $jarDir "verificatum-vcr\verificatum-vcr-3.1.0.jar"

# --- prerequisites --------------------------------------------------------

if ($Java) {
    if (-not (Test-Path $Java)) {
        Write-Host "No java at $Java" -ForegroundColor Red
        exit 1
    }
} else {
    $found = Get-Command java -ErrorAction SilentlyContinue
    if (-not $found) {
        Write-Host "No java on PATH. Install a JDK, or pass -Java <path\to\java.exe>." -ForegroundColor Red
        exit 1
    }
    $Java = $found.Source
}

foreach ($jar in @($vmnJar, $vcrJar)) {
    if (-not (Test-Path $jar)) {
        Write-Host "Missing $jar" -ForegroundColor Red
        Write-Host "The Verificatum jars are expected under crates\braid\verificatum." -ForegroundColor Red
        exit 1
    }
}

Write-Host "java: $Java" -ForegroundColor DarkGray
$banner = (& $Java -version 2>&1 | Out-String) -split "`r?`n" | Where-Object { $_ } | Select-Object -First 1
if ($LASTEXITCODE -ne 0) {
    Write-Host "$Java is not a working java (exit $LASTEXITCODE)." -ForegroundColor Red
    exit 1
}
Write-Host "      $banner" -ForegroundColor DarkGray

# --- random source --------------------------------------------------------
#
# `vog -rndinit RandomDevice /dev/urandom` is the documented way to do this, but
# /dev/urandom does not exist on Windows. The portable alternative is a seeded
# PRG: write 512 random bytes to a file and hand them to PRGHeuristic.

$randomSource = Join-Path $StateDir "random_source"
$randomSeed = Join-Path $StateDir "random_seed"
$classpath = "$vmnJar;$vcrJar"

function Invoke-Vog {
    param([string[]] $VogArgs)
    & $Java -cp $classpath com.verificatum.ui.gen.GeneratorTool `
        vog ":VERIFICATUM_VOG_BUILTIN" $randomSource $randomSeed @VogArgs
    if ($LASTEXITCODE -ne 0) { throw "vog failed: $VogArgs" }
}

if ((Test-Path $randomSource) -and (Test-Path $randomSeed)) {
    Write-Host "Using the random source in $StateDir" -ForegroundColor DarkGray
} else {
    Write-Host "Initialising a random source in $StateDir ..." -ForegroundColor Cyan
    New-Item -ItemType Directory -Force -Path $StateDir -ErrorAction Stop | Out-Null

    # 512 bytes of seed material for the PRG.
    $seedFile = Join-Path $StateDir "seed_material"
    $bytes = [byte[]]::new(512)
    [System.Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
    [System.IO.File]::WriteAllBytes($seedFile, $bytes)

    # The descriptor of the hash function the PRG is built from, then the PRG
    # itself. `vog` writes $randomSource and $randomSeed as a side effect.
    $descriptor = (Invoke-Vog @("-gen", "HashfunctionHeuristic", "SHA-256")).Trim()
    Invoke-Vog @("-seed", $seedFile, "-rndinit", "PRGHeuristic", $descriptor) | Out-Null

    if (-not ((Test-Path $randomSource) -and (Test-Path $randomSeed))) {
        Write-Host "vog did not write the random source or seed." -ForegroundColor Red
        exit 1
    }
    Write-Host "Random source initialised." -ForegroundColor DarkGray
}

# --- run ------------------------------------------------------------------

$env:VMNV_JAVA = $Java
$env:VMNV_JAR_DIR = $jarDir
$env:VMNV_RANDOM_SOURCE = $randomSource
$env:VMNV_RANDOM_SEED = $randomSeed

# --release because the DKG, shuffle and decryption are compute-intensive; a
# debug build turns seconds into minutes.
$cargoArgs = @(
    "test", "--release",
    "-p", "vsvmn",
    # Every test file, not one: the interop tests are split by direction
    # (they_verify_ours, we_verify_theirs) and the transcript checks live
    # alongside them. --include-ignored so the fast tests run too.
    "--", "--include-ignored", "--nocapture"
)
if (-not $All) { $cargoArgs += $Test }

if ($All) {
    Write-Host "Running every vmnv interop test..." -ForegroundColor Green
} else {
    Write-Host "Running $Test ..." -ForegroundColor Green
}

cargo @cargoArgs
$code = $LASTEXITCODE

if ($code -eq 0) {
    Write-Host "vmnv accepted braid's proofs." -ForegroundColor Green
} else {
    # Worth stating explicitly: a passing exit code from `vmnv` itself is not
    # sufficient evidence for a shuffling proof -- it exits 0 on proofs it has
    # rejected. The tests check the transcript too, so trust *their* verdict.
    Write-Host "Interop tests failed (exit $code)." -ForegroundColor Red
}

exit $code
