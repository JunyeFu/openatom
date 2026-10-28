"""以精确有理/Taylor包围独立核对实际Rust binary64区间输出。"""
from fractions import Fraction as F
from pathlib import Path
import json
import subprocess
import sys
sys.dont_write_bytecode=True
from prove import I, trig

HERE=Path(__file__).resolve().parent;OUT=HERE/'output'
def main():
    run=subprocess.run([str(OUT/'float_interval.exe')],check=True,capture_output=True,text=True)
    rows=json.loads(run.stdout);results=[]
    exact={'add_decimal':F(0.1)+F(0.2),'cancellation':F(1e16)+1-F(1e16),
        'multiplication_scale':F(1e150)*F(1e-150),'one_third':F(1,3)}
    for row in rows:
        lo,hi=map(F,row['interval'])
        if row['id'] in exact:
            a=b=exact[row['id']]
        else:
            ref=trig(I(*row['input']),cosine=row['id'].startswith('cos'))
            a,b=ref.lo,ref.hi
        assert lo<=a<=b<=hi,(row['id'],float(lo-a),float(hi-b))
        results.append({'id':row['id'],'actual_Rust_f64_encloses_exact_reference':True,'actual_interval':row['interval']})
    cancel=next(row for row in rows if row['id']=='cancellation')['interval']
    assert cancel[1]-cancel[0]>1e-6
    result={'evidence_level':'actual_std_only_Rust_f64_primitive_enclosures_compared_with_exact_rational_reference',
        'compiler':'rustc 1.97.0','cases':results,'case_count':len(results),'cancellation_must_not_be_precision_Pass':True,
        'full_geometry_kernel_proved':False,'dependencies_installed':False,'shared_types_or_locks_written':False}
    (OUT/'float-interval-raw.json').write_text(json.dumps(rows,indent=2)+'\n',encoding='utf-8')
    (OUT/'float-interval-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'actual_f64_primitive_cases':len(results),'exact_enclosures':'passed','full_geometry_kernel_proved':False},ensure_ascii=False))
if __name__=='__main__':main()
