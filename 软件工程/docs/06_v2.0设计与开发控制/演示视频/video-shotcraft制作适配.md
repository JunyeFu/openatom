# video-shotcraft：超级工厂制作适配

2026-10-28；原V2-11-G01；技能固定提交 `43bd349a136651865ecfa6ea1cf012f6e957d304`。

当前采用**自主自由创作的方法，只执行环境配置与阶段0–3脚本设计**。现有订单主线、6分钟/15段、任务8核心272秒和真实范围保持；不套Ink Press模板，不启动新的产品/云录制、旁白或正式MP4。

## 环境入口

- 已安装技能：[C:\Users\fujunye\.codex\skills\video-shotcraft/SKILL.md](C:\Users\fujunye\.codex\skills\video-shotcraft/SKILL.md)。下轮对话可自动发现，本轮已按该固定源读取应用。
- 私有工具：`F:\SuperFactory-v2-video-20261028/tools`，运行前在pwsh内点导入 `Enter-VideoTools.ps1`；仅改当前进程PATH，不改系统配置。
- 实际检查：[environment-check.json](F:\SuperFactory-v2-video-20261028\shotcraft-setup\environment-check.json)；已有合成11.166667秒H.264/AAC测试片探测/完整解码通过；既有MotionStudy第150帧静帧渲染通过。
- Node24.15.0、Remotion4.0.539、React19.3.1、FFmpeg7.1沿用；FFprobe以平台专用npm包5.1.0补齐；uv0.12.18可用。没有新增Chrome下载。
- 本次发现的旧Zod4.5.4漏洞已在私有工具覆写至4.6.7；npm audit复查0项。Remotion当前固定要求4.5.4，会保留版本提示；静帧渲染已通过，新成片/Workbench兼容仍以实际制作验证，不降回已知有漏洞版本。
- 无BGM所以不装librosa；当前不做网页采集/剪映导出/云AI，故不装其可选依赖或配置付费服务。技能源码已含Workbench，但本轮没有成片可导入，不另起前端服务器。

## 产品视觉与整片方法

实际依据：`apps/planner/src/styles/workbench.css`，采用微软雅黑、浅蓝灰面板、深蓝正文与蓝色交互；保留中央大画布、左右栏、固定上下栏和主时间轴。镜头参数从“专业信赖/B2B”起步，21帧基础转场、无过冲、无手持抖动，阅读停留75–150帧。橙色只沿用试样中的单处关注标注。

技能中的30–60秒能量骨架仅作为参考：保留主体动作完整、落定呼吸、功能与阅读交替；6分钟主线继续订单→生产→维护/改派→结果→云机制/数据复用→保存与交付。不开长品牌序幕，不强制tech-house/BGM或发布会爆点。

## 三张选定镜头卡

|卡名 / Gallery style-key|应用|必须保留与适配|准确实现来源|
|---|---|---|---|
|title-demote-to-label / title-demote-to-label|镜头01，A|A式短业务标题，20帧站稳与26帧单次缩移保留；字体/色板改用实际产品；不以标题遮盖业务关键状态。|C:\Users\fujunye\.codex\skills\video-shotcraft\demos\typography\title-demote-to-label\TitleDemoteToLabel.tsx|
|cursor-flyover / cursor-flyover|镜头05，camera_path_adaptation_without_synthetic_UI_response|只采用经过调校的相机与指示分离语法。真实录屏不重造SVG点击或伪UI响应；阅读停留扩至75–120帧，坐标与字体清晰度按实际素材重标。|C:\Users\fujunye\.codex\skills\video-shotcraft\demos\camera\cursor-flyover\CursorFlyover.tsx|
|speed-ramp-freeze / freeze-annotate|镜头09，annotated_real_frame_hold|保留67帧定格和明确暂停信号，源画面/源时钟共冻；标“定格回看”。去除花哨白闪、手写署名和全屏噪声，圈注只对应已发生的维护事实。|C:\Users\fujunye\.codex\skills\video-shotcraft\demos\rhythm\speed-ramp-freeze\FreezeAnnotateReal.tsx|

已筛选全部124卡/150样式，选中卡全文已读，准确demo定位和关键时序已核对；本轮没有声称复刻或完整观看动态样片。进入逐镜头实现前再读选定demo全文，实际落点/裁切和源帧以素材重标。

不采用：来源节点吸收消失（四设备应保持独立）、剧场悬浮绕光（抢字段）、虚构版本时间轴（不是生产轨道）。图库可在[官方Gallery](https://vincentwei1021.github.io/video-shotcraft/library.html)查看。

## 帧级分镜与声音

|段序/镜头ID|时间|帧起点 / 时长|卡片|
|---|---|---|---|
|01/01 一张订单，遇到设备维护|00:00–00:18|0 / 540|title-demote-to-label|
|02/04 先认识订单和生产路线|00:18–00:43|540 / 750|真实录屏稳剪|
|03/03 确认四台设备的当前状态|00:43–01:08|1290 / 750|真实录屏稳剪|
|04/07 启动生产：从模型到一条命令|01:08–01:38|2040 / 900|真实录屏稳剪|
|05/08 跟着一件物品走完两道工序|01:38–02:13|2940 / 1050|真实录屏稳剪|
|06/09 核心转折：A维护，B承接后续工作|02:13–02:53|3990 / 1200|freeze-annotate|
|07/10 恢复与结果：回到这张订单|02:53–03:13|5190 / 600|真实录屏稳剪|
|08/02 回看机制：数据怎样变成生产决策|03:13–03:35|5790 / 660|真实录屏稳剪|
|09/05 刚才的生产数据可以查询|03:35–04:05|6450 / 900|cursor-flyover|
|10/06 同一份数据，独立读取并全量导出|04:05–04:25|7350 / 600|真实录屏稳剪|
|11/11 回到维护时点，解释这次改派|04:25–04:50|7950 / 750|真实录屏稳剪|
|12/12 保存之后，还能找回这次执行|04:50–05:10|8700 / 600|真实录屏稳剪|
|13/13 扩展能力：纸包装结构计算|05:10–05:25|9300 / 450|真实录屏稳剪|
|14/14 扩展能力：有界的Agent辅助|05:25–05:45|9750 / 600|真实录屏稳剪|
|15/15 交付：从演示到可以继续使用|05:45–06:00|10350 / 450|真实录屏稳剪|

完整操作、画面、旁白、逐字字幕、转场与待补镜头仍只取[现行脚本](演示视频脚本.md)。上表从同一JSON生成；source_frame_binding保持null，不以制作计划代替真实结果。

BGM默认关闭，中文讲解与读屏优先。定格标“定格回看”并冻结源时钟；不以busy状态代新命令、不造点击响应。SFX在画面锁定后用SHOTS[id].from+offset绑定，不模拟系统报警或真实机床。

用户“不自动commit/push”和当前只设计的范围高于上游默认制作流程。当前不恢复任务1、不打断窗口、不建子Agent；成片独立终检等属于未来实际制作阶段。
