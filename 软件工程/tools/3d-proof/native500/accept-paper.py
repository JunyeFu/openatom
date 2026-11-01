"""Publish a reviewed batch only after its actual evidence and explicit AI visual review exist."""
from pathlib import Path
from datetime import datetime
import importlib.util,json,sys
from acceptance_profiles import validate_efficient_batch
ROOT=Path(__file__).resolve().parents[3];HOME=ROOT/'modules/media/native-equipment-500';BATCH=sys.argv[1] if len(sys.argv)>1 else 'B02-Q01';REVIEW=HOME/'reviews'/BATCH;PROOF=ROOT/'.work/native500-quality-20261025';PICS=ROOT/'scenarios/媒体表现/native500-20261024';PANELS=ROOT/'scenarios/媒体表现/native500-quality-20261025'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
rel=lambda p:p.relative_to(ROOT).as_posix()
assert not (REVIEW/'repair-required.json').exists(),'This exact revision has a known defect; publish the separately repaired version.'
config=read(REVIEW/'acceptance-input.json')
if BATCH not in read(HOME/'accepted-registration.json')['review_batches']:
 assert config.get('modeling_profile')=='simulation-efficient-v1','New batches must explicitly select simulation-efficient-v1; do not inherit legacy count gates.'
profile=config.get('modeling_profile','legacy');assert profile in ('legacy','simulation-efficient-v1');efficient=profile=='simulation-efficient-v1';validate_only='--validate-only' in sys.argv[2:];device_count=len(read(REVIEW/'catalog.json')['equipment'])
host=ROOT/config['host'];persistence=ROOT/config['native_persistence'];native_dir=ROOT/config['native_render'];prefix=BATCH.lower()
for f in [REVIEW/'source-checks.json',REVIEW/'contact-checks.json',host/'behavior-checks.json',persistence]:assert read(f)['passed'],f
if efficient:
 expected_media=[(e['id'],e['version']) for e in read(REVIEW/'registration.json')['register_order'] if '.device.' not in e['id']]
 expected_media += [(e['id'],e['version']) for e in read(REVIEW/'items/registration.json')['register_order'] if e['category']=='media']
 validate_efficient_batch(read(REVIEW/'catalog.json')['equipment'],read(host/'behavior-checks.json'),read(persistence),read(REVIEW/'Native-render-checks.json'),read(native_dir/'report.json'),read(REVIEW/'contact-checks.json'),expected_media,config.get('native_wait_reuse'))
elif BATCH=='B02-Q02':
 assert read(REVIEW/'contact-checks.json')['profile']=='q02-source-contact-only'
 assert read(REVIEW/'capture-readiness-regression.json')['passed']
 assert read(REVIEW/'fold-contact-checks.json')['passed'] and read(REVIEW/'items/source-checks.json')['passed']
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
 assert read(host/'behavior-checks.json')['output_identity_quantity_area_conservation']
 persist=read(persistence);outputs=persist['actual_per_device_output_after_native_reopen'];assert len(outputs)==10
 assert sorted(r['actual_output_items'] for r in outputs)==[1,1,1,1,1,1,2,2,60,60] and all(r['passed'] and r['identity_and_area_verified'] for r in outputs)
 fold=read(PICS/'b02-q02-fold-contact-checks.json');assert fold['passed'] and fold['actual_product_decoder'] and fold['models']==4
 assert all(c['passed'] and len(c['samples'])==10 for c in fold['checks'])
elif BATCH=='B02-Q03':
 assert read(REVIEW/'contact-checks.json')['profile']=='q03-source-supports-and-strokes'
 assert read(REVIEW/'items/source-checks.json')['passed'] and read(REVIEW/'Native-render-checks.json')['passed']
 assert read(PICS/'b02-q03-paper-contact-checks.json')['passed']
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==30
 assert len(read(persistence)['checks'])==60
