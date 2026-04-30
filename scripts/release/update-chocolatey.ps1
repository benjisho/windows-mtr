param(
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][string]$ZipPath,
  [string]$NuspecPath = "packaging/chocolatey/windows-mtr.portable.nuspec",
  [string]$InstallScriptPath = "packaging/chocolatey/tools/chocolateyinstall.ps1",
  [string]$VerificationTemplatePath = "packaging/chocolatey/tools/VERIFICATION.txt.template"
)

$ErrorActionPreference = "Stop"
$hash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash

foreach ($path in @($NuspecPath, $InstallScriptPath, $VerificationTemplatePath)) {
  (Get-Content $path -Raw).
    Replace("VERSION", $Version).
    Replace("REPLACE_WITH_RELEASE_SHA256", $hash) |
    Set-Content $path
}

Write-Host "Updated Chocolatey templates for $Version"
