param(
  [string]$Version,
  [string]$ZipPath = "dist/windows-mtr-x86_64.zip"
)
$ErrorActionPreference = "Stop"
if (-not $Version) {
  $cargo = Get-Content -Raw Cargo.toml
  if ($cargo -match 'version\s*=\s*"([^"]+)"') { $Version = $Matches[1] } else { throw "Could not determine version" }
}
$sha = if (Test-Path $ZipPath) { (Get-FileHash -Algorithm SHA256 -Path $ZipPath).Hash } else { "REPLACE_WITH_RELEASE_SHA256" }
$url = "https://github.com/benjisho/windows-mtr/releases/download/v$Version/windows-mtr-x86_64.zip"
$nuspec = "packaging/chocolatey/windows-mtr.portable.nuspec"
$content = Get-Content -Raw $nuspec
$content = $content -replace '<version>.*</version>', "<version>$Version</version>"
$content = $content -replace '__RELEASE_URL__', $url
$content = $content -replace '__ZIP_SHA256__', $sha
Set-Content -Path $nuspec -Value $content
Write-Host "Updated $nuspec for v$Version"
