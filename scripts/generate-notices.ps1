$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    $metadata = cargo metadata --locked --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $noticePath = Join-Path $projectRoot 'docs/THIRD-PARTY-NOTICES.txt'
    $parts = [System.Collections.Generic.List[string]]::new()
    $parts.Add('Threadmark third-party notices. Inventory includes development and all-platform dependencies. Built with GPL-3.0-or-later Threadmark sources; each dependency keeps its own license.')
    foreach ($package in ($metadata.packages | Sort-Object name,version)) {
        $parts.Add("`n===== $($package.name) $($package.version) : $($package.license) =====")
        $folder = Split-Path -Parent $package.manifest_path
        $licenseFiles = Get-ChildItem -LiteralPath $folder -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|UNLICENSE)' }
        foreach ($licenseFile in $licenseFiles) { $parts.Add((Get-Content -LiteralPath $licenseFile.FullName -Raw)) }
        if ($package.name -in @('libgit2-sys','libz-sys','libsqlite3-sys')) { Get-ChildItem -LiteralPath $folder -Recurse -File | Where-Object { $_.DirectoryName -ne $folder -and $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|UNLICENSE)' } | ForEach-Object { $parts.Add((Get-Content -LiteralPath $_.FullName -Raw)) } }
    }
    $nodeModules = Join-Path $projectRoot 'apps/desktop/node_modules'
    foreach ($manifest in (Get-ChildItem -LiteralPath $nodeModules -Filter package.json -Recurse -File)) {
        $package = Get-Content -LiteralPath $manifest.FullName -Raw | ConvertFrom-Json
        if (-not $package.name -or -not $package.version) { continue }
        $folder = Split-Path -Parent $manifest.FullName
        $licenseFiles = Get-ChildItem -LiteralPath $folder -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|UNLICENSE)' }
        $parts.Add("`n===== $($package.name) $($package.version) : $($package.license) =====")
        foreach ($licenseFile in $licenseFiles) { $parts.Add((Get-Content -LiteralPath $licenseFile.FullName -Raw)) }
        if ($package.name -in @('libgit2-sys','libz-sys','libsqlite3-sys')) { Get-ChildItem -LiteralPath $folder -Recurse -File | Where-Object { $_.DirectoryName -ne $folder -and $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|UNLICENSE)' } | ForEach-Object { $parts.Add((Get-Content -LiteralPath $_.FullName -Raw)) } }
    }
    [System.IO.File]::WriteAllText($noticePath, [string]::Join("`n", $parts))
} finally { Pop-Location }

