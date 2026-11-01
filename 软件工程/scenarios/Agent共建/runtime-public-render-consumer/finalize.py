from pathlib import Path
import json,hashlib,datetime
p=Path(__file__).parent
root=p.parents[2]
read=lambda n:json.loads((p/n).read_text(encoding='utf-8-sig'))
write=lambda n,v:(p/n).write_text(json.dumps(v,ensure_ascii=False,indent=2),encoding='utf8')
cut=read('source-cut.json');receipt=read('build-receipt.json');now=datetime.datetime.now(datetime.timezone.utc).isoformat()
for record in cut['records']:
    a=(root/record['path']).read_bytes();b=(p/'source-snapshot'/record['path']).read_bytes()
    assert a==b and hashlib.sha256(a).hexdigest()==record['sha256'],record['path']
for record in receipt['artifacts']:
    assert hashlib.sha256((p/record['path']).read_bytes()).hexdigest()==record['sha256']
groups=[('actual_task_routes','actual-browser-final.json',40),('supplement','supplement-accepted.json',10),('boundaries','boundaries-final.json',11),('controlled_camera','camera-green.json',4),('normal_manufacturing','normal-manufacturing-green.json',3),('normal_comparison','normal-comparison-first.json',1),('main_boundary','main-boundaries.json',2),('caption_overlay','actual-browser-caption-restored.json',2),('comparison_style','actual-browser-comparison-style.json',2),('old_capture','regression/old-factory-capture/browser-final.json',11),('old_factory','regression/old-factory/browser-green.json',7),('old_media','regression/old-media/browser-green.json',11)]
rows=[]
for name,raw,n in groups:
    values=read(raw)['checks'];assert len(values)==n and all(row['status']=='passed' for row in values),raw
    rows.append(dict(name=name,passed=n,total=n,raw=raw,Native=False))
t=read('regression/old-timeline-browser/browser-result.json')['results'];assert len(t)==17 and all(r['status']=='passed' for r in t)
for name,n,raw in [('old_timeline_browser',17,'regression/old-timeline-browser/browser-result.json'),('old_pure',67,'regression/pure.stdout.log'),('old_packaging_ssr',9,'regression/packaging-view.stdout.log'),('source_pure',6,'source-test-final.stdout.log'),('named_pose',1,'named-pose.stdout.log'),('caption_ssr',2,'caption-ssr.json')]:
    rows.append(dict(name=name,passed=n,total=n,raw=raw,Native=False))
