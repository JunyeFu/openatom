"""AQ-03/05 dependency negatives and actual finite change experiments."""
from pathlib import Path
from html.parser import HTMLParser
import copy,json,re,subprocess,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
OUT=HERE/'output'
sys.path.insert(0,str(ROOT/'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator

def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))

def validate_graph(rows):
    c={x['id']:x for x in rows}
    features={'factory_ui','authoring_ui','packaging_ui','delivery_agent_ui'}
    for name in features:
        assert not (set(c[name]['code_dependencies']) & (features-{name})),name
    assert c['shared_ui']['code_dependencies']==['domain']
    assert c['shared_ui']['planned_code_paths']==['apps/planner/src/components','apps/planner/src/styles']
    visiting=set();done=set()
    def walk(x):
        assert x not in visiting
        if x in done:return
        visiting.add(x)
        for d in c[x]['code_dependencies']:walk(d)
        visiting.remove(x);done.add(x)
    for x in c:walk(x)

class Visible(HTMLParser):
    def __init__(self):super().__init__();self.skip=0;self.values=[];self.rows=0
    def handle_starttag(self,tag,attrs):
        if tag in ['style','script']:self.skip+=1
        if tag=='tr':self.rows+=1
    def handle_endtag(self,tag):
        if tag in ['style','script']:self.skip-=1
    def handle_data(self,value):
        if not self.skip and value.strip():self.values.append(value.strip())

def main():
    OUT.mkdir(exist_ok=True)
    arch=read(ROOT/'docs/03_架构与契约/全层组件与接点.json');validate_graph(arch['components'])
    negatives=[]
    for source,target in [('packaging_ui','factory_ui'),('shared_ui','packaging_ui'),('delivery_agent_ui','authoring_ui')]:
        bad=copy.deepcopy(arch['components']);next(x for x in bad if x['id']==source)['code_dependencies'].append(target)
        try:validate_graph(bad)
        except AssertionError:negatives.append(f'{source}->{target}')
        else:raise AssertionError('forbidden dependency accepted')
    # C01: expose an existing supported duration leaf as a public parameter;
    # no new engine effect or public DTO is fabricated.
    before=read(ROOT/'tools/type-contract-proof/fixtures/module-operations.json');after=copy.deepcopy(before)
    after['data']['definition']['parameters'].append({'id':'duration_ms','label':'工序耗时','value_type':'number','unit':'millisecond','binding':'/payload/data/duration/value','integer':True,'minimum':0,'maximum':None,'options':[],'basis':'template_declared_example'})
    after['data']['definition']['payload']['data']['duration']['value']=2000
    after['data']['definition']['version']='0.1.1'
    schema=read(ROOT/'tools/type-contract-proof/output/schemas/module.schema.json');Draft202012Validator(schema).validate(after)
    pointer=after['data']['definition']['parameters'][0]['binding'];value=after['data']['definition']
    for token in pointer.split('/')[1:]:value=value[token]
    assert value==2000
    for name,data in [('parameter-before',before),('parameter-after',after)]:
        (OUT/(name+'.json')).write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    # C02: modify only the CSS in a real existing same-source HTML export.
    path=ROOT/'docs/04_开发与验收/特征细化/DRF05_四格式试做/output/evaluation.type_fixture.html'
    baseline=path.read_text(encoding='utf-8');styled=baseline.replace('</style>','table { font-size: 11px; } th { padding: 8px; }</style>',1)
    assert styled!=baseline
    a,b=Visible(),Visible();a.feed(baseline);b.feed(styled)
    assert a.values==b.values and a.rows==b.rows and a.rows>=69
    (OUT/'report-layout-before.html').write_text(baseline,encoding='utf-8');(OUT/'report-layout-after.html').write_text(styled,encoding='utf-8')
    # C03: compile three real isolated TS consumers before/after an optional
    # shared field refinement. No product frontend is created or built.
    compiler=ROOT/'tools/type-contract-proof/node_modules/typescript/bin/tsc'
    node=Path(r'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node.exe')
    base='export type NumericFieldProps = { value: number; unit: string; readOnly: boolean; onCommit: (v: number) => void; };\nexport const numericField = (p: NumericFieldProps): string => `${p.value} ${p.unit}`;\n'
    changed=base.replace('readOnly: boolean;','readOnly: boolean; precision?: number;')
    compiled=[]
    for name,source in [('before',base),('after',changed)]:
        folder=OUT/('shared-'+name);folder.mkdir(exist_ok=True)
        (folder/'shared_ui.ts').write_text(source,encoding='utf-8')
        for feature in ['authoring','packaging','reporting']:
            (folder/(feature+'.ts')).write_text("import { numericField } from './shared_ui';\nexport const display = numericField({value: 1, unit: 'mm', readOnly: false, onCommit: (_v: number) => {}});\n",encoding='utf-8')
        config={'compilerOptions':{'strict':True,'noEmit':True,'target':'ES2022','module':'ESNext','moduleResolution':'Bundler','types':[]},'include':['*.ts']}
        (folder/'tsconfig.json').write_text(json.dumps(config),encoding='utf-8')
        result=subprocess.run([str(node),str(compiler),'-p',str(folder/'tsconfig.json')],capture_output=True,text=True)
        assert result.returncode==0,result.stdout+result.stderr
        compiled.append(name)
    result={'status':'passed_actual_finite_change_experiments_and_dependency_negatives','components':len(arch['components']),'forbidden_edges_rejected':negatives,'change_scenarios':[{'id':'CH01','stimulus':'公开已有工艺耗时参数并改值','actual':'canonical ModulePackage Schema and pointer resolved; existing DTO reused','affected_design_owners':['module_kit','authoring_ui'],'unchanged_contracts':['domain','simulation','packaging','reporting']},{'id':'CH02','stimulus':'调整HTML报告表格排版','actual':f'{a.rows} table rows and all visible text preserved after CSS-only change','affected_design_owners':['reporting'],'unchanged_contracts':['simulation','domain','module_kit','工程保存格式']},{'id':'CH03','stimulus':'公共数值字段增加可选precision','actual':'three isolated consumers compile before and after','affected_design_owners':['shared_ui'],'unchanged_contracts':['factory_ui','packaging','simulation','reporting指标']}],'TS_compiled_variants':compiled,'actual_product_frontend_or_engine':False,'actual_product_runtime':None}
    (OUT/'aq03-aq05-change-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result,ensure_ascii=False))

if __name__=='__main__':main()
