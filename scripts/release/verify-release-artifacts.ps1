param(
  [string]$ZipPath = "dist/windows-mtr-x86_64.zip",
  [string]$ShaPath = "dist/bundle/SHA256SUM"
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path $ZipPath)) { throw "Missing ZIP: $ZipPath" }
if (-not (Test-Path $ShaPath)) { throw "Missing SHA file: $ShaPath" }

$extractDir = Join-Path ([System.IO.Path]::GetTempPath()) ("windows-mtr-release-" + [System.Guid]::NewGuid().ToString("N"))
Expand-Archive -Path $ZipPath -DestinationPath $extractDir -Force

$expected = @('mtr.exe','windows-mtr.exe','README.txt','SHA256SUM')
foreach ($name in $expected) {
  if (-not (Test-Path (Join-Path $extractDir $name))) {
    throw "ZIP missing expected file: $name"
  }
}

$readme = Get-Content (Join-Path $extractDir 'README.txt') -Raw
if ($readme -notmatch '\\.\\mtr\.exe\s+8\.8\.8\.8') { throw 'README.txt missing expected mtr.exe command' }
if ($readme -notmatch '\\.\\windows-mtr\.exe\s+-r\s+-c\s+10\s+8\.8\.8\.8') { throw 'README.txt missing expected windows-mtr.exe command' }

$shaEntries = Get-Content $ShaPath | Where-Object { $_.Trim() -ne '' }
if ($shaEntries.Count -lt 3) { throw 'SHA256SUM must include at least zip and two exe entries' }

foreach ($entry in $shaEntries) {
  $parts = $entry -split '\s+', 2
  if ($parts.Count -ne 2) { throw "Invalid SHA256SUM line: $entry" }
  $hash = $parts[0].ToLower()
  $file = $parts[1].Trim()
  $candidate = if ($file -eq 'windows-mtr-x86_64.zip') { $ZipPath } else { Join-Path $extractDir $file }
  if (-not (Test-Path $candidate)) { throw "SHA entry references missing file: $file" }
  $actual = (Get-FileHash $candidate -Algorithm SHA256).Hash.ToLower()
  if ($actual -ne $hash) { throw "SHA mismatch for $file" }
}

$manifestFiles = @(
  'packaging/winget/windows-mtr.installer.yaml',
  'packaging/scoop/windows-mtr.json',
  'packaging/chocolatey/windows-mtr.portable.nuspec'
)
foreach ($mf in $manifestFiles) {
  if (-not (Test-Path $mf)) { throw "Missing packaging manifest: $mf" }
  $content = Get-Content $mf -Raw
  if ($content -notmatch 'windows-mtr-x86_64\.zip') {
    throw "Manifest does not reference canonical ZIP name: $mf"
  }
}

Write-Host 'Release artifact validation passed.'
