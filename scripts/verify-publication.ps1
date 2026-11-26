$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
$tracked=@(& git -C $root -c core.quotepath=false ls-files)
if($LASTEXITCODE -ne 0 -or $tracked.Count -eq 0){throw 'Stage or commit the intended source files first.'}
foreach($required in @('LICENSE','.gitignore','README.md','THIRD_PARTY_NOTICES.md','build.ps1','setup-runtime.ps1','packaging/USER_GUIDE.html','packaging/models/README.txt')){if($required -notin $tracked){throw "Missing publication input: $required"}}
# Makro+ 3 is the Rust app; its version lives in rust/Cargo.toml.
$cargo=Join-Path $root 'rust\Cargo.toml'
$versionMatch=if(Test-Path $cargo){[regex]::Match([IO.File]::ReadAllText($cargo),'(?m)^version = "(\d+\.\d+\.\d+)"')}else{[regex]::Match([IO.File]::ReadAllText((Join-Path $root 'src\Brand.cs')),'Version="(\d+\.\d+\.\d+)"')}
if(-not $versionMatch.Success){throw 'Cannot read release version.'}
$version=$versionMatch.Groups[1].Value
$readme=[IO.File]::ReadAllText((Join-Path $root 'README.md'))
foreach($link in @("releases/download/v$version/MakroPlus-$version-win-x64.zip","releases/download/v$version/MakroPlus-$version-source.zip","releases/tag/v$version")){if(-not $readme.Contains($link)){throw "Update README release links before publishing: $link"}}
if(-not ([IO.File]::ReadAllText((Join-Path $root 'CHANGELOG.md')).Contains('## '+$version))){throw 'Update CHANGELOG before publishing.'}
if(('docs/releases/'+$version+'.md') -notin $tracked){throw 'Stage matching English release notes before publishing.'}
$issues=New-Object 'System.Collections.Generic.List[string]'
$bytes=0L
foreach($name in $tracked){
    $file=Get-Item -LiteralPath (Join-Path $root $name)
    $bytes+=$file.Length
    if($name -match '^assets/' -and $name -notin @('assets/cover.png','assets/MakroPlus.ico')){$issues.Add("Unreviewed design intermediary: $name");continue}
    if($name -match '^rust/assets/' -and $name -notin @('rust/assets/logo.png','rust/assets/cover.png','rust/assets/MakroPlus.ico','rust/assets/MakroPlus.manifest')){$issues.Add("Unreviewed asset: $name");continue}
    if($name -match '^(dist|release|runtime|models)/' -or $file.Extension -in @('.gguf','.safetensors','.onnx','.pt','.pth','.exe','.dll','.zip','.pfx','.pem','.key') -or $file.Length -gt 5MB){$issues.Add("Generated/private/oversized input: $name");continue}
    if($file.Extension -notin @('.png','.ico')){
        $content=[IO.File]::ReadAllText($file.FullName)
        if($content -match '(?i)(?:github_pat_[A-Za-z0-9_]{20,}|gh[pousr]_[A-Za-z0-9]{20,}|sk-[A-Za-z0-9_-]{25,}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)'){$issues.Add("Possible credential: $name")}
        if($content -match '(?i)[A-Z]:[\\/]+Users[\\/]+(?!Public[\\/]|USERNAME[\\/]|Example[\\/])[^\\/\s"\u0027]+[\\/]'){$issues.Add("Development-computer path: $name")}
    }
}
if($issues.Count -gt 0){$issues|ForEach-Object {Write-Output $_};throw 'Publication content checks failed.'}
if(-not ([IO.File]::ReadAllText((Join-Path $root 'LICENSE')).Contains('GNU GENERAL PUBLIC LICENSE'))){throw 'GPL license text missing.'}
Write-Output ('PASS: '+$tracked.Count+' reviewed source files; '+$bytes+' bytes; no model/runtime binaries, generated releases, recognized credential formats or development-computer paths.')