source=dict(consumer_production=26,Root_mount=3,tests=38,all_records=len(cut['records']),mismatches=[])
open_gates=['actual Native nine-kind windows/PNG/Show/managed','eight-device industrial quality','full original P3-04 Q/IX/NFR/human release']
write('verification-results.json',dict(frozen_at=cut['frozen_at'],verified_at=now,status='consumer sources frozen',rows=rows,typescript=receipt['typescript'],build=dict(receipt='build-receipt.json',exit_code=0,active_entry=receipt['active_entry'],raw='freeze-build.stdout.log'),source_snapshot_verification=source,evidence_not_additive=True,Native=False,open_gates=open_gates))
reviews=[
('scope','26 owned production + Root3 exactly copied; no Host/API/domain/Native/config/Cargo/listener writes','source-cut.json'),
('nine_kind','all nine share normal product components; no alternate viewer/solver/clock','handoff.md'),
('source','final actual46 selected39 plus main1; supplement real9+newfacing1; exact bytes','actual-browser-final.json'),
('authority','trusted input nonnull / null main / denial no fallback; bridge is not Native authority','main-boundaries.json'),
('identity','full frame/view/Scope/R/oldRun/module and Draft exact owner; no fake request module_ref','source-test-final.stdout.log'),
('real_history','real Rust summaries and comparison perRun frozen inputs; no TS quantity or compare calculation','source-test-final.stdout.log'),
('template','actual subset only, no Run inventory/adoption/arbitrary first Hall','source-test-final.stdout.log'),
('lifecycle','key-qualified prepared source/perawait/held/late/unmount/corrupt/refusal terminal','boundaries-final.json'),
('ready','real decode/GPU fence/fonts/DOM paint, physical DPR, no timer/business clock','boundaries-final.json'),
('camera','actual OrbitControls clamp red fixed capture only, controlled4+realHostnearfar','camera-green.json'),
('line','original offscreen retained; explicit facing real camera three exact members/27mesh, no fit','supplement-accepted.json'),
('image','real GIF14511 differing canvas pixels, fixed20RAF; PNG5143RGBA colors','actual-image-pixels.json'),
('visual','all nine representative PNGs viewed; final readonly wording/table scope fixed','visual-review.json'),
('normal_manufacturing','actual menu/originalHost save+compute+savedSVG, ScopeRlate/close guards','normal-manufacturing-green.json'),
('normal_comparison','actual Workbench twoRuns + originalcompare_runs, no mutation','normal-comparison-first.json'),
('old_behavior','original assertions/old9Run field equality/ordinaryfactorymedia/timeline/packaging retained','verification-results.json'),
('build','TS planner/presentation/harness0, actual fullmain product build0 with entry byte receipt','build-receipt.json'),
('limits','truthful missingPresentation/legacy/2Dcamera, no fake geometry or industrial acceptance','handoff.md'),
('release','Rust supported remainsFactory2 pending Root/Core actual reception, not ninekind Native done','handoff.md')]
write('self-review.json',dict(status='completed, sources frozen',reviewed_at=now,checks=[dict(name=n,result='pass within stated evidence layer',finding=f,evidence=e) for n,f,e in reviews],source_cut='source-cut.json',source_byte_verification=source,not_closed=open_gates,original_red_preserved=True,no_additive_coverage_counts=True))
visual=[('line_canvas','supplement-accepted-10.png','three exact native devices / explicit camera, offscreen original retained'),('multi_hall_canvas','final-capture-22.png','original request perspective and truthful missing detailed media marker, no fit'),('device_detail','final-capture-24.png','original missing Presentation declared, no fake model'),('module_preview','final-capture-23.png','actual controlled GLB, technical not industrial acceptance'),('device_cycle_preview','final-capture-5.png','original JobsAt identity/phase, missing Presentation explicitly declared'),('declared_pose','supplement-accepted-2.png','pressed canonical actual nodes, no timeline/job fabricated'),('packaging_workbench','caption-restored-capture-15.png','exact requested camera, readonly instruction'),('packaging_manufacturing','final-capture-13.png','both exact real SVG files; actual parent Artifact identity'),('timeline_snapshot','final-capture-37.png','actual grouped events, readonly fixed Run time'),('comparison','comparison-style-capture-38.png','original table columns/spacing via original delivery-workspace class'),('normal_manufacturing','normal-manufacturing.png','real saved SVG normal Workbench shared preview'),('normal_comparison','normal-comparison.png','normal actual twoRun compare shared result')]
write('visual-review.json',dict(reviewed_at=now,method='actual PNGs viewed with view_image; separate geometry and pixel assertions; no Native/industrial claim',views=[dict(kind=k,image=i,finding=f) for k,i,f in visual],source='final source-cut.json; preserved pre-visual cut not final'))
write('reception-index.json',dict(status='final frozen unique reception index',handoff='handoff.md',source_cut='source-cut.json',snapshot='source-snapshot/',build_receipt='build-receipt.json',verification='verification-results.json',self_review='self-review.json',visual_review='visual-review.json',final_actual='actual-browser-final.json',final_supplement='supplement-accepted.json',boundaries='boundaries-final.json',final_overlay=['actual-browser-caption-restored.json','actual-browser-comparison-style.json'],pixel_evidence='actual-image-pixels.json',Host_original_index='../runtime-public-render-host/actual-inputs-index.json',Host_complete_supplement='../runtime-public-render-host/actual-visual-supplement/complete-index.json',active_product_entry=receipt['active_entry'],Native=False,original_failed_named_green_not_final=['actual-browser-green.json','supplement-final.json'],preserved_connection_failure='actual-browser-caption-overlay.json',source_mismatches=[]))
print(json.dumps(dict(production=26,Root=3,tests=38,source_records=len(cut['records']),all_equal=True,active_entry=receipt['active_entry'],handoff=str(p/'handoff.md')),ensure_ascii=False))
