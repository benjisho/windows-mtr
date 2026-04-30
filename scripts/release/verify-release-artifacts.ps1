param(
  [Parameter(Mandatory = $true)]
  [string]$ZipPath,
  [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
)

$ErrorActionPreference = "Stop"
$expectedZipName = "windows-mtr-x86_64.zip"
if ([System.IO.Path]::GetFileName($ZipPath) -ne $expectedZipName) {
  throw "Release ZIP name must be '$expectedZipName'"
}

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("windows-mtr-release-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tempDir | Out-Null
Expand-Archive -Path $ZipPath -DestinationPath $tempDir -Force

$expectedFiles = @("mtr.exe", "windows-mtr.exe", "README.txt", "SHA256SUM")
foreach ($name in $expectedFiles) {
  if (-not (Test-Path (Join-Path $tempDir $name))) {
    throw "ZIP missing required file: $name"
  }
}

$readme = Get-Content (Join-Path $tempDir "README.txt") -Raw
if ($readme -notmatch "\\.\\mtr\.exe" -or $readme -notmatch "\\.\\windows-mtr\.exe") {
  throw "README.txt commands do not match expected executable filenames"
}

$shaFile = Join-Path $tempDir "SHA256SUM"
$shaLines = Get-Content $shaFile
if ($shaLines.Count -lt 2) {
  throw "SHA256SUM must include at least mtr.exe and windows-mtr.exe"
}

foreach ($line in $shaLines) {
  if ([string]::IsNullOrWhiteSpace($line)) { continue }
  $parts = $line -split "\s+", 2
  if ($parts.Count -lt 2) { throw "Invalid SHA256SUM line: $line" }
  $expectedHash = $parts[0].Trim().ToUpperInvariant()
  $fileName = $parts[1].Trim()
  $fullPath = Join-Path $tempDir $fileName
  if (-not (Test-Path $fullPath)) {
    throw "SHA256SUM references missing file: $fileName"
  }
  $actualHash = (Get-FileHash -Path $fullPath -Algorithm SHA256).Hash.ToUpperInvariant()
  if ($actualHash -ne $expectedHash) {
    throw "SHA256 mismatch for $fileName"
  }
}

$manifestTargets = @(
  Join-Path $RepoRoot "packaging/scoop/windows-mtr.json",
  Join-Path $RepoRoot "packaging/winget/windows-mtr.installer.yaml",
  Join-Path $RepoRoot "packaging/chocolatey/tools/chocolateyinstall.ps1"
)

foreach ($manifest in $manifestTargets) {
  if (-not (Test-Path $manifest)) { continue }
  $content = Get-Content $manifest -Raw
  if ($content -notmatch "windows-mtr-x86_64.zip") {
    throw "Artifact naming drift: $manifest must reference windows-mtr-x86_64.zip"
  }
}

Write-Host "Release artifact verification passed for $ZipPath"
