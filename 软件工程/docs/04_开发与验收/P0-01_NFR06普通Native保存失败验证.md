# P0-01 NFR-06普通Native保存失败验证

本窗口按原P0-01步骤2—4及NFR-06四个失败出口继续验证。原两个必需组、Q51/Q52、完整NFR与整包验收要求保持；只操作`.work/single-window-20261017/p001-nfr06/`中的测试副本。

| 原分支 | 当前实际结果 |
|---|---|
| NFR06-F01：提交前写入失败 | candidate08普通Native实际Windows5、旧工程完整重开、dirty保留、恢复权限后显式重试及新进程r9读回通过。 |
| NFR06-F02：替换失败且旧根有效 | candidate08普通Native实际Windows32、旧工程完整重开、dirty/关闭取消及显式重试通过。 |
| NFR06-F03：旧根失效后恢复成功 | 尚无本轮实际普通Native证据。 |
| NFR06-F04：旧根恢复也失败 | 尚无本轮预定负例实际普通Native证据。 |

## F02实际过程与判定

预先冻结的副本为P1/r8、generation 3，D1名称`Native 搬移重开验证 D1-C`。普通Native PID240092打开该副本，仅将D1改成`Native 保存失败保留验证 D1-F02`并应用为r9。独立文件句柄允许ReadWrite、拒绝Delete；从应用Ctrl+S和系统目录选择器保存到同一目标，真实`ReplaceFileW`返回Windows错误32。

界面实际显示`IoFailure`，明确旧有效清单仍在`project.json`，失败新根保留于`.sf-new-root-240092-1.json`。新代4已写入但未成为正式版本；r9、D1新名称和“有未保存修订”保持，没有收到本次成功保存输出。错误来自实际`get_task → awaitTask(failed) → HostFailure`消费路径；本轮未捕获原始TaskState报文，证据不冒称原始协议抓取。

释放句柄后，Alt+F4仍弹出未保存提醒，选择“返回编辑”保留r9。新进程PID244208从同一原目录完整打开旧r8，D1旧名称、4设备、2线组和全部引用一致；原编辑进程仍保留未保存r9。失败后的25个既有文件逐字节未变，完整工程与20份清单引用文档符合独立预期。原有专业非人工认可问题保持，没有补造认可或忽略缺失文件。

因此，F02的`fault_handling_status`、`old_content_retention_status`、`project_health_status`及`branch_acceptance_status`均为pass；该次失败的`normal_save_succeeded=false`。此判定来自实际保留内容和产品重开，不从错误码或文件名推断旧根健康。

随后在原PID240092中显式重试同一r9、同一目录，成功提交generation 5并清除dirty；失败临时根和generation 4保留，旧24个非根文件未变。保存后正常退出，再用全新PID233324从原目录重开r9及D1新名称；完整工程和20份引用文档核对通过。这个后续成功保存单独记录，不将前次失败改写为保存成功。本轮未发现需改产品源码的问题，未重复构建候选版或运行无关回归。

## F01实际过程与判定

独立F01副本仍从冻结旧P1/r8、generation 3开始。普通Native PID233324只将D1改成预先声明的`Native 写入失败保留验证 D1-F01`，应用为未保存r9。仅对该测试副本`data`目录给当前用户加不继承的拒绝新建子目录权限；应用实际保存返回`IoFailure: data/g000004: 拒绝访问。 (os error 5)`。新代目录尚未创建，根清单未切换，没有本次成功保存输出；当前r9、D1新名称和dirty保持。25个原文件及全部文件名、字节均未变化，没有新增半写文件。

恢复权限后，新进程PID247324从同一原目标完整重开旧r8，原进程仍保留r9。完整工程和20份清单引用文档核对通过，原有专业非人工认可问题保持。F01四项独立判定均pass，该次失败的`normal_save_succeeded=false`。

测试清理辅助进程退出1：严格SDDL字符串断言发现Windows增加了`DiscretionaryAclAutoInherited`标志。随后独立核对确认所有者、组、权限ACE完整字节及保护状态均与原始权限一致，仅继承元数据标志规范化；临时拒绝规则已移除。此辅助断言失败如实保留，不标成产品缺陷或原始SDDL完全一致，不更改预先固定的产品失败预期。权限核对结果见[清理证据](../../.work/single-window-20261017/p001-nfr06/F01-acl-restored.json)。

原编辑进程显式重试同一r9，保存为generation 4、dirty清除；旧24个非根文件保持。保存后正常退出，再用全新PID243764从同一目录完整重开r9和D1新名称；全工程及20份引用文档读回通过。当前原有效权限已恢复，没有存留的故障注入进程或未保存输入。本轮没有修改产品源码。

## 实际证据

- [总进度JSON](../../.work/single-window-20261017/p001-nfr06/validation.json) / [F02实际结果](../../.work/single-window-20261017/p001-nfr06/F02-validation.json)
- [F02预先固定的条件与预期](../../.work/single-window-20261017/p001-nfr06/F02-precondition.json) / [实际界面观察](../../.work/single-window-20261017/p001-nfr06/ui-observations.jsonl)
- [失败时旧文件字节保留](../../.work/single-window-20261017/p001-nfr06/F02-failure-disk.json) / [旧工程完整内容核对](../../.work/single-window-20261017/p001-nfr06/F02-old-content-after-failure.json)
- [显式重试完整内容](../../.work/single-window-20261017/p001-nfr06/F02-retry-save-content.json) / [孤立新代与旧文件保留](../../.work/single-window-20261017/p001-nfr06/F02-retry-disk.json)
- [F01预先固定的下一输入](../../.work/single-window-20261017/p001-nfr06/F01-precondition.json)
- [F01实际结果](../../.work/single-window-20261017/p001-nfr06/F01-validation.json) / [失败时原目录字节保留](../../.work/single-window-20261017/p001-nfr06/F01-failure-disk.json)
- [F01旧完整工程](../../.work/single-window-20261017/p001-nfr06/F01-old-content-after-failure.json) / [重试后的完整工程](../../.work/single-window-20261017/p001-nfr06/F01-retry-save-content.json) / [重试磁盘核对](../../.work/single-window-20261017/p001-nfr06/F01-retry-disk.json)

实际1176/1177、F03/F04及完整NFR-06仍开放。本轮参考[Microsoft ReplaceFileW返回码说明](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew)核对失败后文件可能的位置；现有单元测试显式安排文件移动后果并构造1177错误，不计作实际Windows生成1177。当前实际错误5/32也不替代1176/1177。

本轮不增加原必需组计数。P0-01整包仍`in_progress`，`actual_result=null`、`review=null`、`whole_task_done=false`及人验边界保持。下一步按原Q51核查实际任务进行中的关闭/离开；[B60冻结输入与预期](../../.work/single-window-20261017/p001-native-task/precondition.json)已准备，尚未执行。
