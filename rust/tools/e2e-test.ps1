# End-to-end shortcut test on Windows: real low-level hooks, real SendInput.
# Uses harmless keys (F23, F13+F14) so nothing reaches other apps even on failure.
param([Parameter(Mandatory)][string]$Exe, [Parameter(Mandatory)][string]$Dir)
$ErrorActionPreference = 'Stop'
if (Test-Path $Dir) { Remove-Item $Dir -Recurse -Force }
New-Item -ItemType Directory -Force (Join-Path $Dir 'data') | Out-Null
Copy-Item $Exe (Join-Path $Dir 'MakroPlus.exe')
Set-Content (Join-Path $Dir 'portable.txt') 'e2e'
$out1 = Join-Path $Dir 'out-f23.txt'; $out2 = Join-Path $Dir 'out-chord.txt'; $out3 = Join-Path $Dir 'out-wheel.txt'
function B($name, $mods, $key, $prefix, $file) {
  [ordered]@{ Id = [guid]::NewGuid().ToString(); Name = $name; Key = $key; Mods = $mods; PrefixKey = $prefix; AppPath = ''; Kind = 'Workflow'; Target = ''; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = ''
    Steps = @([ordered]@{ Kind = 'Write file'; Key = 0; Mods = 0; Delay = 0; Text = "ok {date} {app}"; Path = $file }) }
}
$state = [ordered]@{
  Bindings = @((B 'f23' 7 134 0 $out1), (B 'chord' 0 125 124 $out2), (B 'wheel' 7 257 0 $out3),
    [ordered]@{ Id = [guid]::NewGuid().ToString(); Name = 'off'; Key = 132; Mods = 0; AppPath = ''; Kind = 'Disable key'; Target = ''; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = '' })
  History = @(); Language = 'en'; AllowRemappedInput = $true; Updates = @{ Enabled = $false }; ActiveProfileId = 'work'
}
$state | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $Dir 'data\settings.json') -Encoding UTF8
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class E2E {
  [StructLayout(LayoutKind.Sequential)] struct KI { public ushort vk, scan; public uint flags, time; public IntPtr extra; }
  [StructLayout(LayoutKind.Sequential)] struct MI { public int dx, dy; public uint data, flags, time; public IntPtr extra; }
  [StructLayout(LayoutKind.Explicit)] struct U { [FieldOffset(0)] public KI k; [FieldOffset(0)] public MI m; }
  [StructLayout(LayoutKind.Sequential)] struct IN { public uint type; public U u; }
  [DllImport("user32.dll")] static extern uint SendInput(uint n, IN[] i, int size);
  public static uint Sent;
  public static void Key(ushort vk, bool up) { var i = new IN[1]; i[0].type = 1; i[0].u.k.vk = vk; i[0].u.k.flags = up ? 2u : 0u; Sent += SendInput(1, i, Marshal.SizeOf(typeof(IN))); }
  public static void Wheel(int delta) { var i = new IN[1]; i[0].type = 0; i[0].u.m.flags = 0x0800; i[0].u.m.data = (uint)delta; SendInput(1, i, Marshal.SizeOf(typeof(IN))); }
}
'@
$p = Start-Process -FilePath (Join-Path $Dir 'MakroPlus.exe') -ArgumentList '--snapshot', '--debug-input' -PassThru
Start-Sleep -Seconds 4
function Tap([uint16[]]$down) { foreach ($k in $down) { [E2E]::Key($k, $false); Start-Sleep -Milliseconds 30 }; [array]::Reverse($down); foreach ($k in $down) { [E2E]::Key($k, $true); Start-Sleep -Milliseconds 30 } }
# 1) Ctrl+Alt+Shift+F23
Tap @(162, 164, 160, 134)
Start-Sleep -Milliseconds 800
# 2) F13 held, then F14 (two-key chord)
[E2E]::Key(124, $false); Start-Sleep -Milliseconds 40; [E2E]::Key(125, $false); Start-Sleep -Milliseconds 40; [E2E]::Key(125, $true); Start-Sleep -Milliseconds 40; [E2E]::Key(124, $true)
Start-Sleep -Milliseconds 800
# 3) Ctrl+Alt+Shift + wheel down
[E2E]::Key(162, $false); [E2E]::Key(164, $false); [E2E]::Key(160, $false); Start-Sleep -Milliseconds 40; [E2E]::Wheel(-120); Start-Sleep -Milliseconds 40; [E2E]::Key(160, $true); [E2E]::Key(164, $true); [E2E]::Key(162, $true)
Start-Sleep -Milliseconds 800
# 4) Disabled key F21 (must not fire anything)
Tap @(132)
Start-Sleep -Milliseconds 1200
Stop-Process -Id $p.Id -Force
$results = [ordered]@{ 'Ctrl+Alt+Shift+F23' = (Test-Path $out1); 'F13+F14 chord' = (Test-Path $out2); 'Ctrl+Alt+Shift+WheelDown' = (Test-Path $out3) }
$results.GetEnumerator() | ForEach-Object { "{0,-28} {1}" -f $_.Key, $(if ($_.Value) { 'PASS' } else { 'FAIL' }) }
if (Test-Path $out1) { "content: $(Get-Content $out1)" }
$hist = (Get-Content (Join-Path $Dir 'data\settings.json') -Raw | ConvertFrom-Json).History | Select-Object -First 6 | ForEach-Object { "  history: $($_.Name) • $($_.Result)" }
$hist
"sent events: $([E2E]::Sent)"
if ($results.Values -contains $false) { "debug log:"; Get-Content (Join-Path $Dir 'data\input-debug.log') -ErrorAction SilentlyContinue | Select-Object -First 12; exit 1 }
