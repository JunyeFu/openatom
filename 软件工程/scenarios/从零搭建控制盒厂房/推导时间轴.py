"""本例的有限手算复算：没有通用调度器，不执行产品运行引擎。"""
from pathlib import Path
import json

BASE = Path(__file__).parent

def derive(name, pcb_second_arrival, starts):
    jobs, transfers, units = [], [], []
    def job(kind, unit, start, duration, resources):
        row = dict(kind=kind, item=f'{unit:03}', start_ms=start*1000,
                   end_ms=(start+duration)*1000, resources=resources)
        jobs.append(row)
        return start+duration
    for i in range(1, 7):
        t=(i-1)*60
        job('cnc_machine',i,t,60,['CNC01'])
        job('cnc_load',i,t,10,['W-C'])
        job('cnc_auto',i,t+10,45,[])
        job('cnc_unload',i,t+55,5,['W-C'])
        arrival=i*60+10
        accepted=max(arrival, pcb_second_arrival) if name=='delayed_pcb' and i==6 else arrival
        job('cart_travel',i,i*60,10,['T1'])
        if accepted>arrival: job('cart_receive_wait',i,arrival,accepted-arrival,['T1'])
        job('cart_return',i,accepted,5,['T1'])
        transfers.append(dict(item=f'shell{i:03}',depart_ms=i*60000,
                              arrival_ms=arrival*1000,accepted_ms=accepted*1000,
                              resource_available_ms=(accepted+5)*1000))
    for i, start in enumerate(starts, 1):
        end=job('assembly',i,start,40,['W-A','Assembly01'])
        qc_end=job('inspection',i,end,10,['W-B','Inspection01'])
        if i==3:
            rework_end=job('rework',i,qc_end,20,['W-A','Rework01'])
            qc_end=job('reinspection',i,rework_end,10,['W-B','Inspection01'])
        pack_end=job('packing',i,qc_end,20,['W-B','Packing01'])
        units.append(dict(id=f'package{i:03}',shell=f'shell{i:03}',pcb=f'pcb{i:03}',
                          assembly=f'assembly{i:03}',assembly_start_ms=start*1000,
                          first_inspection='FAIL' if i==3 else 'PASS',
                          reinspection='PASS' if i==3 else None,
                          completed_ms=pack_end*1000))
    # 同时间先完成/到料，再提交与解除阻塞，再开工。净占用统一在事件末观察。
    times=sorted({t['accepted_ms'] for t in transfers}|{u['assembly_start_ms'] for u in units})
    buffer=[]
    for t in times:
        held=[x['item'] for x,u in zip(transfers,units)
              if x['accepted_ms']<=t<u['assembly_start_ms']]
        buffer.append(dict(at_ms=t,items=held,quantity=len(held)))
    return dict(document_kind='manual_expected_schedule_not_runtime_result',
                scenario=name,time_unit='millisecond',observation_end_ms=600000,
                pcb_second_arrival_ms=pcb_second_arrival*1000,jobs=jobs,
                transfers=transfers,buffer_events=buffer,units=units,
                expected_packages=6,expected_completion_ms=max(u['completed_ms'] for u in units),
                actual_result=None)

def verify(data):
    # 验证本次明确需要的数量、前置与资源不变量；不是产品验收。
    occupied={}
    for j in data['jobs']:
        assert 0<=j['start_ms']<j['end_ms']<=600000
        for res in j['resources']:occupied.setdefault(res,[]).append(j)
    for res, rows in occupied.items():
        rows.sort(key=lambda x:(x['start_ms'],x['end_ms']))
        assert all(a['end_ms']<=b['start_ms'] for a,b in zip(rows,rows[1:])),res
    for i,(u,t) in enumerate(zip(data['units'],data['transfers']),1):
        assert u['assembly_start_ms']>=t['accepted_ms']
        assert u['assembly_start_ms']>=(0 if i<=3 else data['pcb_second_arrival_ms'])
    assert max(e['quantity'] for e in data['buffer_events'])<=2
    assert len({u['shell'] for u in data['units']})==6
    assert len({u['pcb'] for u in data['units']})==6
    assert len({u['assembly'] for u in data['units']})==6
    assert len(data['units'])*4==24 and len(data['units'])==6  # fasteners / cartons
    assert data['actual_result'] is None

if __name__=='__main__':
    cases=[derive('baseline',240,[70,130,190,260,310,370]),
           derive('delayed_pcb',420,[70,130,190,420,460,500])]
    for case in cases:verify(case)
    assert [c['expected_completion_ms'] for c in cases]==[440000,570000]
    assert cases[1]['transfers'][5]['resource_available_ms']==425000
    (BASE/'expected-timeline.design.json').write_text(json.dumps(
        dict(document_kind='manual_expectations',calculation='finite scenario arithmetic',
             cases=cases,actual_result=None),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('Manual expectations consistent: baseline 440 s; delayed PCB 570 s; actual_result=null')
