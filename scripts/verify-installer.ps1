$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$installationRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'artifacts/installed'))
$allowedRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'artifacts')) + [IO.Path]::DirectorySeparatorChar
if (-not $installationRoot.StartsWith($allowedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Test installation escapes artifacts directory' }
$registrationPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Threadmark'
if (Test-Path $registrationPath) {
    $registered = (Get-ItemProperty $registrationPath).InstallLocation.Trim('"').TrimEnd('\')
    if ($registered -ne $installationRoot) { throw 'Existing installation belongs to another location; test stopped' }
}
if (Get-Process threadmark-desktop -ErrorAction SilentlyContinue) { throw 'Close Threadmark before installer lifecycle verification' }
$version = (Get-Content -Raw (Join-Path $projectRoot 'apps/desktop/src-tauri/tauri.conf.json') | ConvertFrom-Json).version
$installer = Join-Path $projectRoot "target/release/bundle/nsis/Threadmark_${version}_x64-setup.exe"
$cli = Join-Path $installationRoot 'threadmark.exe'
$before = $null
if (Test-Path -LiteralPath $cli) {
    $before = & $cli --json status | ConvertFrom-Json
    if ($LASTEXITCODE) { throw 'Existing installed CLI failed' }
}
function Install-Candidate {
    $process = Start-Process -FilePath $installer -ArgumentList @('/S',"/D=$installationRoot") -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode) { throw "Installer failed: $($process.ExitCode)" }
    foreach ($name in @('threadmark.exe','threadmark-desktop.exe','LICENSE','THIRD-PARTY-NOTICES.txt','uninstall.exe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $installationRoot $name))) { throw "Missing installed file: $name" }
    }
    if ((Get-ItemProperty $registrationPath).DisplayVersion -ne $version) { throw 'Incorrect installer registration version' }
    foreach ($name in @('threadmark.exe','threadmark-desktop.exe')) {
        $expectedBytes = [IO.File]::ReadAllBytes((Join-Path $projectRoot "target/release/$name"))
        if ($name -eq 'threadmark-desktop.exe') {
            # Tauri patches the installer payload to NSS, then restores the build
            # executable to UNK. Account only for this exact bundle marker.
            $marker = '__TAURI_BUNDLE_TYPE_VAR_UNK'
            $offset = [Text.Encoding]::Latin1.GetString($expectedBytes).IndexOf($marker, [StringComparison]::Ordinal)
            if ($offset -ge 0) {
                [Text.Encoding]::ASCII.GetBytes('__TAURI_BUNDLE_TYPE_VAR_NSS').CopyTo($expectedBytes, $offset)
            }
        }
        $expectedHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($expectedBytes))
        if ((Get-FileHash (Join-Path $installationRoot $name)).Hash -ne $expectedHash) { throw "Installed binary mismatch: $name" }
    }
}
Install-Candidate
$upgraded = & $cli --json status | ConvertFrom-Json
if ($LASTEXITCODE) { throw 'Upgraded CLI failed' }
if ($upgraded.app_version -ne $version -or $upgraded.database_schema -ne 3) { throw 'Version/migration verification failed' }
if ($before -and ($before.theme -ne $upgraded.theme -or $before.repository.root -ne $upgraded.repository.root)) { throw 'Upgrade lost persisted settings/repository' }
$dataPath = $upgraded.database_path
$dataHash = (Get-FileHash -LiteralPath $dataPath).Hash
$process = Start-Process -FilePath (Join-Path $installationRoot 'uninstall.exe') -ArgumentList @('/S',"_?=$installationRoot") -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode) { throw "Uninstaller failed: $($process.ExitCode)" }
if ((Test-Path -LiteralPath $cli) -or (Test-Path $registrationPath)) { throw 'Uninstall left application/registration behind' }
if ((Get-FileHash -LiteralPath $dataPath).Hash -ne $dataHash) { throw 'Uninstall changed the user database' }
Install-Candidate
$reinstalled = & $cli --json status | ConvertFrom-Json
if ($LASTEXITCODE) { throw 'Reinstalled CLI failed' }
if ($reinstalled.theme -ne $upgraded.theme -or $reinstalled.repository.root -ne $upgraded.repository.root -or $reinstalled.database_schema -ne 3) { throw 'Reinstall lost persisted state' }
$report = @{ version=$version; previous_version=$before.app_version; installation=$installationRoot; upgrade=$true; uninstall=$true; reinstall=$true; data_preserved=$true; database_schema=3; signature=(Get-AuthenticodeSignature -LiteralPath $installer).Status.ToString() }
$report | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $projectRoot 'artifacts/installer-lifecycle-0.1.1.json')
$report | ConvertTo-Json
