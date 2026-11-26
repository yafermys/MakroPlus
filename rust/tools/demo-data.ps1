# Creates a portable Makro+ folder filled with SYNTHETIC demo data (for public screenshots).
param([Parameter(Mandatory)][string]$Exe, [Parameter(Mandatory)][string]$Dir, [string]$Language = 'tr', [string]$Theme = 'graphite', [switch]$Background)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force (Join-Path $Dir 'data') | Out-Null
Copy-Item $Exe (Join-Path $Dir 'MakroPlus.exe') -Force
Set-Content (Join-Path $Dir 'portable.txt') 'demo'
$notepad = "$env:SystemRoot\System32\notepad.exe"
$paint = "$env:SystemRoot\System32\mspaint.exe"
$calc = "$env:SystemRoot\System32\calc.exe"
function Id { [guid]::NewGuid().ToString() }
function Step($kind, $extra) { $s = [ordered]@{ Kind = $kind; Key = 0; Mods = 0; Delay = 0; Text = '' }; foreach ($k in $extra.Keys) { $s[$k] = $extra[$k] }; $s }
$circle = @(); for ($i = 0; $i -lt 40; $i++) { $a = $i / 39 * 2 * [math]::PI; $circle += [ordered]@{ X = [math]::Round(120 * [math]::Cos($a), 2); Y = [math]::Round(120 * [math]::Sin($a), 2) } }
$tr = $Language -eq 'tr'
$bindings = @(
  [ordered]@{ Id = (Id); Name = $(if ($tr) { 'Seçimi İngilizceye çevir' } else { 'Translate selection' }); Key = 84; Mods = 3; AppPath = ''; Kind = 'Workflow'; Target = ''; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = ''; Steps = @((Step 'Copy selection' @{ Var = 'selection' }), (Step 'AI' @{ Text = '{selection}'; Instruction = 'Translate to English.'; Output = 'paste' })) }
  [ordered]@{ Id = (Id); Name = $(if ($tr) { "Seçimi Not Defteri'ne gönder" } else { 'Send selection to Notepad' }); Key = 49; Mods = 6; AppPath = ''; Kind = 'Workflow'; Target = ''; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = ''; Steps = @((Step 'Copy selection' @{}), (Step 'Focus' @{ AppPath = $notepad }), (Step 'Key' @{ Key = 35; Mods = 2; AppPath = $notepad }), (Step 'Paste text' @{ Text = '{newline}{selection}'; AppPath = $notepad })) }
  [ordered]@{ Id = (Id); Name = $(if ($tr) { 'Kaydet ve yedekle' } else { 'Save and back up' }); Key = 119; Mods = 0; AppPath = $notepad; Kind = 'Workflow'; Target = ''; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = ''; Steps = @((Step 'Menu command' @{ Text = 'File › Save'; AppPath = $notepad }), (Step 'Wait' @{ Delay = 600 }), (Step 'Copy file' @{ Path = '{documents}\notes.txt'; Text = '{documents}\Backup\' })) }
  [ordered]@{ Id = (Id); Name = $(if ($tr) { 'Ekran alıntısı' } else { 'Screen snip' }); Key = 120; Mods = 0; AppPath = ''; Kind = 'System'; Target = 'screenshot'; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = '' }
  [ordered]@{ Id = (Id); Name = 'YouTube'; Key = 53; Mods = 0; PrefixKey = 46; AppPath = ''; Kind = 'Web address'; Target = 'https://www.youtube.com/'; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = '' }
  [ordered]@{ Id = (Id); Name = $(if ($tr) { 'Hesap makinesi' } else { 'Calculator' }); Key = 5; Mods = 0; AppPath = ''; Kind = 'Application'; Target = $calc; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = '' }
  [ordered]@{ Id = (Id); Name = $(if ($tr) { 'Caps Lock kapalı' } else { 'Caps Lock off' }); Key = 20; Mods = 0; AppPath = ''; Kind = 'Disable key'; Target = ''; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = '' }
  [ordered]@{ Id = (Id); Name = $(if ($tr) { 'Daire: masaüstü' } else { 'Circle: desktop' }); Key = 0; Mods = 0; AppPath = ''; Kind = 'System'; Target = 'desktop'; Arguments = ''; Enabled = $true; PassThrough = $false; SendKey = 0; SendMods = 0; ProfileId = ''; Gesture = [ordered]@{ Points = $circle } }
)
$inv = [ordered]@{ AppPath = $notepad; Scanned = '2026-11-26 10:00'; WindowTitle = 'notes.txt - Notepad'; Commands = @(
  [ordered]@{ Source = 'menu'; Label = 'File › New tab'; Accelerator = 'Ctrl+N' }, [ordered]@{ Source = 'menu'; Label = 'File › Save'; Accelerator = 'Ctrl+S' },
  [ordered]@{ Source = 'menu'; Label = 'File › Save as'; Accelerator = 'Ctrl+Shift+S' }, [ordered]@{ Source = 'menu'; Label = 'Edit › Find'; Accelerator = 'Ctrl+F' },
  [ordered]@{ Source = 'menu'; Label = 'Edit › Replace'; Accelerator = 'Ctrl+H' }, [ordered]@{ Source = 'menu'; Label = 'Edit › Time/Date'; Accelerator = 'F5' },
  [ordered]@{ Source = 'element'; Label = 'Settings'; AutomationId = 'SettingsButton'; ControlType = 50000; Role = 'button' },
  [ordered]@{ Source = 'element'; Label = 'Text editor'; AutomationId = 'RichEditBox'; ControlType = 50004; Role = 'field' },
  [ordered]@{ Source = 'element'; Label = 'Tabs › notes.txt'; AutomationId = 'TabItem1'; ControlType = 50019; Role = 'tab' }) }
$now = Get-Date
$history = @(0..5 | ForEach-Object { [ordered]@{ Time = $now.AddMinutes(-$_ * 7).ToString('yyyy-MM-dd HH:mm:ss'); Name = $bindings[$_ % 6].Name; Result = $(if ($tr) { 'İş akışı tamamlandı • 2 adım' } else { 'Workflow finished • 2 steps' }) } })
$bgPath = Join-Path $Dir 'data\demo-background.png'
if ($Background) {
  Add-Type -AssemblyName System.Drawing
  $bmp = New-Object System.Drawing.Bitmap 1600, 1000; $g = [System.Drawing.Graphics]::FromImage($bmp); $g.SmoothingMode = 'AntiAlias'
  $rect = New-Object System.Drawing.Rectangle 0, 0, 1600, 1000
  $lg = New-Object System.Drawing.Drawing2D.LinearGradientBrush $rect, ([System.Drawing.Color]::FromArgb(255, 18, 32, 58)), ([System.Drawing.Color]::FromArgb(255, 72, 28, 96)), 35.0
  $g.FillRectangle($lg, $rect)
  $rnd = New-Object System.Random 7
  for ($i = 0; $i -lt 26; $i++) { $c = [System.Drawing.Color]::FromArgb(40 + $rnd.Next(60), 80 + $rnd.Next(175), 120 + $rnd.Next(135), 200 + $rnd.Next(55)); $b = New-Object System.Drawing.SolidBrush $c; $s = 60 + $rnd.Next(320); $g.FillEllipse($b, $rnd.Next(1600) - 100, $rnd.Next(1000) - 100, $s, $s); $b.Dispose() }
  $bmp.Save($bgPath, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
}
$state = [ordered]@{
  Bindings = $bindings; History = $history; Dark = $true; Language = $Language; ThemeId = $Theme; CustomThemes = @(); Paused = $false
  ApplicationPaths = @($notepad, $paint, $calc); ModelPath = ''; ModelOptions = [ordered]@{ Remote = $true; Endpoint = 'http://localhost:1234/v1/'; Model = 'qwen3.5-4b-instruct'; StructuredOutput = $true; FastMode = $true; MaxOutputTokens = 4096 }
  ShortcutSources = @(); Profiles = @([ordered]@{ Id = 'work'; Name = 'Work'; SuppressTypingKeys = $false }, [ordered]@{ Id = 'game'; Name = 'Game'; SuppressTypingKeys = $true }, [ordered]@{ Id = 'study'; Name = 'Study'; SuppressTypingKeys = $false })
  ActiveProfileId = 'work'; Preferences = $null; GuideEnabled = $true; GuideHoldKey = 163; Gestures = [ordered]@{ Enabled = $true; Device = 'demo'; PauseMilliseconds = 600; MinimumLength = 120; MinimumScore = 0.88; MinimumMargin = 0.06 }
  AllowRemappedInput = $false; Updates = [ordered]@{ Enabled = $false }; AppInventories = @($inv); CloseToTray = $true
  Background = [ordered]@{ Path = $(if ($Background) { $bgPath } else { '' }); Dim = 0.45; PanelOpacity = 0.78; Enabled = [bool]$Background }; AutoTheme = [bool]$Background
}
$state | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $Dir 'data\settings.json') -Encoding UTF8
$clips = @(
  [ordered]@{ id = (Id); kind = 'text'; text = $(if ($tr) { "Toplantı notları:`n- Tasarım incelemesi Perşembe`n- Sürüm 3.0 yayın planı" } else { "Meeting notes:`n- Design review on Thursday`n- 3.0 release plan" }); files = @(); time = $now.AddMinutes(-2).ToString('yyyy-MM-dd HH:mm:ss'); source = 'notepad'; pinned = $true },
  [ordered]@{ id = (Id); kind = 'text'; text = 'https://github.com/yafermys/MakroPlus'; files = @(); time = $now.AddMinutes(-5).ToString('yyyy-MM-dd HH:mm:ss'); source = 'msedge'; pinned = $false },
  [ordered]@{ id = (Id); kind = 'files'; text = ''; files = @('C:\Demo\report.pdf', 'C:\Demo\chart.png'); time = $now.AddMinutes(-9).ToString('yyyy-MM-dd HH:mm:ss'); source = 'explorer'; pinned = $false },
  [ordered]@{ id = (Id); kind = 'text'; text = $(if ($tr) { 'Merhaba, raporu ekte gönderiyorum. İyi çalışmalar.' } else { 'Hi, the report is attached. Best regards.' }); files = @(); time = $now.AddMinutes(-14).ToString('yyyy-MM-dd HH:mm:ss'); source = 'outlook'; pinned = $false }
)
ConvertTo-Json -InputObject $clips -Depth 5 | Set-Content (Join-Path $Dir 'data\clipboard.json') -Encoding UTF8
"Demo folder: $Dir"
