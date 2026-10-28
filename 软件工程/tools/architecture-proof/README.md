# S05-05 原生接点试做

仅设计阶段隔离技术试做，不是产品代码、18项公开API或SF/IX运行验收。目录有自己的Cargo清单/锁，不写apps/packages、用户全局MCP或Figma。依赖选用已查官方元数据的版本；client/子进程特性仅测试驱动使用，产品桥只需server/stdio。

目标：实际官方Rust MCP SDK stdio初始化/发现/调用，经当前登录会话ACL命名管道访问一个隔离Host fixture；两个调用源以同R修改只能一个成功，旧冻结输入保持；实际Windows文件替换失败保留旧有效文件；Rust JSON/TS/schema生成读取同一类型。

不存在产品工程、库存或求解结果。使用 `lab_context` / `lab_set_duration` 两个实验工具，不向产品目录注册新工具。不以实验工具数修改18项计划操作的实际可用状态。

执行方法和真实结果在源完成、编译/运行后记录。桌面Tauri invoke/3D正确帧/四格式中文报告与纸包装几何接点仍需其他实际试做，不能据本目录单独关闭S05-05。

实际执行：在本目录运行 `cargo run --locked --manifest-path Cargo.toml`。output/native-trial-result.json、fixture.schema.json、fixture.types.txt及current-fixture.json为本机实际输出。初次ts-rs API编译问题已修；原生子集通过，当前产品可用操作仍0，不证明Codex/桌面/FPS/完整纸包装或四格式。

几何/报告：本目录 `cargo run --locked --manifest-path Cargo.toml -- geometry-report`；两盒体独立解析预期gap=1、首次撞击t=1实际通过。输出自建48行四格式，中文两页/图片及特殊字符重开与渲染检查已通过，不能当真实工厂报告。字体来源/原OFL许可在assets内，未修改；PNG是自建图像嵌入测试图。
