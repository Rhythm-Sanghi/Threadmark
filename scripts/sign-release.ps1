param(
    [Parameter(Mandatory=$true)][ValidatePattern('^[a-fA-F0-9]{40}$')][string]$CertificateThumbprint,
    [string]$SignTool,
    [string]$TimestampUrl
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$certificate = Get-Item -LiteralPath ('Cert:\CurrentUser\My\' + $CertificateThumbprint)
if (-not $certificate.HasPrivateKey -or $certificate.NotAfter -lt (Get-Date)) { throw 'A valid certificate with an accessible private key is required' }
if (-not ($certificate.EnhancedKeyUsageList.ObjectId -contains '1.3.6.1.5.5.7.3.3')) { throw 'Certificate is not authorized for code signing' }
$chain = [Security.Cryptography.X509Certificates.X509Chain]::new()
try { if (-not $chain.Build($certificate)) { throw 'Certificate must chain to a trusted authority; self-signed test certificates do not establish release trust' } } finally {$chain.Dispose()}
if (-not $SignTool) {
    $sdk = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
    $SignTool = Get-ChildItem -LiteralPath $sdk -Filter signtool.exe -Recurse -File | Where-Object { $_.Directory.Name -eq 'x64' } | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $SignTool -or -not (Test-Path -LiteralPath $SignTool -PathType Leaf)) {throw 'Pass the installed Windows SDK SignTool executable'}
if ($TimestampUrl -and -not $TimestampUrl.StartsWith('https://')) {throw 'Timestamp service must use HTTPS'}
Push-Location $projectRoot
try {
    cargo build --release -p threadmark-cli --locked
    if ($LASTEXITCODE -ne 0) {throw 'CLI build failed'}
    $arguments = @('sign','/sha1',$CertificateThumbprint,'/fd','SHA256')
    if ($TimestampUrl) {$arguments += @('/tr',$TimestampUrl,'/td','SHA256')}
    & $SignTool @arguments 'target/release/threadmark.exe'
    if ($LASTEXITCODE -ne 0) {throw 'CLI signing failed'}
    $config = @{bundle=@{windows=@{certificateThumbprint=$CertificateThumbprint;digestAlgorithm='sha256'}}}
    if ($TimestampUrl) {$config.bundle.windows.timestampUrl=$TimestampUrl;$config.bundle.windows.tsp=$true}
    $configPath = Join-Path $projectRoot 'artifacts/signing-config.json'
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $configPath) | Out-Null
    $config | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $configPath
    Push-Location (Join-Path $projectRoot 'apps/desktop')
    try {npm run tauri build -- --config $configPath; if($LASTEXITCODE -ne 0){throw 'Signed native build failed'}} finally {Pop-Location}
    # The bundler restores the standalone executable's UNK bundle marker after
    # creating the signed NSIS payload. Sign this separate restored executable
    # too, when restoration leaves it unsigned or invalid.
    if ((Get-AuthenticodeSignature -LiteralPath 'target/release/threadmark-desktop.exe').Status -ne 'Valid') {
        & $SignTool @arguments 'target/release/threadmark-desktop.exe'
        if ($LASTEXITCODE -ne 0) {throw 'Standalone desktop signing failed'}
    }
    $version = (Get-Content -Raw -LiteralPath 'apps/desktop/src-tauri/tauri.conf.json' | ConvertFrom-Json).version
    foreach($binary in @('target/release/threadmark.exe','target/release/threadmark-desktop.exe',"target/release/bundle/nsis/Threadmark_${version}_x64-setup.exe")) {
        if ((Get-AuthenticodeSignature -LiteralPath $binary).Status -ne 'Valid') {throw "Signature validation failed: $binary"}
    }
    & (Join-Path $PSScriptRoot 'package-release.ps1')
} finally {Pop-Location}
