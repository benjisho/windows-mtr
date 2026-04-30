param([Parameter(Mandatory=$true)][string]$Version,[Parameter(Mandatory=$true)][string]$Sha256)
$ErrorActionPreference='Stop'
$path='packaging/scoop/windows-mtr.json'
$obj=Get-Content $path -Raw | ConvertFrom-Json
$obj.version=$Version
$obj.architecture.'64bit'.url="https://github.com/benjisho/windows-mtr/releases/download/v$Version/windows-mtr-x86_64.zip"
$obj.architecture.'64bit'.hash=$Sha256.ToLowerInvariant()
$obj | ConvertTo-Json -Depth 8 | Set-Content $path
Write-Host "Updated Scoop manifest for v$Version"