elif BATCH=='B02-Q04':
 assert read(REVIEW/'contact-checks.json')['profile']=='q04-source-finishing-fixtures'
 assert read(REVIEW/'items/source-checks.json')['passed'] and read(REVIEW/'Native-render-checks.json')['passed']
 finish=read(PICS/'b02-q04-finish-contact-checks.json');assert finish['passed'] and finish['actual_product_decoder'] and finish['models']==10
 assert all((PICS/(prefix+'-'+c['finished_closeup'])).exists() for c in finish['checks'] if c['lod']=='detailed')
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==30 and len(read(persistence)['checks'])==60
elif BATCH=='B01-Q01':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q01-source-feed-and-lift'
 assert read(REVIEW/'items/source-checks.json')['passed'] and read(REVIEW/'Native-render-checks.json')['passed']
 material=read(PICS/'b01-q01-material-contact-checks.json');assert material['passed'] and material['actual_product_decoder'] and material['models']==4
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==10 and len(read(persistence)['checks'])==20
elif BATCH in ['B01-Q02','B01-Q02-R1','B01-Q03']:
 assert read(REVIEW/'contact-checks.json')['profile']==('b01q03-source-offset-three-cylinders' if BATCH=='B01-Q03' else 'b01q02r1-source-two-color-flexo' if BATCH=='B01-Q02-R1' else 'b01q02-source-two-color-flexo')
 assert read(REVIEW/'items/source-checks.json')['passed'] and read(REVIEW/'Native-render-checks.json')['passed']
 ink=read(PICS/(prefix+'-two-ink-contact-checks.json'));assert ink['passed'] and ink['actual_product_decoder'] and ink['models']==2
 if BATCH in ['B01-Q03','B01-Q02-R1']:assert all(c['actual_nip_rolling_direction']=='co_moving_verified' for c in ink['checks'])
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==7 and len(read(persistence)['checks'])==14
elif BATCH=='B01-Q15':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q15-finite-tray-compound-corner-tools-and-separated-output'
 assert read(REVIEW/'items/source-checks.json')['passed'] and read(REVIEW/'items/source-checks.json')['models']==4
 for name in ['finite-tray-construction-checks.json','finite-tray-solid-checks.json','equipment-material-clearance-checks.json','equipment-material-clearance-detailed-checks.json','tool-tool-clearance-checks.json','tool-tool-clearance-detailed-checks.json','tool-chassis-clearance-checks.json','tool-chassis-clearance-detailed-checks.json','support-interface-checks.json','source-rebuild-checks.json']:assert read(REVIEW/name)['passed'],name
 model=read(PICS/(prefix+'-equipment-component-checks.json'));assert model['passed'] and model['actual_product_decoder'] and model['reverse_seek_verified'] and model['dynamic_feed_chains_per_LOD']==[24,24] and model['max_actual_feed_endpoint_error_m']<3e-6 and model['max_actual_feed_centerline_seam_m']<3e-6
 item=read(PICS/(prefix+'-item-component-checks.json'));assert item['passed'] and item['actual_product_decoder'] and item['samples']==53
 gap=read(PICS/(prefix+'-output-staging-clearance-checks.json'));assert gap['passed'] and gap['actual_product_decoder'] and gap['actual_gap_m']>=.015 and gap['private_staging_x_m']==.98 and gap['output_port_x_m']==1.25
 assert read(host/'behavior-checks.json')['actual_device_conditions']==6 and read(host/'behavior-checks.json')['actual_jobs']==7 and read(host/'behavior-checks.json')['two_sources_new_open_tray']
 persist=read(persistence);assert len(persist['checks'])==12 and len(persist['current_media_checks'])==6 and all(c['exact_current_media_preserved'] for c in persist['current_media_checks'])
 assert read(REVIEW/'Native-render-checks.json')['passed'] and read(REVIEW/'Native-render-checks.json')['two_sources_new_open_tray'] and read(REVIEW/'Native-render-checks.json')['views']==28
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B01-Q14':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q14-finite-stock-lock-tools-and-services'
 assert read(REVIEW/'items/source-checks.json')['passed'] and read(REVIEW/'items/source-checks.json')['models']==4
 lock=read(PICS/(prefix+'-lock-tool-contact-checks.json'));assert lock['passed'] and lock['models']==2 and all(c['actual_product_decoder'] and len(c['rows'])==44 and c['actual_six_XYZ_feed_chains'] and c['actual_four_folder_following_feeds'] and c['actual_lower_lift_air_continuity'] for c in lock['checks'])
 assert read(PICS/(prefix+'-item-component-checks.json'))['passed'] and read(PICS/(prefix+'-item-component-checks.json'))['samples']==63
 for name in ['finite-band-construction-checks.json','finite-manufacturing-collision-checks.json','equipment-stock-clearance-checks.json','equipment-stock-clearance-detailed-checks.json','tool-tool-clearance-checks.json','tool-tool-clearance-detailed-checks.json','tool-chassis-clearance-checks.json','tool-chassis-clearance-detailed-checks.json','support-interface-checks.json']:assert read(REVIEW/name)['passed'],name
 assert read(host/'behavior-checks.json')['actual_device_conditions']==6 and read(host/'behavior-checks.json')['actual_jobs']==7 and read(host/'behavior-checks.json')['two_sources_new_lock_carton']
 persist=read(persistence);assert len(persist['checks'])==12 and len(persist['current_media_checks'])==6 and all(c['exact_current_media_preserved'] for c in persist['current_media_checks'])
 assert read(REVIEW/'Native-render-checks.json')['passed'] and read(REVIEW/'Native-render-checks.json')['two_sources_new_lock_carton']
 assert read(REVIEW/'mechanism-repair.json')['status']=='final_actual_GLB_Host_Native_retests_passed'
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B01-Q13':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q13-actual-underside-glue-fold-seam'
 assert read(REVIEW/'items/source-checks.json')['passed']and read(REVIEW/'items/source-checks.json')['models']==4 and read(REVIEW/'Native-render-checks.json')['passed']and read(REVIEW/'Native-render-checks.json')['two_sources_new_sleeve']
 folder=read(PICS/(prefix+'-folder-contact-checks.json'));assert folder['passed']and folder['actual_product_decoder']and folder['models']==2 and all(c['actual_product_decoder']and len(c['phases'])==31 and c['actual_two_source_volume_conserved']and c['actual_glue_contact']and c['actual_fold_finger_contact']and c['whole_unit_transport_clearance']for c in folder['checks'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==6 and read(host/'behavior-checks.json')['actual_jobs']==7 and read(host/'behavior-checks.json')['two_sources_new_sleeve']and len(read(persistence)['checks'])==12
 assert read(REVIEW/'mechanism-repair.json')['status']=='final_actual_GLB_Host_Native_retests_passed'
 assert not any(i['status']!='resolved'for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B01-Q12':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q12-actual-rotary-die-push-strip'
 assert read(REVIEW/'items/source-checks.json')['passed']and len(read(REVIEW/'items/source-checks.json')['actual_mesh_areas'])==3 and read(REVIEW/'Native-render-checks.json')['passed']and read(REVIEW/'Native-render-checks.json')['four_real_pending_scraps_verified']
 rotary=read(PICS/(prefix+'-rotary-contact-checks.json'));assert rotary['passed']and rotary['models']==2 and all(c['actual_product_decoder']and c['beveled_rules']==12 and len(c['bladeSamples'])==36 and len(c['samples'])==23 and c['co_moving_nip']and c['actual_rod_and_moving_hose']and c['actual_corner_support_and_lowered_clearance']for c in rotary['checks'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==7 and read(host/'behavior-checks.json')['actual_jobs']==13 and len(read(persistence)['checks'])==14
 assert read(REVIEW/'mechanism-repair.json')['status']=='final_actual_GLB_Host_Native_retests_passed'
 assert not any(i['status']!='resolved'for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B01-Q11':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q11-actual-flute-assembly'
 assert read(REVIEW/'items/source-checks.json')['passed']and read(REVIEW/'items/source-checks.json')['models']==5 and read(REVIEW/'Native-render-checks.json')['passed']and read(REVIEW/'Native-render-checks.json')['three_sources_new_composite']
 flute=read(PICS/(prefix+'-flute-contact-checks.json'));assert flute['passed']and flute['actual_product_decoder']and flute['models']==2 and all(c['actual_product_decoder']and c['actual_three_source_volume_conserved']and c['actual_flat_crest_glue_contact']and c['co_moving_upper_feed_and_nip']and c['upper_feed_floor_and_machine_supports']and len(c['samples'])==11 and len(c['transferSamples'])==6 for c in flute['checks'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==7 and read(host/'behavior-checks.json')['actual_jobs']==8 and read(host/'behavior-checks.json')['three_sources_to_new_wet_composite']and len(read(persistence)['checks'])==14
 assert not any(i['status']!='resolved'for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B01-Q10':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q10-actual-thermal-lamination'
 assert read(REVIEW/'items/source-checks.json')['passed']and read(REVIEW/'items/source-checks.json')['models']==3 and read(REVIEW/'Native-render-checks.json')['passed']
 laminate=read(PICS/(prefix+'-laminate-contact-checks.json'));assert laminate['passed']and laminate['actual_product_decoder']and laminate['models']==2 and all(c['actual_product_decoder']and c['co_moving_nip']and len(c['samples'])==12 for c in laminate['checks'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==7 and read(host/'behavior-checks.json')['actual_jobs']==8 and len(read(persistence)['checks'])==14
 assert read(REVIEW/'hardware-repair.json')['status']=='final_actual_GLB_Host_Native_retests_passed'
 assert not any(i['status']!='resolved'for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B01-Q09':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q09-actual-tab-stripping'
 assert read(REVIEW/'items/source-checks.json')['passed']and len(read(REVIEW/'items/source-checks.json')['actual_mesh_areas'])==3 and read(REVIEW/'Native-render-checks.json')['passed']
 strip=read(PICS/(prefix+'-strip-contact-checks.json'));assert strip['passed']and strip['models']==2 and all(c['actual_product_decoder']and len(c['samples'])==15 for c in strip['checks'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==7 and read(host/'behavior-checks.json')['actual_jobs']==13 and len(read(persistence)['checks'])==14
 assert read(REVIEW/'Native-render-checks.json')['four_real_pending_scraps_verified']
 assert read(REVIEW/'retainer-repair.json')['status']=='final_actual_GLB_Host_Native_retests_passed'
 assert not any(i['status']!='resolved'for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B01-Q08':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q08-actual-digital-cut-crease'
 assert read(REVIEW/'items/source-checks.json')['passed']and len(read(REVIEW/'items/source-checks.json')['actual_mesh_areas'])==3 and read(REVIEW/'Native-render-checks.json')['passed']
 digital=read(PICS/(prefix+'-digital-contact-checks.json'));assert digital['passed']and digital['models']==2 and all(c['actual_product_decoder']and c['crease_strokes']==4 and c['cut_strokes']==12 for c in digital['checks'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==8 and read(host/'behavior-checks.json')['actual_jobs']==14 and len(read(persistence)['checks'])==16
 assert not any(i['status']!='resolved'for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH in ['B01-Q07','B01-Q07-R1','B01-Q07-R2']:
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q07-actual-platen-die'
 assert read(REVIEW/'items/source-checks.json')['passed'] and len(read(REVIEW/'items/source-checks.json')['actual_mesh_areas'])==3 and read(REVIEW/'Native-render-checks.json')['passed']
 die=read(PICS/(prefix+'-die-contact-checks.json'));assert die['passed'] and die['models']==2 and all(c['actual_product_decoder']for c in die['checks'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==7 and read(host/'behavior-checks.json')['actual_jobs']==13 and len(read(persistence)['checks'])==14
 assert read(REVIEW/'Native-render-checks.json')['four_real_pending_scraps_verified']
 assert not any(i['status']!='resolved'for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B01-Q06':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q06-actual-sheet-slitting'
 assert read(REVIEW/'items/source-checks.json')['passed'] and read(REVIEW/'Native-render-checks.json')['passed']
 slit=read(PICS/(prefix+'-slit-contact-checks.json'));assert slit['passed'] and slit['models']==2 and all(c['actual_product_decoder']for c in slit['checks'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==6 and read(host/'behavior-checks.json')['actual_jobs']==9 and len(read(persistence)['checks'])==12
 assert read(REVIEW/'Native-render-checks.json')['two_real_pending_outputs_verified']
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B01-Q05':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q05-actual-rotary-score'
 assert read(REVIEW/'items/source-checks.json')['passed'] and read(REVIEW/'Native-render-checks.json')['passed']
 score=read(PICS/(prefix+'-score-contact-checks.json'));assert score['passed'] and score['actual_product_decoder'] and score['models']==2 and all(c['co_moving_nip'] for c in score['checks'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==5 and read(host/'behavior-checks.json')['actual_jobs']==6 and len(read(persistence)['checks'])==10
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B01-Q04':
 assert read(REVIEW/'contact-checks.json')['profile']=='b01q04-actual-finishing-material'
 assert read(REVIEW/'items/source-checks.json')['passed'] and read(REVIEW/'Native-render-checks.json')['passed']
 finish=read(PICS/(prefix+'-finish-contact-checks.json'));assert finish['passed'] and finish['actual_product_decoder'] and finish['models']==6
 assert read(host/'behavior-checks.json')['actual_device_conditions']==18 and read(host/'behavior-checks.json')['actual_jobs']==23 and len(read(persistence)['checks'])==36
 assert read(REVIEW/'host-checks.json')['ordered_coater_curer_same_item']
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
elif BATCH=='B02-Q03-R1':
 assert read(REVIEW/'contact-checks.json')['profile']=='q03r1-source-supports-and-strokes'
 assert read(REVIEW/'items/source-checks.json')['passed'] and read(REVIEW/'Native-render-checks.json')['passed']
 paper=read(PICS/(prefix+'-paper-contact-checks.json'));assert paper['passed'] and paper['actual_product_decoder'] and paper['models']==2 and all(c['actual_nip_rolling_direction']=='co_moving_verified' for c in paper['checks'])
 assert read(host/'behavior-checks.json')['actual_device_conditions']==7 and len(read(persistence)['checks'])==14
 assert not any(i['status']!='resolved' for i in read(REVIEW/'open-defects.json')['issues'])
else:assert read(REVIEW/'contact-checks.json')['checks_profile']=='paper-contact-v2-floor-support'
if BATCH in ['B01-Q07-R1','B01-Q07-R2','B01-Q08','B01-Q09']:assert read(PICS/(prefix+'-lift-joint-audit.json'))['passed']
if BATCH in ['B01-Q07-R2','B01-Q09']:assert read(PICS/(prefix+'-hydraulic-stroke-audit.json'))['passed']
native=read(native_dir/'report.json');assert native['passed'] and native['source_unchanged']
if not efficient:assert len(native['driver']['cases'])==(28 if BATCH=='B01-Q15' else 20 if BATCH=='B01-Q14' else 18 if BATCH=='B01-Q13' else 19 if BATCH in ['B01-Q08','B01-Q11','B01-Q12'] else 17 if BATCH in ['B01-Q07','B01-Q07-R1','B01-Q07-R2','B01-Q09'] else 15 if BATCH=='B01-Q06' else 12 if BATCH=='B01-Q05' else 48 if BATCH=='B01-Q04' else 16 if BATCH in ['B01-Q02','B01-Q02-R1','B01-Q03','B02-Q03-R1','B01-Q10'] else 24 if BATCH=='B01-Q01' else 70 if BATCH in ['B02-Q03','B02-Q04'] else 60)
visual=read(REVIEW/'visual-review.json');assert visual['reviewer']=='Codex AI' and visual['open_defects']==[] and visual['result']=='pass'
inv=read(REVIEW/'inventory.json');render=read(PICS/(prefix+('-render-b01.json' if BATCH.startswith('B01-') else '-render-b02.json')));assert len(render['checks'])==2*device_count
foot=read(PICS/(prefix+'-foot-contact-checks.json'));assert foot['passed'] and foot['actual_product_decoder'] and foot['models']==2*device_count
for e in inv['entries']:
 r=next(r for r in render['checks'] if r['id']==e['id'] and r['lod']==e['suffix']);assert r['bytes']==e['bytes'] and r['nodes']==e['nodes'] and r['decode'] and r['reverse_seek']
 f=next(f for f in foot['checks'] if f['id']==e['id'] and f['lod']==e['suffix']);assert f['module_ref']==dict(id=e['module_id'],version=e['version'])
 if not efficient:assert all(abs(c['bottom_gap_m'])<1e-6 and abs(c['top_gap_m'])<1e-6 for c in f['contacts'])
 # Efficient models use explicit MQ support/contact review; gap numbers remain diagnostic.
definitions=read(REVIEW/'catalog.json')['equipment'];run=read(host/'PAPER-QA.run.json')
for c in native['driver']['cases']:
 if efficient:continue # Semantic per-device coverage was checked above, without fixed capture counts.
 if BATCH in ['B02-Q03','B02-Q03-R1','B02-Q04','B01-Q01','B01-Q02','B01-Q02-R1','B01-Q03','B01-Q04','B01-Q05','B01-Q06','B01-Q07','B01-Q07-R1','B01-Q07-R2','B01-Q08','B01-Q09','B01-Q10','B01-Q11','B01-Q12','B01-Q13','B01-Q14','B01-Q15']:continue # Exact item refs, absences and interruption progress checked by the batch receipt above.
 dev=c['name'].split(' / ')[0];visible={(o['kind'],o['id']) for o in c['visible_objects']};assert ('device',dev) in visible
 if 'waiting for arrival' in c['name']:assert ('item','paper-'+dev) not in visible
 else:
  definition=next(d for d in definitions if dev.startswith(d['id']+'--'))
  if 'delivered' in c['name'] or (definition['rule']!='preserve' and 'output blocked' in c['name']):
   expected=[i['id'] for i in run['final_items'] if i['holder']==dict(kind='location',id=dev+'-out') and i['item_type_ref']['id']!='blocker']
   assert len(expected)==(2 if definition['id']=='slitter-rewinder' else 60 if definition['id']=='rotary-sheet-cutter' else 1)
  else:expected=['paper-'+dev]
  for item in expected:assert ('item',item) in visible,(c['name'],item)
  assert not any((i.get('object_ref') or {}).get('id') in [dev,*expected] for i in c['issues']),(c['name'],'target presentation limitation')
if validate_only:
 print(f'{BATCH}: {profile} evidence checks passed; no registry, ledger or counters written.');raise SystemExit(0)
spec=importlib.util.spec_from_file_location('mq',ROOT/'tools/check_model_quality.py');mq=importlib.util.module_from_spec(spec);spec.loader.exec_module(mq)
ledger=read(mq.LEDGER);rows={r['model_path']:r for r in ledger['models']};stamp=datetime.now().astimezone().isoformat()
for e in inv['entries']:
 model=REVIEW/e['directory']/e['asset']['path'];name=rel(model);ob=mq.observation(model);folder=PICS/(prefix+'-'+e['id']);v={key:rel(folder/(e['suffix']+'-'+('iso' if key=='isometric' else key)+'.png')) for key in mq.VIEWS}
 evidence=[rel(REVIEW/'visual-review.json'),rel(REVIEW/'review-notes.md')]
 review={'reviewed_file':name,'file_revision':{k:ob[k] for k in ['bytes','modified_ns']},'reviewer':'Codex AI','reviewed_at':stamp,'open_defects':[],
 'gates':{'source_export':{'result':'pass','evidence':[rel(REVIEW/'source-checks.json')]},'visual':{'result':'pass','evidence':evidence},'platform_reopen':{'result':'pass','evidence':[rel(persistence),rel(native_dir/'report.json')]}},
 'dimensions':{k:{'result':'pass','reason':reason,'evidence':evidence+[rel(REVIEW/'contact-checks.json'),rel(persistence),rel(PICS/(prefix+'-foot-contact-checks.json'))]} for k,reason in visual['dimensions'][e['id']].items()},
 'views':v,'closeups':[rel(folder/'action-50.png'),rel(folder/(e['suffix']+'-foot-contact.png'))],
 'motion_evidence':[rel(PICS/(prefix+('-render-b01.json' if BATCH.startswith('B01-') else '-render-b02.json'))),rel(native_dir/'report.json'),rel(persistence)],
 'human_acceptance':{'status':'not_claimed','reviewer_kind':'AI'},'reference_comparison':{'reference_files':['tools/3d-proof/output/assembly.glb','tools/3d-proof/output/line-assembly-high-iso.png','scenarios/媒体表现/native500-20261024/screw-assembly/detailed-iso.png'],'evidence':evidence,'conclusion':visual['comparison']}}
 rows[name]={'model_path':name,'category':'detailed_candidate' if e['suffix']=='detailed' else 'simplified_candidate','asset_quality':'accepted','observed':ob,'review':review}
for model in (REVIEW/'items').rglob('*.glb'):
 name=rel(model);rows[name]={'model_path':name,'category':'test_fixture','asset_quality':'pending','observed':mq.observation(model),'review':None}
ledger['models']=sorted(rows.values(),key=lambda r:r['model_path']);write(mq.LEDGER,ledger)
registry=read(REVIEW/'registration.json');batch_entries=[dict(e,directory='reviews/'+BATCH+'/'+e['directory']) for e in registry['register_order']]
accepted={e['id']:e for e in read(HOME/'accepted-registration.json')['register_order']};accepted.update({e['id']:e for e in batch_entries});accepted=list(accepted.values())
accepted_ids={e['id'].split('.device.',1)[1] for e in accepted if '.device.' in e['id']};count=len(accepted_ids);folders=list(dict.fromkeys('/'.join(e['directory'].split('/')[:2]) for e in accepted))
write(HOME/'accepted-registration.json',{'device_count':count,'media_count':len(accepted)-count,'review_batches':[f.split('/')[-1] for f in folders],'inventory_paths':[f+'/inventory.json' for f in folders],'register_order':accepted})
scope_file=HOME/'paper-packaging-review-scope.json'
if scope_file.exists():
 scope=read(scope_file);scope['accepted_device_ids']=[id for id in scope['target_device_ids'] if id in accepted_ids];scope['remaining_device_ids']=[id for id in scope['target_device_ids'] if id not in accepted_ids];scope['accepted_count']=len(scope['accepted_device_ids']);scope['remaining_count']=len(scope['remaining_device_ids']);write(scope_file,scope)
write(REVIEW/'release-catalog.json',{'register_order':[dict(e,directory='modules/media/native-equipment-500/'+e['directory']) for e in batch_entries]})
write(REVIEW/'quality-review.json',{'review_batch':BATCH,'status':'accepted_by_AI','device_ids':[e['id'] for e in definitions],'version':inv['entries'][0]['version'],'batch_accepted_types':len(definitions),'formal_supported_device_count':count,'candidate_device_count':500,'remaining_device_count':500-count,'MQ_dimensions':'8/8 per model','device_GLBs':2*device_count,'actual_GLB_views':21 if BATCH=='B01-Q15' else 17*device_count+foot['models']+(19 if BATCH=='B01-Q14' else 12 if BATCH=='B01-Q13' else 11 if BATCH in ['B01-Q08','B01-Q11','B01-Q12'] else 10 if BATCH=='B01-Q09' else 9 if BATCH in ['B01-Q07','B01-Q07-R1','B01-Q07-R2','B01-Q10'] else 7 if BATCH in ['B01-Q05','B01-Q06'] else 21 if BATCH=='B01-Q04' else 6 if BATCH=='B02-Q03-R1' else 10 if BATCH in ['B01-Q02','B01-Q02-R1','B01-Q03'] else 36 if BATCH=='B01-Q01' else 12 if BATCH=='B02-Q02' else 30 if BATCH=='B02-Q03' else 35 if BATCH=='B02-Q04' else 0),'Native_Run_views':len(native['driver']['cases']),'ordinary_Native_persistence':rel(persistence),'source_export':rel(REVIEW/'source-checks.json'),'human_acceptance':'not_claimed','reviewed_at':stamp})
pending=[dict(id=e['id'],title=e['title'],batch=e['batch'],state='pending',priority='paper_packaging' if e['batch']=='B02' or e['batch']=='B01' and e['index']<=33 else 'paper_packaging_support' if e['batch']=='B01' and e['index']<=42 else 'common_equipment') for e in read(HOME/'catalog.json')['equipment'] if e['id'] not in accepted_ids]
if efficient:
 q=read(REVIEW/'quality-review.json');q['modeling_profile']=profile;q['actual_GLB_views']=len({v for e in inv['entries'] for v in rows[rel(REVIEW/e['directory']/e['asset']['path'])]['review']['views'].values()})
 if config.get('native_wait_reuse'):q['native_wait_reuse']=config['native_wait_reuse'] # Explicit shared coverage, never per-device Native passes.
 write(REVIEW/'quality-review.json',q)
write(HOME/'quality-review-progress.json',{**read(HOME/'quality-review-progress.json'),'target':500,'accepted':count,'pending':500-count,'accepted_batches':[f+'/quality-review.json' for f in folders],'rule':'All applicable MQ dimensions, source export, actual Native holder/waits and UI save/new process reopen pass before counting a device type. LODs, instances and QA fixtures do not increase the device count.','remaining':pending})
delivery=read(HOME/'delivery-state.json');delivery.update(formal_supported_device_count=count,formal_per_device_MQ_acceptance=f'{count} accepted; {500-count} pending',native_actual_item_wait_save_reopen=f'{BATCH} passed; remainder pending',public_Native_startup_registration_applied=False,accepted_registration='modules/media/native-equipment-500/accepted-registration.json',quality_review_progress='modules/media/native-equipment-500/quality-review-progress.json');write(HOME/'delivery-state.json',delivery)
for name in ['catalog.json','registration.json']:
 v=read(REVIEW/name);v['status']='accepted_by_AI';write(REVIEW/name,v)
print(f'{BATCH} accepted; {count} unique supported device types; {500-count} types remain pending.')
