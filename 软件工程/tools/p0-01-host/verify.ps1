$ErrorActionPreference = 'Stop'
$productRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$proofRoot = Join-Path $productRoot '.work/p0-01-host/verification'
New-Item -ItemType Directory -Force -Path $proofRoot | Out-Null
Set-Location -LiteralPath $productRoot
$results = [Collections.Generic.List[object]]::new()

function Invoke-SliceCheck([string]$Key, [string]$Program, [string[]]$Arguments) {
    $started = (Get-Date).ToString('o')
    $stdout = Join-Path $proofRoot "$Key.stdout.txt"
    $stderr = Join-Path $proofRoot "$Key.stderr.txt"
    Write-Output "P0-01 Host: $Key"
    & $Program @Arguments 1> $stdout 2> $stderr
    $code = $LASTEXITCODE
    $entry = [ordered]@{
        key = $Key; command = (@($Program) + $Arguments) -join ' '
        started_at = $started; finished_at = (Get-Date).ToString('o'); exit_code = $code
        stdout_ref = ".work/p0-01-host/verification/$Key.stdout.txt"
        stderr_ref = ".work/p0-01-host/verification/$Key.stderr.txt"
    }
    $results.Add($entry)
    [ordered]@{status='running';checks=@($results)} | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $proofRoot 'result.json') -Encoding utf8
    if ($code -ne 0) {
        Get-Content -LiteralPath $stdout -Tail 20
        Get-Content -LiteralPath $stderr -Tail 15
        throw "P0-01 Host check failed: $Key ($code)"
    }
}

try {
    Invoke-SliceCheck 'format' 'cargo' @('fmt','-p','sf-commands','-p','sf-desktop-host','--','--check')
    Invoke-SliceCheck 'workspace-tests' 'npm.cmd' @('run','test:rust')
    Invoke-SliceCheck 'generate-contracts' 'npm.cmd' @('run','generate:contracts')
    Invoke-SliceCheck 'typecheck' 'npm.cmd' @('run','typecheck')
    $runtime = 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
    Invoke-SliceCheck 'schema-consumer' $runtime @('-B','-X','utf8','tools/检查工程与Command边界.py')
    Invoke-SliceCheck 'desktop-build' 'cargo' @('build','-p','sf-desktop-host','--bin','superfactory','--locked')
    # Process execution is hidden by the caller (and has no visible WebView in this smoke mode).
    Invoke-SliceCheck 'desktop-host-smoke' '.\target\debug\superfactory.exe' @('--p0-01-host-smoke')
    $smoke = Get-Content -LiteralPath (Join-Path $proofRoot 'desktop-host-smoke.stdout.txt') -Encoding utf8 |
        Where-Object { $_ -match '^\{' } | ForEach-Object { $_ | ConvertFrom-Json } |
        Where-Object { $_.kind -eq 'P0_01_host_smoke' }
    if ($null -eq $smoke -or $smoke.passed -ne $true) { throw 'Missing actual desktop Host smoke receipt' }
    [ordered]@{status='passed';checks=@($results);native_host_smoke=$smoke;whole_task_done=$false} |
        ConvertTo-Json -Depth 14 | Set-Content (Join-Path $proofRoot 'result.json') -Encoding utf8
    Write-Output 'P0-01 Host verification passed'
} catch {
    [ordered]@{status='failed';checks=@($results);error=$_.Exception.Message} |
        ConvertTo-Json -Depth 12 | Set-Content (Join-Path $proofRoot 'result.json') -Encoding utf8
    throw
}
