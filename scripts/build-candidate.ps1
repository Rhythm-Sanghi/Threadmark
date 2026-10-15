param([string]$SigningThumbprint, [string]$TimestampUrl)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    $untracked = git ls-files --others --exclude-standard
    if ($LASTEXITCODE -ne 0 -or $untracked) { throw 'Commit or move untracked files before candidate build' }
    git diff --quiet HEAD --
    if ($LASTEXITCODE -ne 0) { throw 'Commit tracked source and dependency notices before candidate build' }
    $commit = git rev-parse HEAD
    if ($LASTEXITCODE -ne 0) { throw 'Could not identify build commit' }
    $receiptPath = Join-Path $projectRoot 'artifacts/release-build.json'
    if (Test-Path -LiteralPath $receiptPath) { Remove-Item -LiteralPath $receiptPath }
    if ($SigningThumbprint) {
        & (Join-Path $PSScriptRoot 'sign-release.ps1') -CertificateThumbprint $SigningThumbprint -TimestampUrl $TimestampUrl -SkipPackaging
    } else {
        cargo build --release -p threadmark-cli --locked
        if ($LASTEXITCODE -ne 0) { throw 'CLI build failed' }
        Push-Location 'apps/desktop'
        try { npm run tauri build; if ($LASTEXITCODE -ne 0) { throw 'Native build failed' } } finally { Pop-Location }
    }
    $after = git rev-parse HEAD
    if ($LASTEXITCODE -ne 0 -or $after -ne $commit) { throw 'Source changed during candidate build' }
    git diff --quiet HEAD --
    if ($LASTEXITCODE -ne 0) { throw 'Tracked files changed during candidate build' }
    $version = (Get-Content -Raw 'apps/desktop/src-tauri/tauri.conf.json' | ConvertFrom-Json).version
    $artifacts = @('target/release/threadmark.exe', "target/release/bundle/nsis/Threadmark_${version}_x64-setup.exe") | ForEach-Object {
        @{ path=$_; sha256=(Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant() }
    }
    New-Item -ItemType Directory -Force (Split-Path $receiptPath) | Out-Null
    @{ schema_version=1; commit=$commit; artifacts=@($artifacts) } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $receiptPath
} finally { Pop-Location }
