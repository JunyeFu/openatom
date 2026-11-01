"""只刷新隔离原型的只读设计资料，不运行产品或改公共合同。"""
from pathlib import Path
import json
D=Path(__file__).resolve().parent
S=D.parents[2]
data=json.loads((D/'prototype.data.js').read_text(encoding='utf-8').removeprefix('window.S04_DATA = ').rstrip(';\n'))
A=S/'docs/04_开发与验收/特征细化'
data['h04']=json.loads((A/'DRF04_H04-04界面绑定.json').read_text(encoding='utf-8-sig'))
data['business_fixtures']=json.loads((A/'DRF06_时间轴独立预期.json').read_text(encoding='utf-8-sig'))['fixtures']
data['professional_sample_semantics']=json.loads((A/'DRF03_C03_过程样本语义.json').read_text(encoding='utf-8-sig'))
data['generated_type_basis']=json.loads((S/'docs/04_开发与验收/类型格式检查结果.json').read_text(encoding='utf-8-sig'))
data['template_adoption_basis']=json.loads((A/'DRF03_C04_r4_公共增量.json').read_text(encoding='utf-8-sig'))
data['module_categories']=['items','devices','resources','tasks','operations','transport','templates','professional','media']
for ref in ['docs/04_开发与验收/特征细化/DRF04_H04-04界面绑定.json','docs/04_开发与验收/特征细化/DRF03_C03_过程样本语义.json','docs/04_开发与验收/特征细化/DRF06_时间轴独立预期.json','packages/reporting/public-delta.design.json','docs/04_开发与验收/特征细化/DRF03_C04_r4_公共增量.json']:
    if ref not in data['source_refs']: data['source_refs'].append(ref)
data['source_refresh']={'date':'2026-10-14','h04_fields':len(data['h04']['bindings']),'h04_states':len(data['h04']['state_expectations']),'TS_files':data['generated_type_basis']['generated_TS_files'],'actual_product_runtime':None}
(D/'prototype.data.js').write_text('window.S04_DATA = '+json.dumps(data,ensure_ascii=False,separators=(',',':'))+';\n',encoding='utf-8')
print(json.dumps(data['source_refresh'],ensure_ascii=False))
