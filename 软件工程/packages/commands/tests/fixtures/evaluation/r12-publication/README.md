# R12报告发布的固定输入

此夹具保留合成纸包装案例的实际保存工程、两个冻结Run与其同源报告。它来自2026-10-21上一轮包装生命周期中的sleeve-part_relations_or_repeat_order固定输入，只复制测试实际读取的saved-project及report.json，原26份工程文件和报告逐字节不变。

用于noncu_report_publication.rs通过真实Host测试四格式发布、取消、同名竞争、持有失败和新进程重开。保存工程和原报告不是当前测试的预计算通过结果；测试仍运行当前Host并核对原Run与输入。原历史Win32-5的时刻持有者未知，不由当前成功或本夹具改写。

无需本机.work或.inputs目录；NONCU_PACKAGING_INPUT仅供明确复用其他相同目录布局的冻结输入。原始失败日志仍保留在内部历史交接，未放入测试夹具。
