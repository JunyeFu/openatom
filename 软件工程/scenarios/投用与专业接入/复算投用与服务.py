"""本轮投用、公用容量与专业契约样例的有限算术，不是生产或专业求解引擎。"""
from pathlib import Path
import json

B=Path(__file__).parent
H=3600000;D=24*H;M=60000

def commissioning(cabinet_day,window_hours):
    # 计划日历跨度，不等同施工人员劳动时长。
    mechanical_ready=3*D
    cabinet_ready=cabinet_day*D
    wiring_ready=cabinet_ready+D
    trial_start=max(mechanical_ready,wiring_ready)
    ready=trial_start+D//2
    origin=4*D+8*H
    shifts=[(origin+i*D,origin+i*D+8*H) for i in range(4)]
    jobs=[];production_start=max(origin,ready)
    for a,b in shifts:
        if max(a,production_start)+70*M<=b:
            production_start=max(a,production_start);break
    else:production_start=None
    completion=None if production_start is None else production_start+70*M-origin
    if production_start is not None:
        for i in range(2):
            start=production_start+i*30*M-origin
            jobs.extend([dict(kind='assembly',item=i+1,start_ms=start,end_ms=start+30*M,resources=['W-A','Assembly']),
                         dict(kind='inspection',item=i+1,start_ms=start+30*M,end_ms=start+35*M,resources=['W-B','Inspection']),
                         dict(kind='packing',item=i+1,start_ms=start+35*M,end_ms=start+40*M,resources=['W-B','Packing'])])
    observed=[j for j in jobs if j['kind']=='packing' and j['end_ms']<=window_hours*H]
    assert all(j['start_ms']+origin>=ready for j in jobs)
    assert all(any(a<=j['start_ms']+origin<j['end_ms']+origin<=b for a,b in shifts) for j in jobs)
    return dict(id=f'commission_cabinet_d{cabinet_day}_h{window_hours}',family='commissioning',
                project_tasks=[dict(id='mechanical',start_ms=0,end_ms=mechanical_ready),
                               dict(id='cabinet',start_ms=0,end_ms=cabinet_ready),
                               dict(id='wiring',start_ms=cabinet_ready,end_ms=wiring_ready),
                               dict(id='trial',start_ms=trial_start,end_ms=ready)],
                project_origin='D0 00:00:00',production_origin_project_ms=origin,
                ready_project_ms=ready,ready_relative_ms=ready-origin,
                observation_end_ms=window_hours*H,expected_packages=len(observed),
                expected_order_completion_ms=completion if len(observed)==2 else None,
                forecast_if_extended_ms=completion,jobs=jobs,
                release_decision='self-built planned simulation acceptance',actual_result=None)

def utility(name,power,air,outage=False):
    if outage:intervals=[(0,30000,'M1','active'),(30000,40000,'M2','active'),
                         (40000,70000,'M2','waiting_supply'),(70000,90000,'M2','active')]
    elif power>=12 and air>=4:intervals=[(0,30000,'M1','active'),(0,30000,'M2','active')]
    else:intervals=[(0,30000,'M1','active'),(30000,60000,'M2','active')]
    events=sorted({a for a,b,m,s in intervals}|{b for a,b,m,s in intervals})
    profile=[]
    for a,b in zip(events,events[1:]):
        active=[m for x,y,m,s in intervals if s=='active' and x<=a<y]
        cap_power=0 if outage and 40000<=a<70000 else power
        demand_power=6*len(active);demand_air=2*len(active)
        assert demand_power<=cap_power and demand_air<=air
        profile.append(dict(start_ms=a,end_ms=b,power_kw=demand_power,air_lps=demand_air,power_cap_kw=cap_power,air_cap_lps=air))
    completion=max(b for a,b,m,s in intervals if s=='active')
    active_ms=sum(b-a for a,b,m,s in intervals if s=='active')
    assert active_ms==60000
    return dict(id=name,family='service_capacity',nominal_supply=dict(power_kw=power,air_lps=air),
                intervals=[dict(start_ms=a,end_ms=b,machine=m,state=s) for a,b,m,s in intervals],
                service_profile=profile,expected_completion_ms=completion,
                expected_energy_kwh=6*active_ms/3600000,expected_air_liter=2*active_ms/1000,
                expected_packages=2,standby_draw=0,actual_result=None)

def artifact_case():
    product_r1=[100,60,30];product_r2=[108,60,30];old_inner=[104,64,34];new_inner=[112,64,34]
    clearance=2
    fits=lambda product,inner:all(x+2*clearance<=y for x,y in zip(product,inner))
    expected=dict(r1_old_envelope=fits(product_r1,old_inner),r2_old_envelope=fits(product_r2,old_inner),r2_new_envelope=fits(product_r2,new_inner))
    assert expected==dict(r1_old_envelope=True,r2_old_envelope=False,r2_new_envelope=True)
    findings=[dict(id='old_order_r1',accepted=True,reason='same subject and declared nominal-envelope scope'),
              dict(id='new_order_old_binding',accepted=False,reason='product_revision_mismatch'),
              dict(id='new_order_rechecked_envelope',accepted=True,reason='r2 envelope scope only; not folding/strength approval'),
              dict(id='calculated_nc_requested_but_absent',accepted=False,reason='calculation_result_absent'),
              dict(id='unsupported_nc_dialect',accepted=False,reason='outside declared NC subset'),
              dict(id='air_reference_conditions_mismatch',accepted=False,reason='volumetric rates cannot be directly summed')]
    return dict(id='artifact_scope_and_revision',family='professional_contract',dimension_unit='mm',
                product_r1=product_r1,product_r2=product_r2,old_inner=old_inner,new_inner=new_inner,
                per_side_clearance_mm=clearance,manual_expected_envelope_checks=expected,
                manual_expected_binding_findings=findings,producer_result=None,actual_result=None)

def verify_resources(case):
    groups={}
    for j in case.get('jobs',[]):
        for r in j['resources']:groups.setdefault(r,[]).append(j)
    for name,rows in groups.items():
        rows.sort(key=lambda x:x['start_ms'])
        assert all(a['end_ms']<=b['start_ms'] for a,b in zip(rows,rows[1:])),name

if __name__=='__main__':
    cases=[commissioning(2,24),commissioning(5,24),commissioning(5,72),
           utility('utility_base',10,3),utility('utility_power_only',14,3),
           utility('utility_air_only',10,5),utility('utility_both',14,5),
           utility('utility_outage_pause_resume',10,3,True),artifact_case()]
    for c in cases:verify_resources(c);assert c['actual_result'] is None
    assert cases[0]['expected_order_completion_ms']==70*M
    assert cases[1]['expected_packages']==0 and cases[1]['expected_order_completion_ms'] is None
    assert cases[2]['expected_order_completion_ms']==52*H+70*M
    assert [cases[i]['expected_completion_ms'] for i in (3,4,5,6,7)]==[60000,60000,60000,30000,90000]
    assert all(c['expected_energy_kwh']==0.1 and c['expected_air_liter']==120 for c in cases[3:8])
    (B/'expected-readiness.design.json').write_text(json.dumps(dict(
        document_kind='manual_readiness_and_professional_expectations',source='self-built finite arithmetic',
        cases=cases,actual_result=None),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('9 design variants consistent: project/production time, readiness, joint utility capacity, interruption and artifact scope. No runtime results.')
