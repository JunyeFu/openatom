$ErrorActionPreference='Stop'
Set-Location -LiteralPath (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$mediaOutput='scenarios/媒体表现/runtime/shared-preview'
$mediaNode='C:/Program Files/nodejs/node.exe'
$mediaPython='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
New-Item -ItemType Directory -Path "$mediaOutput/engine" -Force | Out-Null
& $mediaPython -B -X utf8 modules/media/p302-controlled/install_packages.py *> "$mediaOutput/install-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Standard packages failed'}
$mediaPinned=Get-Content scenarios/媒体表现/runtime/actual-libraries.json -Encoding utf8 | ConvertFrom-Json
# Keep the accepted, internally consistent Engine/dependency set while other owners build.
$mediaSim=$mediaPinned.simulation
$mediaDomain=$mediaPinned.domain
$mediaJson=$mediaPinned.serde_json
@{simulation=$mediaSim;domain=$mediaDomain;serde_json=$mediaJson}|ConvertTo-Json|Set-Content "$mediaOutput/actual-libraries.json" -Encoding utf8
$mediaPreviousOutput=$env:P302_TEST_OUTPUT
try {
  $env:P302_TEST_OUTPUT="$mediaOutput/engine"
  & 'C:/Users/fujunye/.cargo/bin/rustc.exe' --edition=2024 tests/planner/media/engine_consumer.rs -L dependency=target/debug/deps --extern "sf_simulation=$mediaSim" --extern "sf_domain=$mediaDomain" --extern "serde_json=$mediaJson" -o "$mediaOutput/engine_consumer.exe" *> "$mediaOutput/engine-build-final.raw.txt"
  if($LASTEXITCODE -ne 0){throw 'Original Engine consumer compile failed'}
} finally {$env:P302_TEST_OUTPUT=$mediaPreviousOutput}
& ./scenarios/媒体表现/runtime/shared-preview/engine_consumer.exe *> "$mediaOutput/original21-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Original21 Engine regression failed'}
& 'C:/Users/fujunye/.cargo/bin/rustc.exe' --edition=2024 tests/planner/media/preview_engine.rs -L dependency=target/debug/deps --extern "sf_simulation=$mediaSim" --extern "serde_json=$mediaJson" -o "$mediaOutput/preview_engine.exe" *> "$mediaOutput/prefix-build-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Actual prefix compile failed'}
& ./scenarios/媒体表现/runtime/shared-preview/preview_engine.exe *> "$mediaOutput/prefix-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Actual prefix expectations failed'}
& $mediaNode --test tests/planner/media/media.test.mjs *> "$mediaOutput/original9-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Original9 media regression failed'}
& $mediaNode --test tests/planner/media/shared-preview.test.mjs *> "$mediaOutput/shared-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Shared declaration/catalog/transition/package tests failed'}
& $mediaNode node_modules/typescript/bin/tsc --noEmit -p packages/presentation/tsconfig.json *> "$mediaOutput/package-typecheck-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Package strict TS failed'}
& $mediaNode node_modules/typescript/bin/tsc --noEmit --strict --skipLibCheck --target ES2022 --module ESNext --moduleResolution Bundler --jsx react-jsx apps/planner/src/features/media/index.ts apps/planner/src/features/media/harness.tsx apps/planner/src/features/media/sharedHarness.tsx *> "$mediaOutput/owned-entry-typecheck-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Owned media entries strict TS failed'}
& $mediaNode node_modules/typescript/bin/tsc --noEmit -p apps/planner/tsconfig.json *> "$mediaOutput/app-typecheck-final.raw.txt"
$mediaFullAppExit=$LASTEXITCODE
& $mediaNode node_modules/typescript/bin/tsc --noEmit --strict --skipLibCheck --target ES2022 --module ESNext --moduleResolution Bundler --jsx react-jsx tests/planner/media/mode-types.ts *> "$mediaOutput/mode-types-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Discriminated mode contract type checks failed'}
& $mediaNode tests/planner/media/build.mjs *> "$mediaOutput/build-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Independent harness build failed'}
@{original_engine_cases=21;original_shared_tests=9;new_shared_tests=7;separate_actual_prefix=1;owned_entry_tsc_exit=0;package_tsc_exit=0;mode_contract_tsc_exit=0;independent_build_exit=0;full_app_tsc_exit=$mediaFullAppExit}|ConvertTo-Json|Set-Content "$mediaOutput/verification-results.json" -Encoding utf8
if($mediaFullAppExit -ne 0){Write-Output 'Owned checks and build passed; full app TS remains red. See app-typecheck-final.raw.txt; no read-only owner files were modified.';exit $mediaFullAppExit}
Write-Output 'Original21 Engine + original9 checks retained; shared7 tests, separate actual prefix, strict TS and independent build passed. Browser/Native/industrial/NFR remain separately reported.'
