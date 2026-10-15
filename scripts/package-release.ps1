param([string]$SourceRef = 'HEAD', [switch]$SourceOnly, [string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $projectRoot 'artifacts/release' }
if ((Test-Path -LiteralPath $OutputDirectory) -and (Get-ChildItem -LiteralPath $OutputDirectory -Force | Select-Object -First 1)) {
    throw 'Output directory must be empty; choose a new candidate directory'
}
Push-Location $projectRoot
try {
    $commit = git rev-parse --verify --end-of-options "$SourceRef^{commit}"
    if ($LASTEXITCODE -ne 0) { throw 'SourceRef must identify a known commit' }
    $config = git show "${commit}:apps/desktop/src-tauri/tauri.conf.json"
    if ($LASTEXITCODE -ne 0) { throw 'Could not read committed version' }
    $version = ($config | ConvertFrom-Json).version
    if ($version -notmatch '^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?$') { throw 'Invalid committed version' }
    if (-not $SourceOnly) {
        $headCommit = git rev-parse HEAD
        if ($LASTEXITCODE -ne 0 -or $headCommit -ne $commit) { throw 'Binary packaging requires SourceRef to match HEAD' }
        git diff --quiet HEAD --
        if ($LASTEXITCODE -ne 0) { throw 'Commit all tracked changes before binary packaging' }
        $receiptPath = Join-Path $projectRoot 'artifacts/release-build.json'
        if (-not (Test-Path -LiteralPath $receiptPath)) { throw 'Build a candidate through scripts/build-candidate.ps1 first' }
        $receipt = Get-Content -Raw -LiteralPath $receiptPath | ConvertFrom-Json
        if ($receipt.commit -ne $commit) { throw 'Binary build receipt belongs to another source commit' }
        foreach ($artifact in $receipt.artifacts) {
            if ((Get-FileHash -LiteralPath (Join-Path $projectRoot $artifact.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $artifact.sha256) {
                throw "Binary changed after candidate build: $($artifact.path)"
            }
        }
        foreach ($path in @('target/release/threadmark.exe', "target/release/bundle/nsis/Threadmark_${version}_x64-setup.exe")) {
            if ($path -notin $receipt.artifacts.path) { throw "Build receipt missing $path" }
        }
    }
    New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
    $OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
    $stagingDirectory = Join-Path $projectRoot ('artifacts/package-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $stagingDirectory | Out-Null
    $sourceZip = Join-Path $OutputDirectory "Threadmark-${version}-source.zip"
    # Read immutable blobs, including dotfiles, never working-tree contents.
    git -c core.autocrlf=false archive --format=zip "--prefix=Threadmark-$version/" "--output=$sourceZip" $commit
    if ($LASTEXITCODE -ne 0) { throw 'Could not archive committed source' }
    $inventory = git ls-tree -r --full-tree $commit
    if ($LASTEXITCODE -ne 0) { throw 'Could not inventory committed source' }
    $manifest = [ordered]@{ schema_version=1; commit=$commit; version=$version; source_archive=[IO.Path]::GetFileName($sourceZip); tree_inventory=$inventory; binaries=@() }
    if (-not $SourceOnly) {
        Expand-Archive -LiteralPath $sourceZip -DestinationPath $stagingDirectory
        $sourceDirectory = Join-Path $stagingDirectory "Threadmark-$version"
        $cliDirectory = Join-Path $stagingDirectory 'cli'
        New-Item -ItemType Directory -Path $cliDirectory | Out-Null
        Copy-Item -LiteralPath 'target/release/threadmark.exe' -Destination $cliDirectory
        foreach ($path in @('LICENSE','README.md','docs/THIRD-PARTY-NOTICES.txt')) {
            Copy-Item -LiteralPath (Join-Path $sourceDirectory $path) -Destination $cliDirectory
        }
        Compress-Archive -Path (Join-Path $cliDirectory '*') -DestinationPath (Join-Path $OutputDirectory "Threadmark-${version}-cli-x64.zip")
        Copy-Item -LiteralPath "target/release/bundle/nsis/Threadmark_${version}_x64-setup.exe" -Destination $OutputDirectory
        $manifest.binaries = $receipt.artifacts
    }
    $manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'SOURCE-MANIFEST.json')
    Get-ChildItem -LiteralPath $OutputDirectory -File | Sort-Object Name | Get-FileHash -Algorithm SHA256 | ForEach-Object {
        "$($_.Hash.ToLowerInvariant())  $([IO.Path]::GetFileName($_.Path))"
    } | Set-Content -LiteralPath (Join-Path $OutputDirectory 'SHA256SUMS.txt')
} finally { Pop-Location }
