$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$version = (Get-Content -LiteralPath (Join-Path $projectRoot 'apps/desktop/src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
$releaseDirectory = Join-Path $projectRoot 'artifacts/release'
$stagingDirectory = Join-Path $projectRoot ('artifacts/package-' + [guid]::NewGuid().ToString('N'))
$sourceDirectory = Join-Path $stagingDirectory ("Threadmark-" + $version)
$cliDirectory = Join-Path $stagingDirectory 'cli'
New-Item -ItemType Directory -Force -Path $releaseDirectory,$sourceDirectory,$cliDirectory | Out-Null
Push-Location $projectRoot
try {
    if (-not (Test-Path -LiteralPath 'target/release/threadmark.exe')) { throw 'Build release CLI first' }
    $sourceFiles = git -c core.quotePath=false ls-files --cached --others --exclude-standard
    if ($LASTEXITCODE -ne 0) { throw 'Could not inventory repository source' }
    foreach ($relative in $sourceFiles) {
        $absolute = [System.IO.Path]::GetFullPath((Join-Path $projectRoot $relative))
        if (-not $absolute.StartsWith($projectRoot + [System.IO.Path]::DirectorySeparatorChar,[System.StringComparison]::OrdinalIgnoreCase)) { throw "Source path escapes project: $relative" }
        if (-not (Test-Path -LiteralPath $absolute -PathType Leaf)) { continue }
        $destination = Join-Path $sourceDirectory $relative
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
        Copy-Item -LiteralPath $absolute -Destination $destination
    }
    Copy-Item -LiteralPath 'target/release/threadmark.exe','LICENSE','README.md','docs/THIRD-PARTY-NOTICES.txt' -Destination $cliDirectory
    Compress-Archive -LiteralPath $sourceDirectory -DestinationPath (Join-Path $releaseDirectory ("Threadmark-" + $version + "-source.zip")) -Force
    Compress-Archive -Path (Join-Path $cliDirectory '*') -DestinationPath (Join-Path $releaseDirectory ("Threadmark-" + $version + "-cli-x64.zip")) -Force
    $installer = ("target/release/bundle/nsis/Threadmark_" + $version + "_x64-setup.exe")
    if (Test-Path -LiteralPath $installer) { Copy-Item -LiteralPath $installer -Destination $releaseDirectory }
    Get-ChildItem -LiteralPath $releaseDirectory -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' } | Get-FileHash -Algorithm SHA256 | ForEach-Object { "$($_.Hash.ToLowerInvariant())  $([System.IO.Path]::GetFileName($_.Path))" } | Set-Content -LiteralPath (Join-Path $releaseDirectory 'SHA256SUMS.txt')
} finally { Pop-Location }
