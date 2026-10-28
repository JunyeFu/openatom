param(
    [switch]$IntegratedBackendOnly,
    [ValidatePattern('^\.work/whole-packages/[a-z0-9-]+$')]
    [string]$ProofDirectory
)
$ErrorActionPreference = 'Stop'
$productRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$relativeProof = if ($ProofDirectory) { $ProofDirectory } elseif ($IntegratedBackendOnly) { '.work/whole-packages/verification' } else { '.work/whole-packages/verification-with-packaging' }
$proofRoot = Join-Path $productRoot $relativeProof
New-Item -ItemType Directory -Force -Path $proofRoot | Out-Null
Set-Location -LiteralPath $productRoot
$results = [Collections.Generic.List[object]]::new()

function Invoke-PackageCheck([string]$Key, [string]$Program, [string[]]$Arguments) {
    $started = (Get-Date).ToString('o')
    $stdout = Join-Path $proofRoot "$Key.stdout.txt"
    $stderr = Join-Path $proofRoot "$Key.stderr.txt"
    Write-Output "Whole task integration: $Key"
    try {
        $ErrorActionPreference = 'Continue'
        & $Program @Arguments 1> $stdout 2> $stderr
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = 'Stop' }
    $entry = [ordered]@{
        key=$Key; command=(@($Program)+$Arguments)-join ' '
        started_at=$started; finished_at=(Get-Date).ToString('o'); exit_code=$code
        stdout_ref="$relativeProof/$Key.stdout.txt"; stderr_ref="$relativeProof/$Key.stderr.txt"
    }
    if ($Key -eq 'workspace-tests' -and $code -eq 0) {
        $passed = 0
        Select-String -LiteralPath $stdout -Pattern 'test result: ok\. (\d+) passed;' -AllMatches |
            ForEach-Object { foreach ($match in $_.Matches) { $passed += [int]$match.Groups[1].Value } }
        $entry.distinct_tests_executed = $passed
    }
    $results.Add($entry)
    [ordered]@{status='running';checks=@($results);whole_task_done=$false} |
        ConvertTo-Json -Depth 16 | Set-Content (Join-Path $proofRoot 'result.json') -Encoding utf8
    if ($code -ne 0) {
        Get-Content -LiteralPath $stdout -Tail 24
        Get-Content -LiteralPath $stderr -Tail 20
        throw "Whole package check failed: $Key ($code)"
    }
}

try {
    if ($IntegratedBackendOnly) {
        Invoke-PackageCheck 'format' 'cargo' @('fmt','-p','sf-domain','-p','sf-commands','-p','sf-simulation','-p','sf-desktop-host','--','--check')
        # P4-02 implementation and its independent acceptance tests are still in
        # progress. This verifies the integrated backend, not that unfinished package.
        Invoke-PackageCheck 'workspace-tests' 'cargo' @('test','--workspace','--exclude','sf-packaging','--locked','--offline')
    } else {
        Invoke-PackageCheck 'format' 'cargo' @('fmt','--all','--','--check')
        Invoke-PackageCheck 'workspace-tests' 'cargo' @('test','--workspace','--locked')
    }
    Invoke-PackageCheck 'generate-contracts' 'npm.cmd' @('run','generate:contracts')
    Invoke-PackageCheck 'typecheck' 'npm.cmd' @('run','typecheck')
    $runtime = 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
    Invoke-PackageCheck 'schema-consumer' $runtime @('-B','-X','utf8','tools/检查工程与Command边界.py','--output',"$relativeProof/schema-validation-result.json")
    Invoke-PackageCheck 'desktop-build' 'cargo' @('build','-p','sf-desktop-host','--bin','superfactory','--locked')
    Invoke-PackageCheck 'desktop-host-smoke' '.\target\debug\superfactory.exe' @('--p0-01-host-smoke')
    $smoke = Get-Content -LiteralPath (Join-Path $proofRoot 'desktop-host-smoke.stdout.txt') -Encoding utf8 |
        Where-Object { $_ -match '^\{' } | ForEach-Object { $_ | ConvertFrom-Json } |
        Where-Object { $_.kind -eq 'P0_01_host_smoke' }
    if ($null -eq $smoke -or $smoke.passed -ne $true) { throw 'Actual desktop Host did not pass' }
    $previous = Get-Content -LiteralPath '.work/p0-02-stock/verification/result.json' -Encoding utf8 | ConvertFrom-Json
    $names = @($smoke.checks | ForEach-Object { $_.name })
    foreach ($old in $previous.native_host_smoke.checks) {
        if ($old.name -notin $names) { throw ('Previous native check removed: '+$old.name) }
    }
    $oldTests = Select-String -LiteralPath '.work/p0-02-stock/verification/workspace-tests.stdout.txt' -Pattern '^test .+ \.\.\. ok$' | ForEach-Object { $_.Line.Trim() }
    $newTests = Select-String -LiteralPath (Join-Path $proofRoot 'workspace-tests.stdout.txt') -Pattern '^test .+ \.\.\. ok$' | ForEach-Object { $_.Line.Trim() }
    foreach ($old in $oldTests) {
        if ($old -notin $newTests) { throw ('Previous Rust test removed: '+$old) }
    }
    [ordered]@{status='passed';checks=@($results);native_host_smoke=$smoke;whole_task_done=$false;
        integrated_backend_only=[bool]$IntegratedBackendOnly;
        excluded_test_packages=$(if ($IntegratedBackendOnly) {@('sf-packaging')} else {@()});
        acceptance_scope='Actual test and native Host results; whole original task acceptance requires separate review'} |
        ConvertTo-Json -Depth 100 | Set-Content (Join-Path $proofRoot 'result.json') -Encoding utf8
    Write-Output 'Whole package integration checks passed'
} catch {
    [ordered]@{status='failed';checks=@($results);error=$_.Exception.Message;whole_task_done=$false} |
        ConvertTo-Json -Depth 16 | Set-Content (Join-Path $proofRoot 'result.json') -Encoding utf8
    throw
}
