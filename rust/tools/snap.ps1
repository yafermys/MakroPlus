# Developer helper: start a portable test copy off-screen (no focus) on a page and
# render ONLY its own window with PrintWindow â€” nothing else on the screen is captured.
param([Parameter(Mandatory)][string]$Exe, [string]$Page = 'home', [Parameter(Mandatory)][string]$Out, [int]$Wait = 4, [switch]$Keep, [string[]]$Extra = @())
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class SnapWin {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);
}
'@
[SnapWin]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null
Get-Process MakroPlus -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $Exe } | Stop-Process -Force
Start-Sleep -Milliseconds 300
$p = Start-Process -FilePath $Exe -ArgumentList (@('--snapshot', '--page', $Page) + $Extra) -PassThru
$h = [IntPtr]::Zero
for ($i = 0; $i -lt 60 -and $h -eq [IntPtr]::Zero; $i++) { Start-Sleep -Milliseconds 250; $p.Refresh(); if ($p.MainWindowTitle -eq 'Makro+') { $h = $p.MainWindowHandle } }
if ($h -eq [IntPtr]::Zero) { throw 'window not found' }
Start-Sleep -Seconds $Wait
$r = New-Object SnapWin+RECT
[SnapWin]::GetWindowRect($h, [ref]$r) | Out-Null
$w = $r.R - $r.L; $hh = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
[SnapWin]::PrintWindow($h, $hdc, 2) | Out-Null
$g.ReleaseHdc($hdc)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
if (-not $Keep) { Stop-Process -Id $p.Id -Force }
"$Out ($w x $hh)"

