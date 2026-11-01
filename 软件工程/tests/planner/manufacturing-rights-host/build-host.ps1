$ErrorActionPreference='Stop'
Set-Location -LiteralPath (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$out='scenarios/专业桌面/runtime-manufacturing-ui-host'
$cut=Get-Content -LiteralPath "$out/accepted-library-cut.json" -Encoding utf8 | ConvertFrom-Json
$argsList=@('--edition=2024','tests/planner/manufacturing-rights-host/host.rs','-L',"dependency=$out/pinned-libraries",'-L','dependency=target/debug/deps')
foreach($name in @('sf_commands','sf_domain','serde','serde_json','zip')) {
  $argsList+=@('--extern',"$name=$($cut.libraries.$name.copied)")
}
$argsList+=@('-o',"$out/actual-host.exe")
$prior=$env:CARGO_MANIFEST_DIR
try {
  $env:CARGO_MANIFEST_DIR=(Resolve-Path packages/commands).Path
  & 'C:/Users/fujunye/.cargo/bin/rustc.exe' @argsList 1> "$out/host-build.stdout.txt" 2> "$out/host-build.stderr.txt"
  $code=$LASTEXITCODE
} finally {$env:CARGO_MANIFEST_DIR=$prior}
Get-Content -LiteralPath "$out/host-build.stderr.txt" -Tail 12
if($code-ne 0){throw "Pinned rustc adapter failed: $code"}
Write-Output 'Exact accepted Host dependency group linked with rustc; no Cargo execution.'
