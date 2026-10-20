param(
    [string]$GamePath = $(if ($env:FALLOUT_GAME_PATH) { $env:FALLOUT_GAME_PATH } else { Join-Path $PSScriptRoot '..\..\..\common\Fallout' }),
    [string]$OutputPath = (Join-Path $PSScriptRoot 'reports'),
    [string]$Search = ''
)
$ErrorActionPreference = 'Stop'
$game = (Resolve-Path -LiteralPath $GamePath).Path
$out = [IO.Path]::GetFullPath($OutputPath)
if ($out.Equals($game, [StringComparison]::OrdinalIgnoreCase) -or $out.StartsWith($game + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Reports must be outside the game folder.'
}
if (!(Test-Path -LiteralPath (Join-Path $game 'FALLOUTW.EXE'))) { throw 'Expected original Fallout installation.' }
$files = @(Get-ChildItem -LiteralPath $game -File -Recurse | Sort-Object FullName)
$inventory = @($files | ForEach-Object {
    [pscustomobject]@{path=$_.FullName.Substring($game.Length+1); bytes=$_.Length; extension=$_.Extension.ToLowerInvariant()}
})
$messages = @()
foreach ($file in $files | Where-Object Extension -ieq '.msg') {
    $text = [IO.File]::ReadAllText($file.FullName, [Text.Encoding]::GetEncoding(1252))
    foreach ($m in [regex]::Matches($text, '\{(\d+)\}\s*\{([^}]*)\}\s*\{([^}]*)\}')) {
        $messages += [pscustomobject]@{path=$file.FullName.Substring($game.Length+1); id=[int]$m.Groups[1].Value; audio=$m.Groups[2].Value; text=$m.Groups[3].Value}
    }
}
$strings = @()
foreach ($file in $files | Where-Object { $_.Extension -ieq '.exe' -and $_.Name -match '^falloutw(HR)?\.exe$' }) {
    $text = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($file.FullName))
    foreach ($m in [regex]::Matches($text, '[\x20-\x7e]{6,}')) {
        $strings += [pscustomobject]@{path=$file.Name; offset=$m.Index; text=$m.Value}
    }
}
if ($Search) {
    @($messages) + @($strings) | Where-Object { $_.text.IndexOf($Search,[StringComparison]::OrdinalIgnoreCase) -ge 0 } | Format-Table path,id,offset,text -Wrap
    return
}
New-Item -ItemType Directory -Path $out -Force | Out-Null
$fingerprints = @($files | Where-Object { $_.Extension -in '.exe','.dll','.dat','.cfg','.ini','.int' } | ForEach-Object {
    [pscustomobject]@{path=$_.FullName.Substring($game.Length+1); sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash}
})
$summary = [ordered]@{game=$game; generated_utc=[DateTime]::UtcNow.ToString('o'); file_count=$files.Count; total_bytes=($files | Measure-Object Length -Sum).Sum; loose_message_count=$messages.Count; executable_string_count=$strings.Count; limitations=@('DAT archives are fingerprinted, not decoded.','INT bytecode is inventoried, not decompiled.','Messages and strings establish clues, not formulas or runtime behavior.','Only loose MSG files are indexed; archived messages are not included.'); extensions=@($inventory | Group-Object extension | Sort-Object Count -Descending | ForEach-Object { [pscustomobject]@{extension=$_.Name; count=$_.Count} })}
foreach ($pair in @(@('inventory',$inventory),@('fingerprints',$fingerprints),@('messages',$messages),@('strings',$strings),@('summary',$summary))) {
    ConvertTo-Json -InputObject $pair[1] -Depth 8 | Set-Content -LiteralPath (Join-Path $out ($pair[0]+'.json')) -Encoding utf8
}
$rows = $summary.extensions | ConvertTo-Html -Fragment
$sample = $messages | Select-Object -First 100 path,id,text | ConvertTo-Html -Fragment
$html = @"
<!doctype html><html><head><meta charset="utf-8"><title>Fallout Research Inventory</title>
<style>body{background:#101815;color:#dcebdc;font:16px system-ui;max-width:1100px;margin:40px auto;padding:20px}h1,h2{color:#9ee5a5}td,th{padding:9px;text-align:left;border-bottom:1px solid #385044}table{border-collapse:collapse;width:100%}code{color:#b5e7be}</style></head><body>
<h1>Fallout Research Inventory</h1><p>$($files.Count) files · $($messages.Count) loose messages · $($strings.Count) executable strings</p>
<p>Read-only scan. Original installation is the behavioral reference. See MECHANICS.md for research coverage and next experiments.</p>
<h2>Coverage limits</h2><p>Archives are not decoded. Scripts are not decompiled. Strings are clues, not verified mechanics.</p>
<h2>File types</h2>$rows<h2>Loose message evidence (first 100)</h2>$sample
<p>Full evidence and SHA-256 fingerprints are in the adjacent JSON files. Search with Inspect-Fallout.ps1 -Search 'action points'.</p></body></html>
"@
$html | Set-Content -LiteralPath (Join-Path $out 'index.html') -Encoding utf8
Write-Output "Scanned $($files.Count) files; indexed $($messages.Count) messages and $($strings.Count) executable strings. Report: $out\index.html"
