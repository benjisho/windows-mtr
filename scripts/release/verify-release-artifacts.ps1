param(
  [Parameter(Mandatory=$true)][string]$ZipPath,
  [Parameter(Mandatory=$false)][string]$ChecksumPath = "",
  [Parameter(Mandatory=$false)][string]$ScoopManifest = "packaging/scoop/windows-mtr.json",
  [Parameter(Mandatory=$false)][string]$WingetManifest = "packaging/winget/windows-mtr.installer.yaml"
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path $ZipPath)) { throw "ZIP not found: $ZipPath" }
if ([string]::IsNullOrWhiteSpace($ChecksumPath)) { $ChecksumPath = Join-Path (Split-Path $ZipPath -Parent) 'SHA256SUM' }
if (-not (Test-Path $ChecksumPath)) { throw "SHA256SUM not found: $ChecksumPath" }

$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("windows-mtr-release-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp | Out-Null
Expand-Archive -Path $ZipPath -DestinationPath $temp -Force

$required = @('mtr.exe','windows-mtr.exe','README.txt','SHA256SUM')
foreach($f in $required){ if(-not(Test-Path (Join-Path $temp $f))){ throw "ZIP missing required file: $f" } }

$readme = Get-Content (Join-Path $temp 'README.txt') -Raw
if($readme -notmatch '\\.\\mtr\.exe\s+8\.8\.8\.8'){ throw 'README.txt missing .\mtr.exe command example' }
if($readme -notmatch '\\.\\windows-mtr\.exe\s+-r\s+-c\s+10\s+8\.8\.8\.8'){ throw 'README.txt missing .\windows-mtr.exe command example' }

$sumLines = Get-Content $ChecksumPath | Where-Object { $_.Trim() -ne '' }
foreach($line in $sumLines){
  if($line -notmatch '^([A-Fa-f0-9]{64})\s+\*?(.+)$'){ throw "Bad SHA256SUM line: $line" }
  $expected = $matches[1].ToUpperInvariant(); $name = $matches[2].Trim()
  $candidate = Join-Path (Split-Path $ChecksumPath -Parent) $name
  if(-not(Test-Path $candidate)){ $candidate = Join-Path $temp $name }
  if(-not(Test-Path $candidate)){ throw "SHA256SUM references missing file: $name" }
  $actual = (Get-FileHash -Algorithm SHA256 $candidate).Hash.ToUpperInvariant()
  if($actual -ne $expected){ throw "SHA mismatch for $name" }
}

$zipName = [System.IO.Path]::GetFileName($ZipPath)
if($ScoopManifest -and (Test-Path $ScoopManifest)){
  $scoop = Get-Content $ScoopManifest -Raw
  if($scoop -notmatch [regex]::Escape($zipName)){ throw "Scoop manifest URL does not reference $zipName" }
}
if($WingetManifest -and (Test-Path $WingetManifest)){
  $winget = Get-Content $WingetManifest -Raw
  if($winget -notmatch [regex]::Escape($zipName)){ throw "WinGet manifest URL does not reference $zipName" }
}

Write-Host "Release artifact verification passed for $ZipPath"
