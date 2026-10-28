from pathlib import Path
import json

ROOT=Path(__file__).resolve().parents[4]
HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q07'
attempt=ROOT/'.work/native500-quality-20261025/b02q07/attempt-01'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
run=read(attempt/'PAPER-QA.run.json'); receipt=read(attempt/'host-receipt.json')
checks=[]; captures=[]
quantity=lambda n:dict(unit='piece',value=dict(coefficient=n,decimal_places=0))
value=lambda item,key:item['attributes'][key]['value']
assert receipt['fresh_Host_save_reopen'] and run['whole_window_computed']
assert all(job['state']=='completed' for job in run['jobs'])

for spec in read(HOME/'catalog.json')['equipment']:
    key=spec['id']; hive=spec['rule']=='assemble'; total=spec['cycle_seconds']*1000
    load=round(total*.2); auto=round(total*.6); rows={}
    conditions=['normal','missing','phase-wait','interrupted','output-wait','missing-auxiliary']+spec['acceptance_conditions']
    for condition in conditions:
        dev=key+'--'+condition
        job=next(j for j in run['jobs'] if j['execution_ref']['data']['device_id']==dev)
        assert job['operation_ref']==dict(id=spec['definition_id'],version=spec['version'])
        expected={'normal':None,'missing':('missing_material',0,8000),'missing-component':('missing_material',0,8000),'missing-auxiliary':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',load+int(auto*.4),load+int(auto*.4)+5000),'output-wait':('output_capacity',total,40000),'waste-wait':('output_capacity',total,40000)}[condition]
        waits=[w for w in run['waits'] if w['object_ref']['id']==job['id']]
        if expected:
            kind,a,b=expected
            wait=next(w for w in waits if any(r['kind']==kind for r in w['reasons']))
            assert wait['interval']==dict(start_ms=a,end_ms=b) and wait['end_cause']=='resolved',(dev,wait)
        else:
            assert not waits
        finish=40000 if condition in ['output-wait','waste-wait'] else total+(8000 if condition.startswith('missing') else 5000 if condition in ['phase-wait','interrupted'] else 0)
        assert job['finished_at_ms']==finish
        assert sum(p['interval']['end_ms']-p['interval']['start_ms'] for p in run['phase_work_intervals'] if p['job_id']==job['id'])==total
        sources=['paper-'+dev]+(['component-'+dev] if hive else [])+['auxiliary-'+dev]
        by_id={i['id']:i for i in run['final_items']}
        raw=by_id[sources[0]]
        assert raw['quantity']==quantity(spec['input_quantity'])
        assert all(by_id[uid]['lifecycle']=='transformed' for uid in sources)
        assert by_id['auxiliary-'+dev]['quantity']==quantity(1)
        outputs=[i for i in run['final_items'] if i['item_type_ref']['id']==key+'-output' and raw['id'] in i['composition_refs']]
        waste=[i for i in run['final_items'] if i['item_type_ref']['id']==key+'-waste' and raw['id'] in i['composition_refs']]
        assert len(outputs)==1 and len(waste)==int(not hive)
        for item in outputs+waste:
            assert item['holder']==dict(kind='location',id=dev+('-waste' if item in waste else '-out'))
            assert item['lifecycle']=='available' and item['quality']=='released' and item['quantity']==quantity(1) and not item['contents']
            assert set(item['composition_refs'])==set(sources)
            assert value(item,'presentation_state')==('collected_angle_end_waste' if item in waste else spec['final'])
        glue=by_id['auxiliary-'+dev]
        if hive:
            faces=by_id['component-'+dev]
            assert faces['quantity']==quantity(2) and value(faces,'sheets_in_visual_lot')==2
            assert abs(value(outputs[0],'thickness_m')-.022)<1e-12 and value(outputs[0],'face_sheet_quantity')==2
            assert value(glue,'glue_ml')==2 and raw['item_type_ref']['id']==key+'-input'
        else:
            assert value(raw,'sheets_in_visual_lot')==4 and value(outputs[0],'laminate_ply_count')==4
            assert value(glue,'glue_ml')==1.5 and abs(value(outputs[0],'length_m')-.600)<1e-12
            # Business quantities use declared specification, independent of the approximate visual mesh.
            assert abs(4*value(raw,'paper_volume_per_piece_m3')-value(outputs[0],'paper_volume_m3')-value(waste[0],'removed_volume_m3'))<1e-12
        rows[condition]={'job_id':job['id'],'wait':expected,'finished_ms':finish,'effective_work_ms':total,'input_id':raw['id'],'output_ids':[i['id'] for i in outputs],'waste_ids':[i['id'] for i in waste],'source_ids':sources,'destination':dev+'-out','identity_quantities_origins_quality_verified':True,'passed':True}

        def capture(label,at,lod='detailed',eye=None,target=None):
            view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+dev,'object_ref':{'kind':'line','id':'line-'+dev},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':eye or dict(x=6.8,y=7.3,z=3.8),'target':target or dict(x=3,y=3.2,z=1),'zoom':1}}
            captures.append({'name':dev+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+dev},'preview':None,'delivery':'capture_only'}})

        at=load+round(auto*(.70 if hive else .50)) if condition=='normal' else round((expected[1]+expected[2])/2)
        capture('actual '+condition,at)
        if condition=='normal':
            capture('simple processing',at,'simplified')
            capture('finished package closeup',finish+1,eye=dict(x=5.02,y=2.55,z=1.14),target=dict(x=4.32,y=3,z=.93))
            if hive:
                capture('open core before upper face',load+round(auto*.20),eye=dict(x=3.50,y=2.50,z=1.40),target=dict(x=3,y=3,z=.92))
    checks.append({'id':key,'version':spec['version'],'conditions':rows})

write(attempt/'behavior-checks.json',{'passed':True,'scope':'real Host/new Host reopen; 1 core + face batch quantity 2 + glue to panel; paper batch quantity 4 + glue to angle and waste','run_id':run['id'],'checks':checks,'fresh_Host_save_reopen':True,'actual_device_conditions':sum(len(c['conditions']) for c in checks),'actual_jobs':len(run['jobs']),'capture_framing':{'honeycomb-laminator--normal / open core before upper face':{'outside_frame':['auxiliary-honeycomb-laminator--normal'],'reason':'core/face closeup; auxiliary holder is checked in the normal wide capture and both ordinary processes'}}})
write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':captures})
write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'device_conditions':sum(len(c['conditions']) for c in checks),'jobs':len(run['jobs']),'Native':'pending'})
print(f'Q07: {len(run["jobs"])} completed Jobs, {sum(len(c["conditions"]) for c in checks)} conditions, {len(captures)} Native states planned.')
