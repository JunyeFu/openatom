"""Read already saved relocated archives in separate hidden processes; no new Run."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];OUT=ROOT/'.work/cu-delivery'
for mode,attempt in [('workflow-success','r5'),('workflow-repair','r1')]:
    original=OUT/'cases'/mode;stage=OUT/'cases'/(mode+'-history');stage.mkdir(exist_ok=False)
    case=json.loads((original/'case.json').read_text());data=json.loads((original/'input.json').read_text())
    data['workflow']=json.loads((original/('actual-'+attempt)/'workflow-result.json').read_text())
    case.update(mode='workflow-history',seed=str(OUT/'relocated'/(mode+'-'+attempt)))
    (stage/'case.json').write_text(json.dumps(case,ensure_ascii=False,indent=2),encoding='utf-8')
    (stage/'input.json').write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')
print('prepared actual saved-Run history consumers at relocated paths')
