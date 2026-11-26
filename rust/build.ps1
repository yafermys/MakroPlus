param([switch]$Release, [switch]$Package)
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
# Keep linker inputs on an ASCII path (the repository path may contain non-ASCII characters).
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA 'MakroPlusBuild\target' }
# windows-gnu needs dlltool/ld from the toolchain's self-contained folder.
$rustlib = Join-Path $env:USERPROFILE '.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin'
if (Test-Path $rustlib) { $env:PATH = (Join-Path $rustlib 'self-contained') + ';' + $rustlib + ';' + $env:PATH }
Push-Location $here
try {
    $cargoArgs = @('build')
    if ($Release) { $cargoArgs += '--release' }
    # cargo reports progress on stderr; do not treat that as a PowerShell error.
    $ErrorActionPreference = 'Continue'
    & cargo @cargoArgs 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($code -ne 0) { throw 'cargo build failed' }
} finally { Pop-Location }
$profile = if ($Release) { 'release' } else { 'debug' }
$exe = Join-Path $env:CARGO_TARGET_DIR "$profile\MakroPlus.exe"
& (Join-Path $here 'tools\set-resources.ps1') -Exe $exe -Icon (Join-Path $here 'assets\MakroPlus.ico') -Manifest (Join-Path $here 'assets\MakroPlus.manifest')
if ($Package) { & (Join-Path $here 'tools\package.ps1') -Exe $exe }
Write-Host "Ready: $exe"
