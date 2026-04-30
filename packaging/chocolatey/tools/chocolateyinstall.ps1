$ErrorActionPreference = 'Stop'

$packageName = 'windows-mtr.portable'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$url64 = 'https://github.com/benjisho/windows-mtr/releases/download/vVERSION/windows-mtr-x86_64.zip'
$checksum64 = 'REPLACE_WITH_RELEASE_SHA256'

Install-ChocolateyZipPackage \
  -PackageName $packageName \
  -Url64bit $url64 \
  -Checksum64 $checksum64 \
  -ChecksumType64 'sha256' \
  -UnzipLocation $toolsDir
