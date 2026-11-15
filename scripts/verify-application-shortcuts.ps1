param([string]$OutputDirectory='dist\application-shortcuts',[switch]$DesktopPreview,[switch]$SkipBuild)
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
if(-not $SkipBuild){& (Join-Path $root 'build.ps1') -OutputDirectory $OutputDirectory}
$folder=Join-Path $root $OutputDirectory
$argument=if($DesktopPreview){'--app-shortcut-preview'}else{'--app-shortcut-test'}
$process=Start-Process -FilePath (Join-Path $folder 'MakroPlus.exe') -ArgumentList $argument -WindowStyle Hidden -PassThru
if(-not $process.WaitForExit(60000)){throw "Application shortcut checks timed out; inspect process $($process.Id)."}
$report=Get-Content -LiteralPath (Join-Path $folder 'application-scan-tests.txt') -Raw -Encoding UTF8
if($process.ExitCode -ne 0 -or -not $report.StartsWith('PASS')){throw $report}
Write-Output (($report -split '\r?\n')[0])
Write-Output (Join-Path $folder 'application-scan-tests.txt')
