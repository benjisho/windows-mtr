param(
  [string]$ZipPath = "dist/windows-mtr-x86_64.zip"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $ZipPath)) { throw "Release ZIP not found: $ZipPath" }

$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("windows-mtr-smoke-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $temp | Out-Null

try {
  Expand-Archive -Path $ZipPath -DestinationPath $temp -Force
  $mtr = Join-Path $temp "mtr.exe"
  $windowsMtr = Join-Path $temp "windows-mtr.exe"
  $csvPath = Join-Path $temp "report.csv"
  $apiPort = 39876

  foreach ($binary in @($mtr, $windowsMtr)) {
    if (-not (Test-Path $binary)) { throw "Packaged executable not found: $binary" }
  }

  & $mtr --help | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "Packaged mtr.exe --help failed with exit code $LASTEXITCODE" }

  & $windowsMtr --version | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "Packaged windows-mtr.exe --version failed with exit code $LASTEXITCODE" }

  $json = & $mtr --json -n -c 1 127.0.0.1
  if ($LASTEXITCODE -ne 0) { throw "Packaged JSON report failed with exit code $LASTEXITCODE" }
  $jsonReport = $json | ConvertFrom-Json
  if ($jsonReport.schema_version -ne "1.0") { throw "Packaged JSON report did not emit schema_version 1.0" }

  & $mtr --csv $csvPath -n -c 1 127.0.0.1 | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "Packaged CSV report failed with exit code $LASTEXITCODE" }
  if (-not (Test-Path $csvPath)) { throw "Packaged CSV report was not created" }
  if ((Get-Content -First 1 $csvPath) -ne "hop,ip,hostname,avg_ms,best_ms,worst_ms,loss_pct") {
    throw "Packaged CSV report has an unexpected header"
  }

  & $mtr -T -P 443 -n -r -c 1 127.0.0.1 | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "Packaged TCP probe report failed with exit code $LASTEXITCODE" }

  & $mtr -U -P 53 -n -r -c 1 127.0.0.1 | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "Packaged UDP probe report failed with exit code $LASTEXITCODE" }

  $apiProcess = Start-Process -FilePath $mtr -ArgumentList "--api", "--api-bind", "127.0.0.1:$apiPort" -PassThru -WindowStyle Hidden
  try {
    $health = $null
    for ($attempt = 0; $attempt -lt 20 -and $null -eq $health; $attempt++) {
      Start-Sleep -Seconds 1
      try { $health = Invoke-RestMethod "http://127.0.0.1:$apiPort/api/v1/health" } catch { }
    }
    if ($null -eq $health) { throw "Packaged REST API did not become healthy" }
    if ($health.meta.schema_version -ne "v1" -or $health.data.status -ne "ok") {
      throw "Packaged REST API health response is invalid"
    }
  } finally {
    if (-not $apiProcess.HasExited) { Stop-Process -Id $apiProcess.Id -Force }
  }

  Write-Host "Release artifact smoke tests passed for $ZipPath"
} finally {
  if (Test-Path $temp) { Remove-Item -LiteralPath $temp -Recurse -Force }
}
