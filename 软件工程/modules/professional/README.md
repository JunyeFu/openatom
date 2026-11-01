# 专业计算模块

professional_contract声明输入、输出成果、单位/坐标、工具与支持范围。intended_modes是设计目标，available_modes是实际提供者能力，两者分开；缺提供者不能把声明工时当专业结果。

当前纸包装入口：[纸包装结构与过程模块](paper-packaging/README.md)，包含真实Rust计算生产者的接口、`Versioned<ModulePackage>`及五份可编辑Job输入。局部原语、内核/独立测试、Host消费和完整IX-03接收分别记录；可用能力仍以当前Host实际登记为准。当前模块为runtime_candidate，作者/许可未确认，不标发布或可再分发。

其他样例：[数控加工成果提供者](machining-producer/README.md)，尚无实际计算能力。完整41份方向保持[原专业工作](../../../模块研究/专业接入说明.md)，示例不是能力上限。成果、认可、消费者绑定和执行结果分开，工厂simulation才提交物品变化。协议见[九类清单](../../docs/03_架构与契约/九类模块清单与消费者.md)。
