$ErrorActionPreference = 'Stop'
$testWork = Join-Path $PSScriptRoot '.work'
$testTemp = Join-Path $testWork 'temp'
New-Item -ItemType Directory -Path $testTemp -Force | Out-Null
$previousTemp = $env:TEMP
$previousTmp = $env:TMP
$testExit = 1
try {
    $env:TEMP = $testTemp
    $env:TMP = $testTemp
    & cargo test --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml') --locked --offline --target-dir (Join-Path $testWork 'target') -j 1 -- --nocapture
    $testExit = $LASTEXITCODE
} finally {
    $env:TEMP = $previousTemp
    $env:TMP = $previousTmp
}
exit $testExit
