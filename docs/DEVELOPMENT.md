# 开发指南

产品源码位于 `软件工程/`。只想运行应用，请使用[正式下载](https://github.com/JunyeFu/superfactory-releases/releases/tag/v2.0.1)。

## 获取源码与资源

```powershell
git clone https://github.com/JunyeFu/openatom.git
cd openatom
git lfs install --local
git lfs pull
cd 软件工程
```

GLB、PDF、视频和压缩包使用 Git LFS；指针文件不能替代实际资源。

## 环境与启动

按项目声明使用 Windows 11 x64、MSVC C++ 构建工具、WebView2 Runtime、Rust 1.97.0、Node.js 24.15.0、npm 12.0.1。版本依据为 `rust-toolchain.toml`、`package.json` 和锁文件。

```powershell
npm ci --ignore-scripts
npm run typecheck
npm run start
```

开发入口使用 `127.0.0.1:8922`，启动前确认没有重复实例。构建桌面调试候选：

```powershell
npm run build --workspace @superfactory/planner
```

## 按改动范围检查

```powershell
npm test
cargo test -p sf-simulation --tests --locked
```

云服务是独立 Cargo 工作区，见[云服务说明](../软件工程/apps/cloud-service/README.md)。Agent入口见[MCP Bridge](../软件工程/apps/mcp-bridge/README.md)。H1诊断按[工具说明](../软件工程/tools/fv0-h1/README.md)执行。

## 参与协作

先看[当前状态](STATUS.md)及[协作入口](../软件工程/docs/06_v2.0设计与开发控制/GitHub协作入口.md)，按对应 Issue 的输入和范围提交变更。历史设计中的 planned 状态保留原阶段含义，当前安排以 FV0—FV4 控制资料为准。
