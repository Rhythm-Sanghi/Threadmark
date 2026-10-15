$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$fixture = Join-Path $projectRoot ('artifacts/packaging-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path "$fixture/scripts","$fixture/apps/desktop/src-tauri","$fixture/docs" | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'package-release.ps1') -Destination "$fixture/scripts"
[IO.File]::WriteAllText("$fixture/apps/desktop/src-tauri/tauri.conf.json", '{"version":"0.1.1"}')
[IO.File]::WriteAllText("$fixture/.gitignore", "ignored-secret.txt`nartifacts/`n")
[IO.File]::WriteAllText("$fixture/README.md", 'committed source')
[IO.File]::WriteAllText("$fixture/LICENSE", 'fixture license')
[IO.File]::WriteAllText("$fixture/docs/THIRD-PARTY-NOTICES.txt", 'fixture notices')
Push-Location $fixture
try {
    git init --quiet
    if ($LASTEXITCODE) { throw 'Fixture git init failed' }
    git add .
    git -c user.name=Fixture -c user.email=fixture@example.invalid commit --quiet -m fixture
    if ($LASTEXITCODE) { throw 'Fixture commit failed' }
    $commit = git rev-parse HEAD
    [IO.File]::WriteAllText("$fixture/untracked-secret.txt", 'must never enter archive')
    [IO.File]::WriteAllText("$fixture/ignored-secret.txt", 'must never enter archive')
    [IO.File]::WriteAllText("$fixture/README.md", 'uncommitted replacement')
    & ./scripts/package-release.ps1 -SourceOnly -OutputDirectory "$fixture/artifacts/first"
    & ./scripts/package-release.ps1 -SourceOnly -OutputDirectory "$fixture/artifacts/second"
    $first = "$fixture/artifacts/first/Threadmark-0.1.1-source.zip"
    $second = "$fixture/artifacts/second/Threadmark-0.1.1-source.zip"
    if ((Get-FileHash $first).Hash -ne (Get-FileHash $second).Hash) { throw 'Archive is not reproducible for one commit' }
    Expand-Archive -LiteralPath $first -DestinationPath "$fixture/artifacts/extracted"
    $source = "$fixture/artifacts/extracted/Threadmark-0.1.1"
    if ((Get-Content -Raw -LiteralPath "$source/README.md") -ne 'committed source') { throw 'Archive copied dirty source' }
    if (-not (Test-Path -LiteralPath "$source/.gitignore")) { throw 'Archive lost dotfiles' }
    foreach ($name in @('untracked-secret.txt','ignored-secret.txt')) {
        if (Test-Path -LiteralPath "$source/$name") { throw "Archive leaked $name" }
    }
    $manifest = Get-Content -Raw "$fixture/artifacts/first/SOURCE-MANIFEST.json" | ConvertFrom-Json
    if ($manifest.commit -ne $commit) { throw 'Manifest has wrong commit' }
    foreach ($line in (Get-Content "$fixture/artifacts/first/SHA256SUMS.txt")) {
        $hash,$name = $line -split '  ',2
        if ((Get-FileHash -LiteralPath "$fixture/artifacts/first/$name").Hash.ToLowerInvariant() -ne $hash) { throw "Checksum mismatch: $name" }
    }
    $rejected = $false
    try { & ./scripts/package-release.ps1 -SourceOnly -OutputDirectory "$fixture/artifacts/first" } catch { $rejected = $_.Exception.Message -like '*must be empty*' }
    if (-not $rejected) { throw 'Packaging accepted stale output directory' }
    $rejected = $false
    try { & ./scripts/package-release.ps1 -OutputDirectory "$fixture/artifacts/binary" } catch { $rejected = $_.Exception.Message -like '*tracked changes*' }
    if (-not $rejected) { throw 'Binary packaging accepted dirty source' }
    git restore --source=HEAD -- README.md
    if ($LASTEXITCODE) { throw 'Could not restore fixture source' }
    $rejected = $false
    try { & ./scripts/package-release.ps1 -OutputDirectory "$fixture/artifacts/missing-receipt" } catch { $rejected = $_.Exception.Message -like '*build-candidate*' }
    if (-not $rejected) { throw 'Binary packaging accepted missing build provenance' }
    @{ commit='incorrect'; artifacts=@() } | ConvertTo-Json | Set-Content "$fixture/artifacts/release-build.json"
    $rejected = $false
    try { & ./scripts/package-release.ps1 -OutputDirectory "$fixture/artifacts/wrong-receipt" } catch { $rejected = $_.Exception.Message -like '*another source commit*' }
    if (-not $rejected) { throw 'Binary packaging accepted another commit receipt' }
    Write-Output "PASS: immutable source, untracked/ignored exclusion, dotfiles, repeatability, manifest, hashes, stale output, dirty binary and missing/wrong receipt rejection. Fixture: $fixture"
} finally { Pop-Location }
