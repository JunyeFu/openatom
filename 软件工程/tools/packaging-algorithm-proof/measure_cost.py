"""AQ-04 actual reference-method cost/scaling; not a Rust product benchmark."""
from pathlib import Path
from fractions import Fraction as F
import json, platform, time
import prove as g

HERE=Path(__file__).resolve().parent

def shifted(cell,index):
    def pose(t):
        r,v=cell.pose(t)
        return r,[v[0]+1000*index,v[1],v[2]]
    return g.Cell(f'{cell.ident}-{index}',cell.bounds,pose,frozenset(f'{a}-{index}' for a in cell.allowance_ids))

def main():
    fixtures={c['id']:c for c in json.loads((HERE/'finite-expectations.json').read_text(encoding='utf-8'))['cases']}
    rows=[]
    # Outcomes are existing independently frozen expectations; replication translates
    # complete pairs apart, preserving local contact/kinematic relations.
    for ident in ['LC01','LC05','KIN03','KIN04','KIN07']:
        c=fixtures[ident];p=c['parameters']
        make={'contact':lambda:g.contact_model(p),'rotation':lambda:g.rotation_model(p),'chain':g.chain_model,'extraction':lambda:g.extraction_model(p)}[c['kind']]
        for copies in [1,2,4]:
            a,b=make();a=[shifted(x,i) for i in range(copies) for x in a];b=[shifted(x,i) for i in range(copies) for x in b]
            budget=g.Budget(2048,512);start=time.perf_counter();outcome,evidence=g.certify_all(a,b,budget);elapsed=(time.perf_counter()-start)*1000
            assert outcome==c['expected'],(ident,copies,outcome,evidence)
            rows.append({'id':ident,'copies':copies,'cell_counts':[len(a),len(b)],'expected':c['expected'],'actual':outcome,'queries':budget.queries,'subdivisions':budget.subdivisions,'elapsed_ms':round(elapsed,3)})
            print(json.dumps(rows[-1]),flush=True)
    for limit in [1,8,64]:
        c=fixtures['KIN02'];a,b=g.rotation_model(c['parameters']);budget=g.Budget(limit,512);start=time.perf_counter();outcome,evidence=g.certify_all(a,b,budget)
        assert outcome in ['Fail','Undetermined']
        rows.append({'id':'KIN02-budget','query_limit':limit,'actual':outcome,'queries':budget.queries,'subdivisions':budget.subdivisions,'elapsed_ms':round((time.perf_counter()-start)*1000,3)})
    result={'status':'passed_actual_reference_cost_scaling_and_budget_boundaries','python':platform.python_version(),'platform':platform.platform(),'arithmetic':'Fraction/Taylor reference, rectangular cells; not full three-mechanism Rust solver','rows':rows,'max_queries':max(r['queries'] for r in rows),'max_elapsed_ms':max(r['elapsed_ms'] for r in rows),'reference_profile':{'candidate_limit':1,'interval_query_limit':2048,'subdivision_limit':512},'product_30_second_guarantee':False,'actual_product_runtime':None}
    (HERE/'output/aq04-cost-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:v for k,v in result.items() if k!='rows'},ensure_ascii=False))

if __name__=='__main__':main()
