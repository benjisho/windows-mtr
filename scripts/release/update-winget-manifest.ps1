param([Parameter(Mandatory=$true)][string]$Version,[Parameter(Mandatory=$true)][string]$Sha256)
$ErrorActionPreference='Stop'
$url = "https://github.com/benjisho/windows-mtr/releases/download/v$Version/windows-mtr-x86_64.zip"
$path='packaging/winget/windows-mtr.installer.yaml'
$text=Get-Content $path -Raw
$text=$text -replace 'PackageVersion: .*',"PackageVersion: $Version"
$text=$text -replace 'InstallerUrl: .*',"    InstallerUrl: $url"
$text=$text -replace 'InstallerSha256: .*',"    InstallerSha256: $Sha256"
Set-Content $path $text
(Get-Content 'packaging/winget/windows-mtr.yaml') -replace 'PackageVersion: .*',"PackageVersion: $Version" | Set-Content 'packaging/winget/windows-mtr.yaml'
(Get-Content 'packaging/winget/windows-mtr.locale.en-US.yaml') -replace 'PackageVersion: .*',"PackageVersion: $Version" | Set-Content 'packaging/winget/windows-mtr.locale.en-US.yaml'
Write-Host "Updated WinGet manifests for v$Version"
