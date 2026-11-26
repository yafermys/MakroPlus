# Build the portable folder: MakroPlus.exe + optional local model runtime + docs.
param([Parameter(Mandatory)][string]$Exe, [string]$OutRoot)
$ErrorActionPreference = 'Stop'
$repo = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$version = (Select-String -Path (Join-Path $repo 'rust\Cargo.toml') -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
if (-not $OutRoot) { $OutRoot = Join-Path $repo 'dist-rs' }
$out = Join-Path $OutRoot "MakroPlus-$version-win-x64"
if (Test-Path $out) { Remove-Item $out -Recurse -Force }
New-Item -ItemType Directory -Force (Join-Path $out 'models') | Out-Null
Copy-Item $Exe (Join-Path $out 'MakroPlus.exe')
$runtime = Join-Path $repo 'runtime\llama'
if (Test-Path (Join-Path $runtime 'llama-server.exe')) {
    New-Item -ItemType Directory -Force (Join-Path $out 'runtime\llama') | Out-Null
    # Only what llama-server needs: the server, its DLLs and licences.
    Get-ChildItem $runtime -File | Where-Object { $_.Name -like 'llama-server*' -or $_.Extension -eq '.dll' -and $_.Name -notlike 'llama-*-impl.dll' -or $_.Name -like 'LICENSE*' } | Copy-Item -Destination (Join-Path $out 'runtime\llama')
    Copy-Item (Join-Path $runtime 'llama-server-impl.dll') (Join-Path $out 'runtime\llama') -ErrorAction SilentlyContinue
}
Copy-Item (Join-Path $repo 'packaging\models\README.txt') (Join-Path $out 'models\README.txt') -ErrorAction SilentlyContinue
foreach ($f in 'LICENSE', 'THIRD_PARTY_NOTICES.md') { Copy-Item (Join-Path $repo $f) $out -ErrorAction SilentlyContinue }
Set-Content -Path (Join-Path $out 'Install.cmd') -Encoding ASCII -Value "@echo off`r`n""%~dp0MakroPlus.exe"" --install-ui`r`n"
$readme = @"
Makro+ $version

TASINABILIR KULLANIM
  MakroPlus.exe dosyasini acmaniz yeterli. Kurulum gerekmez.
  Ayarlar %LOCALAPPDATA%\MakroPlus klasorunde tutulur. Bu klasorun yanina
  portable.txt koyarsaniz tum veriler buradaki data klasorunde kalir.
  Makro+ 2.x ayarlariniz ilk acilista otomatik iceri aktarilir (eskileri degismez).

BILGISAYARA YUKLEME (ISTEGE BAGLI)
  Install.cmd dosyasini calistirin veya uygulamada Ayarlar > Bu bilgisayara yukle.
  Makro+ Baslat menusune ve Ayarlar > Uygulamalar > Yuklu uygulamalar listesine eklenir.
  Yonetici izni gerekmez. Kaldirmak icin Ayarlar > Uygulamalar'i kullanin.

YAPAY ZEKA
  Yerel model icin bir .gguf dosyasini models klasorune kopyalayin,
  veya uygulamada Yapay zeka modeli sayfasindan bir API sunucusu girin.

GUNCELLEMELER
  Makro+ her acilista GitHub'daki son surumu denetler (Ayarlar'dan kapatilabilir).
"@
Set-Content -Path (Join-Path $out 'BENIOKU.txt') -Value $readme -Encoding UTF8
$size = (Get-ChildItem $out -Recurse -File | Measure-Object Length -Sum).Sum
"Package: $out ($([math]::Round($size/1MB,1)) MB)"
# Release archives: full (with the local model engine) and lite (app only).
$upload = Join-Path $OutRoot 'upload'
New-Item -ItemType Directory -Force $upload | Out-Null
$full = Join-Path $upload "MakroPlus-$version-win-x64.zip"
$lite = Join-Path $upload "MakroPlus-$version-win-x64-lite.zip"
Remove-Item $full, $lite -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $out '*') -DestinationPath $full -CompressionLevel Optimal
$liteDir = Join-Path $OutRoot "lite\MakroPlus-$version-win-x64-lite"
if (Test-Path $liteDir) { Remove-Item $liteDir -Recurse -Force }
New-Item -ItemType Directory -Force $liteDir | Out-Null
Get-ChildItem $out | Where-Object { $_.Name -ne 'runtime' } | Copy-Item -Destination $liteDir -Recurse
Compress-Archive -Path (Join-Path $liteDir '*') -DestinationPath $lite -CompressionLevel Optimal
foreach ($z in $full, $lite) {
    $hash = (Get-FileHash $z -Algorithm SHA256).Hash.ToLower()
    Set-Content -Path "$z.sha256" -Value "$hash  $(Split-Path $z -Leaf)" -Encoding ASCII -NoNewline
    "{0} ({1:N1} MB) sha256={2}" -f (Split-Path $z -Leaf), ((Get-Item $z).Length / 1MB), $hash
}
