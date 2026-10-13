"""从当前选用记录同步提示词，不调用生成服务或覆盖图片。"""
from pathlib import Path
import json
base=Path(__file__).parent
rows=json.loads((base/'图片记录.json').read_text(encoding='utf-8-sig'))
plan={'interaction':'两张常态画布；详细/简化/区域；单击详情；保留人工操作与物流','assets':rows,'goal_generation_requirement':{'ref':'../软件工程/docs/03_架构与契约/目标驱动厂房生成.md','visible_stages':['template_and_parameter_sources','nine_category_fill','layout_connections_schedule','draft_check','actual_run','repair_comparison','report_export'],'asset_status':'existing_22_previews_do_not_yet_show_complete_SF17'}}
(base/'视图生成计划.json').write_text(json.dumps(plan,ensure_ascii=False,indent=2),encoding='utf-8')
(base/'生图提示词.md').write_text('# 当前选用图提示词\n\n当前图保留原生成提示词；SF-17新面板需求见当前交互规格与视图生成计划，既有22张图不冒充完整目标生成界面。\n\n'+'\n\n'.join('## '+r['id']+' '+r['title']+'\n'+r.get('prompt','原始提示词待补充') for r in rows),encoding='utf-8')
print(f'已同步 {len(rows)} 项记录')

