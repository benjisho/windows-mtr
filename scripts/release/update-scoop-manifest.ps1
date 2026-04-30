param(
  [string]$Version,
  [string]$Sha256 = 'REPLACE_WITH_RELEASE_SHA256'
)
$ErrorActionPreference='Stop'
if (-not $Version) {
  $Version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
}
$url = "https://github.com/benjisho/windows-mtr/releases/download/v$Version/windows-mtr-x86_64.zip"
$json = Get-Content packaging/scoop/windows-mtr.json -Raw | ConvertFrom-Json
$json.version = $Version
$json.architecture.'64bit'.url = $url
$json.architecture.'64bit'.hash = $Sha256
$json | ConvertTo-Json -Depth 8 | Set-Content packaging/scoop/windows-mtr.json
Write-Host "Updated Scoop manifest for version $Version"
