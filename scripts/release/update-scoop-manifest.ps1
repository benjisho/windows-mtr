param(
  [string]$Version,
  [string]$ZipPath = "dist/windows-mtr-x86_64.zip"
)
$ErrorActionPreference = "Stop"
if (-not $Version) {
  $cargo = Get-Content -Raw Cargo.toml
  if ($cargo -match 'version\s*=\s*"([^"]+)"') { $Version = $Matches[1] } else { throw "Could not determine version" }
}
$sha = if (Test-Path $ZipPath) { (Get-FileHash -Algorithm SHA256 -Path $ZipPath).Hash.ToLowerInvariant() } else { "REPLACE_WITH_RELEASE_SHA256" }
$url = "https://github.com/benjisho/windows-mtr/releases/download/v$Version/windows-mtr-x86_64.zip"
$path = "packaging/scoop/windows-mtr.json"
$json = Get-Content -Raw $path | ConvertFrom-Json
$json.version = $Version
$json.architecture."64bit".url = $url
$json.architecture."64bit".hash = $sha
($json | ConvertTo-Json -Depth 8) + "`n" | Set-Content $path
Write-Host "Updated $path for v$Version"
