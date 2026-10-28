$ErrorActionPreference='Stop'
$standardRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
Set-Location -LiteralPath $standardRoot
$standardOut='scenarios/媒体表现/runtime-line-grade/standard-package-fix'
$standardLibs=Get-Content -LiteralPath "$standardOut/actual-host-libraries.json" -Raw | ConvertFrom-Json
& 'C:/Users/fujunye/.cargo/bin/rustc.exe' --edition=2024 tests/planner/media-line-grade/standard_host.rs -L dependency=target/debug/deps --extern "sf_commands=$($standardLibs.commands)" --extern "sf_domain=$($standardLibs.sf_domain)" --extern "serde_json=$($standardLibs.serde_json)" -o "$standardOut/standard_host.exe" *> "$standardOut/host-build.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Matching real Host consumer compilation failed'}
& './scenarios/媒体表现/runtime-line-grade/standard-package-fix/standard_host.exe' *> "$standardOut/actual-host-after.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Actual Host registration/query/exact/bytes failed'}
& 'C:/Program Files/nodejs/node.exe' --test tests/planner/media-line-grade/geometry.test.mjs tests/planner/media-line-grade/rebuild.test.mjs *> "$standardOut/geometry-rebuild.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Original geometry/rebuild checks failed'}
& 'C:/Program Files/nodejs/node.exe' node_modules/typescript/bin/tsc --noEmit --incremental false -p apps/planner/tsconfig.json *> "$standardOut/app-typecheck.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Planner typecheck failed'}
Write-Output 'Actual same-Host standard registration passed; no Cargo/Native/listener or Engine168 rerun.'
