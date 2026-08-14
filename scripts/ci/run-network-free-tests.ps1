[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Keep normal CI deterministic and free of live probes. Loopback and externally
# reachable probe suites are invoked only from their dedicated workflow lanes.
$commands = @(
    @('test', '--locked', '--workspace', '--lib', '--bins'),
    @(
        'test', '--locked',
        '--test', 'api_contract_tests',
        '--test', 'api_integration_tests',
        '--test', 'cli_tests',
        '--test', 'probe_parity_tests',
        '--test', 'report_tests',
        '--test', 'rest_api_security_tests'
    )
)

foreach ($command in $commands) {
    Write-Host "cargo $($command -join ' ')"
    & cargo @command
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
