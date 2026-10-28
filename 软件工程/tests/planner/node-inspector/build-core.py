"""Link the approved frozen item-layout + CapturePreview cut. No Cargo invocation."""
import json,subprocess
from pathlib import Path
root=Path.cwd();out=root/'scenarios/媒体表现/runtime-node-inspector'
core=root/'target/debug/deps/libsf_commands-e579663d2bd6c8b8.rlib'
finger=json.loads((root/'target/debug/.fingerprint/sf-commands-e579663d2bd6c8b8/lib-sf_commands.json').read_text(encoding='utf-8'))
expected=next(d[-1] for d in finger['deps'] if d[1]=='serde_json').to_bytes(8,'little').hex()
matches=[p for p in (root/'target/debug/.fingerprint').glob('serde_json-*/lib-serde_json') if p.read_text(encoding='utf-8').strip()==expected];assert len(matches)==1
command=['C:/Users/fujunye/.cargo/bin/rustc.exe','--edition','2024',str(root/'tests/planner/node-inspector/core_consumer.rs'),'-L','dependency=target/debug/deps','--extern',f'sf_commands={core}','--extern',f'serde_json={root/"target/debug/deps"/("lib"+matches[0].parent.name+".rlib")}','-o',str(out/'core_consumer.exe')]
r=subprocess.run(command,capture_output=True,text=True,encoding='utf-8');(out/'core-build.stdout.txt').write_text(r.stdout,encoding='utf-8');(out/'core-build.stderr.txt').write_text(r.stderr,encoding='utf-8');(out/'core-build.json').write_text(json.dumps({'command':command,'exit_code':r.returncode,'Core_mtime':core.stat().st_mtime,'cut':'Root approved Node8 + frozen-instance4 cut; exact libsf_commands-e579663d2bd6c8b8.rlib','Cargo_used':False},ensure_ascii=False,indent=2),encoding='utf-8');raise SystemExit(r.returncode)
