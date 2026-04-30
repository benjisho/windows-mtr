param([Parameter(Mandatory=$true)][string]$Version,[Parameter(Mandatory=$true)][string]$Sha256)
$ErrorActionPreference='Stop'
$nuspec='packaging/chocolatey/windows-mtr.portable.nuspec'
$text=Get-Content $nuspec -Raw
$text=$text -replace '<version>.*</version>',"<version>$Version</version>"
$text=$text -replace 'REPLACE_WITH_RELEASE_TAG',"v$Version"
$text=$text -replace 'REPLACE_WITH_SHA256',$Sha256.ToLowerInvariant()
Set-Content $nuspec $text
$verification='packaging/chocolatey/tools/VERIFICATION.txt.template'
((Get-Content $verification -Raw) -replace 'REPLACE_WITH_RELEASE_TAG',"v$Version") -replace 'REPLACE_WITH_SHA256',$Sha256.ToLowerInvariant() | Set-Content $verification
Write-Host "Updated Chocolatey template for v$Version"
