# P0-01 Q52普通Native异常打开验证

本窗口按原P0-01步骤2—4、Q52与NFR-06验证未知格式、缺必需模块和缺固定素材。测试只在工程副本中进行，当前活动工程固定为P1/r8、D1名称`Native 搬移重开验证 D1-C`；不改原21包、原步骤或验收分组。

## 已执行结果

candidate07普通Native（PID218232）经系统目录选择器打开已保存的C/r8工程，再逐个选择四份预先固定的输入。完整素材副本解析出25对象、11个精确模块版本，只有原预期中的专业人工认可缺项；取消采纳后当前工程不变。缺必需`module-devices.json`与缺`assets/fixed.png`分别显示`MissingReference`和实际相对路径，C/r8工程与对象树保持。

未知`schema_version="99.0"`也被拒绝且保留当前工程，但candidate07仅显示泛化英文，没有指出文件、实际版本和支持版本。该提示不满足Q52的具体问题说明，已定位到`packages/commands/src/project_store.rs::decode`。

修复只补充同一`UnsupportedSchema`消息中的文件路径、实际版本和本版支持的`1.0`；不新增格式迁移或兼容承诺，不改公开类型、错误码、Schema及保存格式。新增针对根清单与模块文件的回归：修复前实际失败，修复后本模块31项测试通过。

后继candidate08前端、Native及Bridge实际构建均退出0。普通Native新进程PID240092重新打开C/r8，再通过系统选择器打开同一未知版本副本，实际显示`UnsupportedSchema: project.json: 不支持 schema_version="99.0"；当前支持 schema_version="1.0"。`，当前工程仍C/r8、4设备、2线组。修复完成。其前端与07构建内容逐字节一致；缺模块/缺素材保留07实际结果，改动的未知版本路径以08实际复验，不扩大08覆盖范围。

## 证据与边界

- [预先固定的输入与预期](../../.work/single-window-20261017/p001-q52/expectations.json)
- [实际验证JSON](../../.work/single-window-20261017/p001-q52/validation.json)
- [普通Native界面观察](../../.work/single-window-20261017/p001-q52/ui-observations.jsonl)
- [输入逐文件及字节保留核对](../../.work/single-window-20261017/p001-q52/input-preservation.json)：四份输入分别23、23、22、22文件，均无增删或改写。
- [修复前回归](../../.work/single-window-20261017/p001-q52/schema-message-red.log) / [修复后31项结果](../../.work/single-window-20261017/p001-q52/project-store-green.log)
- [candidate08实际构建与限定启动](../../.work/whole-packages/p001-20261017/portable-candidate-08/portable-build.json) / [活动工程完整磁盘核对](../../.work/single-window-20261017/p001-q52/active-disk-after-08.json)：工程全文和20份引用文档仍符合独立C/r8预期。

本次对应原NFR-06的`unknown_version`、`missing_module`、`missing_asset`三个既有组。完整NFR-06还包括正常重开、F01—F04保存失败及实际Windows1176/1177等原要求；Native活动任务/并发、其他正式NFR和真人/第二机验收仍分别保留。P0-01整包继续`in_progress`、`whole_task_done=false`，完整结果和人验不代填。
