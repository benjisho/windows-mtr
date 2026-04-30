param(
  [string]$ZipPath = "dist/windows-mtr-x86_64.zip",
  [string]$WingetManifest = "packaging/winget/windows-mtr.installer.yaml",
  [string]$ScoopManifest = "packaging/scoop/windows-mtr.json"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $ZipPath)) { throw "Release ZIP not found: $ZipPath" }

$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("windows-mtr-release-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $temp | Out-Null
Expand-Archive -Path $ZipPath -DestinationPath $temp -Force

$required = @("mtr.exe", "windows-mtr.exe", "README.txt", "SHA256SUM")
foreach ($file in $required) {
  if (-not (Test-Path (Join-Path $temp $file))) {
    throw "Release ZIP is missing required file: $file"
  }
}

$readme = Get-Content -Raw -Path (Join-Path $temp "README.txt")
if ($readme -notmatch '\\.\\mtr\\.exe\s+8\\.8\\.8\\.8') {
  throw "README.txt missing required command: .\\mtr.exe 8.8.8.8"
}
if ($readme -notmatch '\\.\\windows-mtr\\.exe\s+-r\s+-c\s+10\s+8\\.8\\.8\\.8') {
  throw "README.txt missing required command: .\\windows-mtr.exe -r -c 10 8.8.8.8"
}

$hashLines = Get-Content (Join-Path $temp "SHA256SUM")
$map = @{}
foreach ($line in $hashLines) {
  if ($line -match '^([A-Fa-f0-9]{64})\s+(.+)$') {
    $map[$Matches[2].Trim()] = $Matches[1].ToUpperInvariant()
  }
}

foreach ($file in @("mtr.exe", "windows-mtr.exe")) {
  if (-not $map.ContainsKey($file)) {
    throw "SHA256SUM missing entry for $file"
  }
  $actual = (Get-FileHash -Path (Join-Path $temp $file) -Algorithm SHA256).Hash.ToUpperInvariant()
  if ($actual -ne $map[$file]) {
    throw "SHA256 mismatch for $file"
  }
}

$zipName = Split-Path -Leaf $ZipPath
if (Test-Path $WingetManifest) {
  $winget = Get-Content -Raw $WingetManifest
  if ($winget -notmatch [regex]::Escape($zipName)) {
    throw "WinGet manifest URL does not reference $zipName"
  }
}

if (Test-Path $ScoopManifest) {
  $scoop = Get-Content -Raw $ScoopManifest
  if ($scoop -notmatch [regex]::Escape($zipName)) {
    throw "Scoop manifest URL does not reference $zipName"
  }
}

Write-Host "Release artifact verification passed for $ZipPath"
