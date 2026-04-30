param(
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][string]$ZipPath,
  [string]$ManifestPath = "packaging/scoop/windows-mtr.json"
)

$ErrorActionPreference = "Stop"
$hash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash.ToLowerInvariant()
$url = "https://github.com/benjisho/windows-mtr/releases/download/v$Version/windows-mtr-x86_64.zip"

$json = Get-Content $ManifestPath -Raw | ConvertFrom-Json
$json.version = $Version
$json.architecture.'64bit'.url = $url
$json.architecture.'64bit'.hash = $hash
$json.autoupdate.architecture.'64bit'.url = 'https://github.com/benjisho/windows-mtr/releases/download/v$version/windows-mtr-x86_64.zip'

$json | ConvertTo-Json -Depth 8 | Set-Content $ManifestPath
Write-Host "Updated Scoop manifest for $Version"
