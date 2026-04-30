param(
  [string]$Version,
  [string]$Sha256 = 'REPLACE_WITH_RELEASE_SHA256'
)
$ErrorActionPreference='Stop'
if (-not $Version) {
  $Version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
}
$baseUrl = "https://github.com/benjisho/windows-mtr/releases/download/v$Version/windows-mtr-x86_64.zip"
(Get-Content packaging/winget/windows-mtr.yaml -Raw).Replace('VERSION', $Version) | Set-Content packaging/winget/windows-mtr.yaml
$content = Get-Content packaging/winget/windows-mtr.installer.yaml -Raw
$content = $content.Replace('VERSION', $Version).Replace('REPLACE_WITH_RELEASE_SHA256', $Sha256)
$content = [regex]::Replace($content, 'InstallerUrl:\s*.+', "InstallerUrl: $baseUrl")
Set-Content packaging/winget/windows-mtr.installer.yaml $content
(Get-Content packaging/winget/windows-mtr.locale.en-US.yaml -Raw).Replace('VERSION', $Version) | Set-Content packaging/winget/windows-mtr.locale.en-US.yaml
Write-Host "Updated WinGet manifests for version $Version"
