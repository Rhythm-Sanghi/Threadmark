$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$outputDirectory = Join-Path $projectRoot 'artifacts'
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
Push-Location $projectRoot
try {
    $metadata = cargo metadata --locked --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $metadata.packages | Select-Object name,version,license | Sort-Object name | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $outputDirectory 'rust-licenses.json')
    $nodeModules = Join-Path $projectRoot 'apps/desktop/node_modules'
    if (-not (Test-Path -LiteralPath $nodeModules)) { throw 'Run npm ci first' }
    Get-ChildItem -LiteralPath $nodeModules -Filter package.json -Recurse -File |
        ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw | ConvertFrom-Json } |
        Where-Object { $_.name -and $_.version } |
        Select-Object name,version,license | Sort-Object name,version -Unique |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $outputDirectory 'npm-licenses.json')
} finally { Pop-Location }
