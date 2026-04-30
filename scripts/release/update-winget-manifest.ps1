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
$path = "packaging/winget/windows-mtr.installer.yaml"
$c = Get-Content -Raw $path
$c = $c -replace 'PackageVersion: .*', "PackageVersion: $Version"
$c = $c -replace 'InstallerUrl: .*', "InstallerUrl: $url"
$c = $c -replace 'InstallerSha256: .*', "InstallerSha256: $sha"
Set-Content -Path $path -Value $c
Write-Host "Updated $path for v$Version"
