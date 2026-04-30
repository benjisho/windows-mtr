param(
  [string]$Version,
  [string]$Sha256 = 'REPLACE_WITH_RELEASE_SHA256'
)
$ErrorActionPreference='Stop'
if (-not $Version) {
  $Version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
}
$url = "https://github.com/benjisho/windows-mtr/releases/download/v$Version/windows-mtr-x86_64.zip"
$nuspec = Get-Content packaging/chocolatey/windows-mtr.portable.nuspec -Raw
$nuspec = $nuspec.Replace('VERSION', $Version)
Set-Content packaging/chocolatey/windows-mtr.portable.nuspec $nuspec
$verification = Get-Content packaging/chocolatey/tools/VERIFICATION.txt.template -Raw
$verification = $verification.Replace('VERSION', $Version).Replace('REPLACE_WITH_RELEASE_SHA256', $Sha256).Replace('RELEASE_URL', $url)
Set-Content packaging/chocolatey/tools/VERIFICATION.txt.template $verification
Write-Host "Updated Chocolatey templates for version $Version"
