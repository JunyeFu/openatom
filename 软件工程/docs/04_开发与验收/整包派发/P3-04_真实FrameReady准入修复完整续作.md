# P3-04 真实 FrameReady 准入修复完整续作

继续[原封存目录装载完整续作](P3-04_原生封存目录装载修复完整续作.md)全部双相机/九视图/原Host Task与release要求。Root实际03已进一步跑到真实FrameReady，不能在新的准入失败处停成完成。

## 已观测的实际03

Root唯一启动portable03，PID80268，原生真实exit1、约1.5秒终结。actual03的原stages证明正常http://tauri.localhost导航、原嵌入HTML/JS 200且正确MIME、真实DOMContentLoaded、plan.command和controller UI/COM/oneshot完成、窗口hidden/unfocused/非最小化1440×900/controller true。failure.json为step0 frame_admission：“FrameReady does not match the exact active plan/view/source/ScopeR/ProjectR or viewport”。无PNG；stderr另有退出时Chrome_WidgetWin_0注销1412原消息，保留不吞。actual01/02/03与旧候选全保留。

## 定位与修复要求

交原owner p0_01_host_core，继续原probe/lib窄源、本人新测试/证据/私有override和Cargo独占，不改Host/API/domain/公共配置锁/当前共享前端/原frozen-dist。先据实际源key和原浏览器FrameReady记录复现准入不等点，不能删掉Scope/R/view/camera/尺寸或一次在途检查凑通过。下一实际失败须保存收到的严格Frame DTO及具体不等字段，原前端仍不提交PNG/路径/窗口。

当前明确待核的源码点：validate_frame把JSON.stringify生成的key解析成serde_json::Value，再与Rust json!(原RenderViewSpec)直接相等；JS把23.0/1.0写作整数23/1，Rust浮点Serialize保留23.0/1.0时Value数值表示可能不同。必须以锁定serde_json/原生成类型及实际JS key证明，不以猜测修改。若确为该问题，按原key tuple的真实类型读取并比较规范RenderViewSpec/Revision/原source identity，保留完整相机/单位/LOD/ScopeR/ProjectR一致，不能用通配/忽略key或只比较部分字段。加入真正JS数值表示正例和相机/修订不等反例回归；没有必要不新增一套校验系统。

保持已修正Directory封存配置、原STA CapturePreview、Project/三个完整旧Run不变及全部真实就绪门。必要Native单元和同一frozen-dist custom-protocol release no-bundle locked/offline，原22/23历史不累加。仅新portable04，资源仍37映射/370文件/line-grade28、Fit4除外；旧候选与原raw不覆盖。真实双PNG仍由Root唯一实际运行，不由你启动或借CDP/显示窗口/外部截图/默认图替代。

完整交回复现的具体字段与实际序列化证据、精确diff/命令/退出码/raw、自审、正确新exe资源与Cargo释放。Root下一output为`.work/whole-packages/native-hidden-capture-root-actual-04`（不预建），input/views仍原两者。Root再实际核原图非空白/真实设备/相机差异/隐藏性及原Project/三Run不变；若又有真实失败继续同完整任务，不把“FrameReady已到”报为捕获成功。公开RenderView/managed/完整九视图及工业/人验/NFR/release全部保留。
