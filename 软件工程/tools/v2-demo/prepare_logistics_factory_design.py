"""Extend the original factory design with identity-preserving case logistics."""
import xml.etree.ElementTree as ET
from prepare_full_factory import DESIGN, ART, Drawing, write


def specification():
    return {
        'kind': 'finished_case_logistics', 'version': '1.0.0', 'package': 'V2-05-G01',
        'source': 'preparation-host07 actual project, scenario and frozen modules',
        'hall_geometry_changed': False, 'AI_views_reused': True,
        'devices': ['belt-conveyor', 'right-angle-transfer', 'roller-conveyor', 'vacuum-palletizer'],
        'case': {'type': 'PF-packed-case', 'count': 3, 'length_m': .32, 'width_m': .24,
                 'height_m': .24, 'contents_per_case': 6, 'state': 'two_band_pack'},
        'flow': 'Final inspection -> belt -> right-angle transfer -> rollers -> single-case vacuum placement -> shipping',
        'stock_policy': 'Remove four isolated demonstration inputs; all four stations consume only actual upstream released cases.',
        'identity_policy': 'Preserve the same three case IDs, eighteen contained item IDs, seals, labels, straps and inspection quality.',
        'geometry_policy': 'Keep accepted device geometry and motion; reuse the actual 320x240x240mm packed-case geometry on each declared cargo trajectory. Do not scale the complete machine or replace cases with 240x180x120mm example boxes.',
        'vacuum_height_fixture': {'height_difference_m': .12, 'node_y_offsets_m': {'J3_VerticalTool': .12, 'PaperSeat': -.12, 'TelescopingStage_1': .04, 'TelescopingStage_2': .08},
                                 'rule': 'Raise suction tooling for the actual case height; compensate its child cargo seat so the case bottom follows the original world trajectory. Apply to both accepted LODs and all declared phases.'},
        'transfer_policy': 'Existing three carriers connect separated workstations via declared aisles; one case per trip, including return.',
        'expected': {'manufacturing_jobs': 161, 'transport_trips': 117,
                     'accepted_orders': {'PF-ORDER-CASE': 3, 'PF-ORDER-SLEEVE': 3, 'PF-ORDER-TUBE': 6,
                                         'PF-ORDER-LABEL': 18, 'PF-ORDER-BAG': 3,
                                         'PF-ORDER-SHEETS': 60, 'PF-ORDER-BOARDS': 6, 'PF-ORDER-CURED': 3},
                     'connected_devices': 30, 'independent_cells': 62},
        'expected_derivation': '153 - 4 isolated one-cycle jobs + 4*3 actual case jobs = 161; replace 3 direct shipping trips with 5*3 trips = 117.',
        'pallet_followup': {'state': 'design_required_not_implemented', 'container_m': [1.2, .8, .12],
                            'canonical_declared_case_contents': 6, 'film_m_per_load': 9,
                            'required': 'Explicitly assemble a pallet container with actual case contents before wrapping; the current three single-case placements do not constitute a six-case pallet.'},
        'acceptance': ['Three identities preserved across all four devices', 'Each case retains six original contained items',
                       'All eight existing orders remain exact', 'Actual 15 logistics trips and complete returns',
                       'Exact save/reopen', 'Native trajectories and box dimensions verified before template promotion'],
    }


def attach_diagram(path, spec):
    dr = Drawing(); dr.file = ET.parse(path).getroot(); dr.counter = 12000
    for page in list(dr.file):
        if page.get('id') == 'case-logistics-detail': dr.file.remove(page)
    dr.page('09 成品输送、单箱码放与整托边界', 2020, 1050)
    dr.file[-1].set('id', 'case-logistics-detail')
    dr.box('成品物流｜同一纸箱完成输送、转向与单箱码放', 60, 25, 1900, 60, font=28, style='strokeColor=none;fontStyle=1;')
    dr.box('沿用104×90×9m厂房、12区和92台设备位置；复用同布局AI三视图。三只320×240×240mm成品箱，每箱六件内装物。', 60, 110, 1900, 65, font=20, style='strokeColor=none;align=left;')
    nodes = [('终检放行', '3只已封箱、贴标、打带纸箱'), ('皮带输送 · Z11', '原箱号、尺寸和内装物不变'),
             ('直角移载 · Z11', '横移转向路径，箱体朝向保持'), ('辊筒输送 · Z11', '原箱经过辊筒段'),
             ('真空码放 · Z11', '逐箱放入固定托位'), ('成品入库', '仍按3箱验收，不冒充整托')]
    ids = [dr.box(title + '\n' + detail, 60+i*320, 260, 275, 160, '#dcebd9' if i in (0, 5) else '#edf3f8', 19, key='LOG-'+str(i)) for i, (title, detail) in enumerate(nodes)]
    for left, right in zip(ids, ids[1:]): dr.edge(left, right, '实物转运')
    dr.box('四台独立演示设备各1次作业，改为各处理3只真实上游箱：153 − 4 + 12 = 161次制造。原3趟直送改为5段×3趟：105 − 3 + 15 = 117趟运输。八订单仍接受102件产品。', 60, 480, 1900, 110, '#f3e7cf', 21, style='align=left;')
    dr.box('当前实施出口\n保持3箱、18件内装物身份和质量；不新增假来料，车辆按通道往返。\n箱高增加120mm：真空吸头上移120mm，子支承点下移120mm，箱底轨迹不变；机架不缩放。', 60, 660, 920, 180, '#dcebd9', 21, style='align=left;')
    dr.box('1.5.0阶段边界 · 整托接续见第10页\n1.2×0.8m托盘需明确装入箱件，原缠绕素材声明6箱、9m膜。\n三次单箱码放不等于整托；第10页明确三箱装托及实际缠绕要求。', 1040, 660, 920, 180, '#f3e7cf', 21, style='align=left;')
    dr.box('验收：Host数量、身份、内装物、完整运输、保存重开 → Native阶段与箱体尺寸 → 新精确版本模板。30台联线、62个独立单元；未达到整厂完成。', 60, 900, 1900, 80, font=20, style='strokeColor=none;align=left;')
    for cell in dr.root.findall('mxCell'):
        if cell.get('value'): cell.set('value', cell.get('value').replace('\n', '<br>'))
        if cell.get('style'): cell.set('style', cell.get('style').replace('html=0', 'html=1'))
    dr.save(path)


if __name__ == '__main__':
    spec = specification(); write(DESIGN / 'logistics-line-spec.json', spec)
    attach_diagram(ART / '纸包装整厂区域与设备分配.drawio', spec)
    print('Original design extended with four actual case-logistics stations and explicit pallet boundary.')
