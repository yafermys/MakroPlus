# Touchpad-shape end-to-end test through real Windows Raw Input.
# Synthetic pointer movement (no clicks) arrives like a Precision Touchpad
# without a device handle. Tests detection, relative and absolute movement.
param([Parameter(Mandatory)][string]$Exe, [Parameter(Mandatory)][string]$Dir)
$ErrorActionPreference = 'Stop'
if (Test-Path $Dir) { Remove-Item $Dir -Recurse -Force }
New-Item -ItemType Directory -Force (Join-Path $Dir 'data') | Out-Null
Copy-Item $Exe (Join-Path $Dir 'MakroPlus.exe')
Set-Content (Join-Path $Dir 'portable.txt') 'gesture-e2e'
$out = Join-Path $Dir 'out-shape.txt'
$circle = @(); for ($i = 0; $i -lt 40; $i++) { $a = $i / 39 * 2 * [math]::PI; $circle += [ordered]@{ X = [math]::Round(100 * [math]::Cos($a), 2); Y = [math]::Round(100 * [math]::Sin($a), 2) } }
$state = [ordered]@{
  Bindings = @([ordered]@{ Id = [guid]::NewGuid().ToString(); Name = 'circle'; Key = 0; Mods = 0; AppPath = ''; Kind = 'Workflow'; Target = ''; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = ''
    Gesture = [ordered]@{ Points = $circle }; Steps = @([ordered]@{ Kind = 'Write file'; Key = 0; Mods = 0; Delay = 0; Text = 'shape {time}'; Path = $out; Append = $true }) })
  History = @(); Language = 'en'; Updates = @{ Enabled = $false }; Gestures = [ordered]@{ Enabled = $true; Device = ''; PauseMilliseconds = 600; MinimumLength = 120; MinimumScore = 0.88; MinimumMargin = 0.06 }
}
$state | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $Dir 'data\settings.json') -Encoding UTF8
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class GE {
  [StructLayout(LayoutKind.Sequential)] struct MI { public int dx, dy; public uint data, flags, time; public IntPtr extra; }
  [StructLayout(LayoutKind.Explicit)] struct U { [FieldOffset(0)] public MI m; }
  [StructLayout(LayoutKind.Sequential)] struct IN { public uint type; public U u; }
  [StructLayout(LayoutKind.Sequential)] public struct P { public int X, Y; }
  [DllImport("user32.dll")] static extern uint SendInput(uint n, IN[] i, int size);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out P p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern int GetSystemMetrics(int i);
  public static uint Sent;
  public static void Move(int dx, int dy, bool abs) { var i = new IN[1]; i[0].type = 0; i[0].u.m.dx = dx; i[0].u.m.dy = dy; i[0].u.m.flags = abs ? 0xC001u : 0x0001u; Sent += SendInput(1, i, Marshal.SizeOf(typeof(IN))); }
}
'@
$start = New-Object GE+P; [GE]::GetCursorPos([ref]$start) | Out-Null
$p = Start-Process -FilePath (Join-Path $Dir 'MakroPlus.exe') -ArgumentList '--snapshot', '--test-gesture-calibrate', '--debug-input' -PassThru
Start-Sleep -Seconds 4
# 1) Detection: the first movement identifies the touchpad.
[GE]::Move(4, 0, $false); Start-Sleep -Milliseconds 30; [GE]::Move(-4, 0, $false)
Start-Sleep -Seconds 2
$device = ([IO.File]::ReadAllText((Join-Path $Dir 'data\settings.json')) | ConvertFrom-Json).Gestures.Device
function Circle([bool]$abs, [int]$r) {
  $vx = [GE]::GetSystemMetrics(76); $vy = [GE]::GetSystemMetrics(77); $vw = [GE]::GetSystemMetrics(78); $vh = [GE]::GetSystemMetrics(79)
  $cx = $start.X; $cy = $start.Y; $px = $r; $py = 0
  for ($i = 1; $i -le 60; $i++) {
    $a = $i / 60 * 2 * [math]::PI; $x = [int]($r * [math]::Cos($a)); $y = [int]($r * [math]::Sin($a))
    if ($abs) { [GE]::Move([int](($cx + $x - $vx) * 65535 / $vw), [int](($cy + $y - $vy) * 65535 / $vh), $true) } else { [GE]::Move($x - $px, $y - $py, $false) }
    $px = $x; $py = $y; Start-Sleep -Milliseconds 12
  }
}
# 2) Relative movement (most touchpads/mice).
Circle $false 90; Start-Sleep -Milliseconds 1400
$rel = if (Test-Path $out) { (Get-Content $out).Count } else { 0 }
# 3) Absolute movement (many Precision Touchpads) - the path that used to be ignored.
Circle $true 120; Start-Sleep -Milliseconds 1400
$abs = if (Test-Path $out) { (Get-Content $out).Count } else { 0 }
Stop-Process -Id $p.Id -Force
[GE]::SetCursorPos($start.X, $start.Y) | Out-Null
"{0,-34} {1}" -f 'Touchpad detection', $(if ($device -eq 'precision-touchpad') { "PASS ($device)" } else { "FAIL ('$device')" })
"{0,-34} {1}" -f 'Shape via relative movement', $(if ($rel -ge 1) { 'PASS' } else { 'FAIL' })
"{0,-34} {1}" -f 'Shape via absolute movement', $(if ($abs -ge 2) { 'PASS' } else { 'FAIL' })
"pointer events sent: $([GE]::Sent) of 122"
if ($device -ne 'precision-touchpad' -or $rel -lt 1 -or $abs -lt 2) { Get-Content (Join-Path $Dir 'data\input-debug.log') -ErrorAction SilentlyContinue | Group-Object | Select-Object Count, Name -First 8 | Format-Table -AutoSize | Out-String; exit 1 }
