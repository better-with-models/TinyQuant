# End-to-end CLI smoke chain: info -> codec train -> compress -> decompress
# -> verify -> corpus search. Mirrors scripts/cli-smoke.sh.
#
# Usage: pwsh scripts/cli-smoke.ps1 [-Bin path\to\tinyquant.exe]
# Exit: 0 on success, non-zero on any failure.

[CmdletBinding()]
param(
    [string]$Bin = $(if ($env:TINYQUANT_BIN) { $env:TINYQUANT_BIN } else { "target\release\tinyquant.exe" })
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Bin)) {
    throw "tinyquant binary not found at $Bin"
}

$Rows  = 1024
$Cols  = 32
$Bit   = 4
$Seed  = 7

$Tmp       = Join-Path $env:TEMP ("tinyquant-smoke-" + [guid]::NewGuid().ToString("N"))
$null      = New-Item -ItemType Directory -Path $Tmp

try {
    $Input     = Join-Path $Tmp "train.f32.bin"
    $Codebook  = Join-Path $Tmp "codebook.tqcb"
    $Config    = Join-Path $Tmp "config.json"
    $Corpus    = Join-Path $Tmp "corpus.tqcv"
    $Decomp    = Join-Path $Tmp "decompressed.f32.bin"
    $Query     = Join-Path $Tmp "query.f32.bin"

    # Use a deterministic Gaussian generator — raw RNG bytes interpreted
    # as f32 hit NaN / Inf bit patterns and break the downstream MSE
    # check. The PowerShell rand path mirrors the bash script's Python
    # shim for consistency across the smoke harness.
    function New-GaussianFloats {
        param([int]$Count, [int]$Seed)
        $rng = [System.Random]::new($Seed)
        $out = [byte[]]::new($Count * 4)
        for ($i = 0; $i -lt $Count; $i++) {
            $u1 = [Math]::Max($rng.NextDouble(), 1e-12)
            $u2 = $rng.NextDouble()
            $g  = [Math]::Sqrt(-2.0 * [Math]::Log($u1)) * [Math]::Cos(2.0 * [Math]::PI * $u2)
            $bytes = [System.BitConverter]::GetBytes([float]$g)
            [Buffer]::BlockCopy($bytes, 0, $out, $i * 4, 4)
        }
        return ,$out
    }
    [System.IO.File]::WriteAllBytes($Input, (New-GaussianFloats -Count ($Rows * $Cols) -Seed 0))
    [System.IO.File]::WriteAllBytes($Query, (New-GaussianFloats -Count $Cols -Seed 1))

    Write-Host "== info =="
    & $Bin info

    Write-Host "== codec train =="
    & $Bin codec train `
        --input $Input --rows $Rows --cols $Cols `
        --bit-width $Bit --seed $Seed --residual `
        --format f32 --output $Codebook --config-out $Config

    Write-Host "== codec compress =="
    & $Bin codec compress `
        --input $Input --rows $Rows --cols $Cols `
        --config-json $Config --codebook $Codebook `
        --output $Corpus --format f32

    Write-Host "== codec decompress =="
    & $Bin codec decompress `
        --input $Corpus --config-json $Config --codebook $Codebook `
        --output $Decomp --format f32

    Write-Host "== verify codebook + corpus =="
    & $Bin verify $Codebook
    & $Bin verify $Corpus

    Write-Host "== corpus search =="
    & $Bin corpus search `
        --corpus $Corpus --query $Query `
        --codebook $Codebook --config-json $Config `
        --top-k 3 --format json

    Write-Host "== MSE check =="
    $python = (Get-Command python3 -ErrorAction SilentlyContinue) ?? (Get-Command python -ErrorAction SilentlyContinue)
    if ($python) {
        $mseScript = @"
import struct, sys
def load(p):
    with open(p, 'rb') as f:
        data = f.read()
    return struct.unpack(f'{len(data) // 4}f', data)
orig = load(sys.argv[1])
back = load(sys.argv[2])
assert len(orig) == len(back), f'length mismatch {len(orig)} vs {len(back)}'
sq = sum((a - b) ** 2 for a, b in zip(orig, back))
mse = sq / len(orig)
print(f'mse={mse:.6f}')
assert mse < 1.0, f'mse {mse} too large'
"@
        $scriptPath = Join-Path $Tmp "mse.py"
        Set-Content -Path $scriptPath -Value $mseScript -Encoding ASCII
        & $python.Source $scriptPath $Input $Decomp
    } else {
        $s1 = (Get-Item $Input).Length
        $s2 = (Get-Item $Decomp).Length
        if ($s1 -ne $s2) {
            throw "byte-length mismatch: $s1 != $s2"
        }
        Write-Host "byte-length check passed (python not available for MSE)"
    }

    Write-Host "== smoke ok =="
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
