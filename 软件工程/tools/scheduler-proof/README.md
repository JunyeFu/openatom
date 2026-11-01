# S06-01 原生队列隔离试做

仅编译std的最小堆、稳定键和观察窗边界；无产品/前端代码、无依赖安装或服务端口。

```powershell
New-Item -ItemType Directory -Force output
rustc --edition 2024 queue.rs -o output/std-queue-proof.exe
& output/std-queue-proof.exe
```

该输出不证明完整Engine、业务状态转换、NFR性能或SF/IX通过。全算法依据在`docs/03_架构与契约/调度与状态转换算法.md`。
