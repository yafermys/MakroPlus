param([string]$OutputDirectory='dist\interface-preview',[switch]$DesktopPreview,[switch]$SkipBuild)
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
if(-not $SkipBuild){& (Join-Path $root 'build.ps1') -OutputDirectory $OutputDirectory}
$folder=Join-Path $root $OutputDirectory
$argument=if($DesktopPreview){'--interface-preview'}else{'--system-test'}
$reportName=if($DesktopPreview){'interface-test-results.txt'}else{'system-test-results.txt'}
$process=Start-Process -FilePath (Join-Path $folder 'MakroPlus.exe') -ArgumentList $argument -WindowStyle Hidden -PassThru
if(-not $process.WaitForExit(60000)){throw "Preview checks did not finish in 60 seconds; inspect process $($process.Id)."}
$report=Get-Content -LiteralPath (Join-Path $folder $reportName) -Raw -Encoding UTF8
if($process.ExitCode -ne 0 -or -not $report.StartsWith('PASS')){throw $report}
Write-Output (($report -split '\r?\n')[0])
Write-Output (Join-Path $folder $reportName)
if($DesktopPreview){Write-Output "Screenshots: $folder"}
