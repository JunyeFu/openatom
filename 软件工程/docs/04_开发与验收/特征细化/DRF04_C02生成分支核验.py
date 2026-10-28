"""Extract only the new pure branch; never import/run the overwriting generator."""
import ast
import copy
import json
import os
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).parent
sys.path.insert(0,str(ROOT/'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator


def main():
    read=lambda p:json.loads(p.read_text(encoding='utf-8'))
    source=ROOT/'modules/templates/生成三类模板设计.py'
    tree=ast.parse(source.read_text(encoding='utf-8'))
    selected=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='select_packaging_branch')
    module=ast.fix_missing_locations(ast.Module(body=[selected],type_ignores=[]))
    reads=[]
    def reader(path):
        reads.append(path.relative_to(ROOT).as_posix())
        return read(path)
    namespace={'copy':copy,'PROJECT':ROOT,'read':reader}
    exec(compile(module,str(source),'exec'),namespace)
    branch=namespace['select_packaging_branch']
    prototype={'id':'Packing01','kind':'fixed','module':'community.parameterized-pack','process_mode':'declared_process',
               'parameters':{'product_item':'assembly','cycle_seconds':20},
               'port_bindings':{'product':'store-released','carton':'store-carton','packed':'store-package'},
               'job_resources':['Packing01','W-B'],'resource_bindings':{'packer':'W-B'}}
    snapshots={folder:read(ROOT/'modules/templates'/folder/'project.design.json') for folder in ('machining-delivery','purchased-assembly','parallel-electromechanical')}
    result,info=branch(prototype,'declared_process')
    assert result==prototype and info is None and reads==[]
    results=[{'id':'C02G01','status':'passed','expected':'legacy declaration unchanged, source0.2 not loaded'}]
    result,info=branch(prototype,'calculated_nominal')
    assert result['configuration']=={} and info['missing_selection']==['packaging_job_id','packaging_job_revision']
    assert result['module_ref']=={'id':'community.parameterized-pack','version':'0.2.0'}
    results.append({'id':'C02G02','status':'passed','expected':'explicit0.2 branch loaded; no selected Job -> two missing values, no defaults'})
    result,info=branch(prototype,'calculated_nominal',{'packaging_job_id':'EXPLICIT_TEST_JOB','packaging_job_revision':7})
    assert result['configuration']=={'packaging_job_id':{'kind':'string','value':'EXPLICIT_TEST_JOB'},'packaging_job_revision':{'kind':'integer','value':7}}
    assert result['parameters']==prototype['parameters'] and result['port_bindings']==prototype['port_bindings']
    actual=read(ROOT/'packages/packaging/consumer-device.design.json')['data']['definition']
    assert info['parameter_declarations']==[x for x in actual['parameters'] if x['id'] in ('packaging_job_id','packaging_job_revision')]
    schema=read(ROOT/'tools/type-contract-proof/output/schemas/all-types.schema.json')
    value_schema={'$schema':schema['$schema'],'$defs':schema['$defs'],'$ref':'#/$defs/TypedValue'}
    for value in result['configuration'].values():Draft202012Validator(value_schema).validate(value)
    results.append({'id':'C02G03','status':'passed','expected':'explicit pair becomes two canonical TypedValues; legacy20s/product/ports unchanged; no result/registration'})
    partial,info=branch(prototype,'calculated_nominal',{'packaging_job_id':'EXPLICIT_TEST_JOB'})
    assert partial['configuration']=={} and info['missing_selection']==['packaging_job_revision']
    results.append({'id':'C02G04','status':'passed','expected':'partial selection does not partially write configuration or choose latest revision'})
    zero,info=branch(prototype,'calculated_nominal',{'packaging_job_id':'EXPLICIT_TEST_JOB','packaging_job_revision':0})
    assert zero['configuration']['packaging_job_revision']=={'kind':'integer','value':0}
    results.append({'id':'C02G05','status':'passed','expected':'explicit revision0 preserved, not treated as missing/default'})
    try:branch(prototype,'calculated_nominal',{'packaging_job_id':'EXPLICIT_TEST_JOB','packaging_job_revision':True})
    except ValueError:results.append({'id':'C02G06','status':'passed','expected':'Boolean is not a SafeUint revision'})
    else:raise AssertionError('Boolean revision accepted')
    try:branch(prototype,'software_in_loop')
    except ValueError:results.append({'id':'C02G07','status':'passed','expected':'unsupported branch has no silent declaration fallback'})
    else:raise AssertionError('Unsupported mode accepted')

    # Exercise the existing build body with an in-memory output path; no direct
    # save or README write reaches the source templates/devices/catalog.
    captured={}
    class OutputPath:
        def __init__(self,path):self.path=Path(path)
        def __truediv__(self,name):return OutputPath(self.path/name)
        def __fspath__(self):return str(self.path)
        @property
        def parent(self):return OutputPath(self.path.parent)
        def as_posix(self):return self.path.as_posix()
        def write_text(self,text,encoding):captured[self.as_posix()]=text
    builder_names={'device','module_ref','build'}
    builder_ast=ast.fix_missing_locations(ast.Module(body=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in builder_names],type_ignores=[]))
    namespace.update(BASE=OutputPath(ROOT/'modules/templates'),Path=Path,os=os,
                     definitions={folder:read(ROOT/'modules/devices'/folder/'manifest.json') for folder in ('control-box-assembly','declared-board-assembly','parameterized-pack','parameterized-inspect')},
                     save=lambda path,value:captured.__setitem__(path.as_posix(),copy.deepcopy(value)))
    exec(compile(builder_ast,str(source),'exec'),namespace)
    build=namespace['build']
    def generated(folder,**options):
        captured.clear();build(folder,'fixture title',{'machining-delivery':'machining','purchased-assembly':'purchased','parallel-electromechanical':'parallel'}[folder],**options)
        return captured[(ROOT/'modules/templates'/folder/'project.design.json').as_posix()]
    legacy=generated('purchased-assembly')
    pack=next(d for d in legacy['devices'] if d['id']=='Packing01')
    assert pack['process_mode']=='declared_process' and pack['parameters']['cycle_seconds']==20
    assert next(x for x in legacy['module_refs'] if x['id']=='community.parameterized-pack')['version']=='0.1.0'
    assert 'selected_packaging_branch' not in legacy and legacy['demand']['quantity']==2
    results.append({'id':'C02G08','status':'passed','expected':'default build remains0.1.0/20s/two products, no selected nominal branch'})
    nominal=generated('purchased-assembly',packing_mode='calculated_nominal',packaging_job_selection={'packaging_job_id':'EXPLICIT_TEST_JOB','packaging_job_revision':7})
    pack=next(d for d in nominal['devices'] if d['id']=='Packing01')
    ref=next(x for x in nominal['module_refs'] if x['id']=='community.parameterized-pack')
    assert pack['configuration']['packaging_job_id']['value']=='EXPLICIT_TEST_JOB' and ref['version']=='0.2.0'
    assert ref['manifest'].endswith('packages/packaging/consumer-device.design.json')
    assert pack['parameters']['cycle_seconds']==20 and nominal['initial_stock']==legacy['initial_stock'] and nominal['connections']==legacy['connections']
    results.append({'id':'C02G09','status':'passed','expected':'selected build references owned0.2, explicit TypedValue pair, same legacy supply/recipe/ports; no IO'})
    missing=generated('purchased-assembly',packing_mode='calculated_nominal')
    assert missing['selected_packaging_branch']['missing_selection']==['packaging_job_id','packaging_job_revision']
    assert next(d for d in missing['devices'] if d['id']=='Packing01')['configuration']=={}
    results.append({'id':'C02G10','status':'passed','expected':'selected nominal build without Job exposes missing pair, no demo/latest default'})
    expected_stock={'machining-delivery':{'blank':2,'carton':2},'purchased-assembly':{'shell':2,'pcb':2,'fastener':8,'carton':2},'parallel-electromechanical':{'blank':2,'bare_pcb':2,'component':16,'fastener':8,'carton':2}}
    for folder,stock in expected_stock.items():
        data=generated(folder)
        assert {x['item_type']:x['quantity'] for x in data['initial_stock']}==stock
        assert data['demand']['quantity']==2 and all(x['version']=='0.1.0' for x in data['module_refs'])
    results.append({'id':'C02G11','status':'passed','expected':'all three default template stock/BOM quantities and0.1.0 references preserved in memory'})
    assert all(read(ROOT/'modules/templates'/folder/'project.design.json')==before for folder,before in snapshots.items())
    assert prototype['process_mode']=='declared_process' and 'configuration' not in prototype
    build_node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='build')
    assert build_node.args.args[-2].arg=='packing_mode' and build_node.args.defaults[0].value=='declared_process'
    assert any(isinstance(n,ast.Call) and isinstance(n.func,ast.Name) and n.func.id=='select_packaging_branch' for n in ast.walk(build_node))
    record={'document_kind':'actual_extracted_pure_generator_branch_checks_not_product_or_full_generator_run',
            'status':'passed_11_explicit_selection_and_inmemory_build_cases','generator_source':'modules/templates/生成三类模板设计.py',
            'source_definition':'packages/packaging/consumer-device.design.json','source_version':'0.2.0',
            'cases':results,'canonical_TypedValue_schema':'passed','three_template_files_unchanged':True,
            'generator_main_executed':False,'catalog_or_devices_written':False,
            'actual_packaging_job_lookup':None,'actual_provider_registration':None,'actual_Host':None,'actual_product_runtime':None,'actual_result':None}
    (HERE/'DRF04_C02生成分支核验.json').write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:record[k] for k in ('status','canonical_TypedValue_schema','three_template_files_unchanged','generator_main_executed','actual_product_runtime')}))


if __name__=='__main__':main()
