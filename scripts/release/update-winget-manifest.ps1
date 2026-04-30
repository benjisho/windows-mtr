param(
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][string]$ZipPath,
  [string]$ManifestDir = "packaging/winget"
)

$ErrorActionPreference = "Stop"
$hash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash
$url = "https://github.com/benjisho/windows-mtr/releases/download/v$Version/windows-mtr-x86_64.zip"

Get-ChildItem -Path $ManifestDir -Filter "*.yaml" | ForEach-Object {
  (Get-Content $_.FullName -Raw).
    Replace("VERSION", $Version).
    Replace("REPLACE_WITH_RELEASE_SHA256", $hash) |
    Set-Content $_.FullName
}

(Get-Content (Join-Path $ManifestDir "windows-mtr.installer.yaml") -Raw).
  Replace("https://github.com/benjisho/windows-mtr/releases/download/v$Version/windows-mtr-x86_64.zip", $url) |
  Set-Content (Join-Path $ManifestDir "windows-mtr.installer.yaml")

Write-Host "Updated WinGet manifests for $Version"
