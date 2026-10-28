# V2-03 可信设备子进程

Python标准库实现；公共wire为 `sf.execution/2.0-draft11`。消费现有C03/C04/C14/C19，没有增加云端字段或端点。`execution` 与 `telemetry_only` 是同一可信脚本的两种启动模式。平台使用Sealos；脚本不创建或配置任何云资源。

## 文件与运行

- `virtual_device.py`：真实子进程入口、HTTPS客户端、单写者循环、私有控制。
- `journal.py`：SQLite持久化和进程独占锁；poll/command/outbox原payload保留。
- `protocol.py`：设备实际消费的typed边界检查。
- `devices.demo.json`：按任务8案例配置的不含凭据的10/15秒演示profile；组件计时测试单独使用250/400毫秒。
- Native 的 `factory-commands` 配置用于整厂软件演示：同一物理端点可接收多个冻结阶段，执行时间为 `max(1000, ceil(nominal_work_ms / 30))` 毫秒。私有启动信封明确传入 `processing_time_source=command` 和 `processing_speed=30`；界面说明此演示速度。Started/Completed、有效工时与Host时钟仍来自真实执行，不以名义工时直接结算，也不作为实体设备产能。
- `tests/`：无GUI、无服务端口的组件测试和自有短进程夹具。

在软件工程根目录执行：

```powershell
python -X utf8 -m unittest discover -s integrations/devices/tests -v
```

组件测试使用明确的进程内HTTP客户端替身；生产入口始终使用真正的stdlib HTTPS客户端。测试不替换生产Host/Native IPC。真实云、Native启动/退出持有、随包Python3.14.9及Sealos互通仍由公共集成验证。

## Native启动约定（私有stdin，不是新公共wire）

只启动已安装、可信的脚本，不执行用户工程内的脚本。参数中只有Python和脚本位置：

```python
child = subprocess.Popen(
    [trusted_python, "-u", trusted_script],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, encoding="utf-8",
    creationflags=subprocess.CREATE_NO_WINDOW,  # Windows Native调用方
)
child.stdin.write(json.dumps(bootstrap) + "\n")
child.stdin.flush()
```

Native持有真实Child并持续读stdout；不使用shell、不按名称/PID批量关闭。地址、token不进入argv、工程JSON、stdout、SQLite或报告。

第一行必须是一个完整JSON，且只有以下11个字段：

```json
{
  "contract_version": "sf.execution/2.0-draft11",
  "mode": "execution",
  "base_url": "https://YOUR-SEALOS-SERVICE.example",
  "token": "DEVICE-SCOPED-SECRET-SUPPLIED-IN-MEMORY",
  "journal_path": "D:/Native-private/host-a/session-a/device-a/boot-a/device.sqlite",
  "device_id": "device-a",
  "device_boot_id": "boot-a",
  "dataset_id": "execution-dataset-a",
  "identity": {
    "session_id": "session-a",
    "execution_epoch_id": "epoch-a",
    "project_id": "project-a",
    "host_context_session_id": "host-a"
  },
  "processing_time_ms": 250,
  "accept_commands": false
}
```

- `base_url`只允许固定HTTPS origin（无用户名、密码、查询、fragment或业务路径）。正常校验证书，所有重定向均拒绝，设备不会向其他地址转发凭据。
- `journal_path`必须为绝对路径，由Native按host/session/device/boot隔离，不进入工程导出。每次新boot使用新目录；不能换boot覆盖旧journal。
- `processing_time_ms`是可信设备profile的实际软件在环等待时长；不使用命令的 `nominal_work_ms` 自动报完成。0表示该profile确实无等待。
- `accept_commands`可先设false。注册只证明C19身份，不证明freshness或Host已prepared；Host仍按原C13/发布及probe协议决定准备和派工。
- `telemetry_only`启动使用独立、预置的采集dataset、`identity:null`、`accept_commands:false`；不调用C03/C19，不产生加工receipt。dataset授权和purpose最终由服务端固定范围确认。

## 同stdin控制与stdout

bootstrap之后每行一个控制JSON：

```json
{"action":"status"}
{"action":"set_accept_commands","accept_commands":true}
{"action":"shutdown"}
```

`set_accept_commands`仅执行模式可用，关闭已开始后不能重新开接单。即便false或busy也继续C03轮询probe，pending poll精确复发其原accept_commands，不修改正在重试的请求。

stdout为JSON行：

- `event:status`含 `registered/state/accept_commands/shutdown_ready/pending_poll/pending_events/sampled_slots/missed_slots/last_error` 和本设备/boot身份。`registered:false`时C19尚未确认；状态输出不是云端接纳或设备新鲜度证据。
- `event:shutdown_blocked`表示仍有active command、未知poll、未确认outbox或未确认注册；设置停止接单并继续原义务，不强杀。Native可后续status/重发shutdown。关闭意图下idle不再制造新的poll/采集slot，但会结清原pending poll；busy仍轮询probe。
- `event:shutdown,state:idle,outbox_settled:true`之后进程正常退出0。Native还须核验OS退出、原义务结清，再由gateway调用C20、最后C10。设备专属token不调用gateway专属退役接口。
- stdin EOF表示父持有者已离开，停止新接单并尝试结清后正常退出；仍有未知义务时不以EOF代替结清。
- 协议/身份/持久化错误仅输出固定 `device_requires_reconciliation` 并退出2；不打印请求、地址、token或任意服务端错误文本。原journal保留。非正常退出不视为C20退役。

## 运行语义

1. 启动先写固定boot身份并持有独占文件锁。第二进程无法成为同journal写者。已有journal即使正常退出过也拒绝自动重启；崩溃后没有跨进程续做或按墙钟补Completed，先交Native/Host核对原义务。
2. poll在发送前落盘。收到原command/empty后，响应副本、命令accepted/outbox及新probe样本同事务提交。相同poll/command/probe身份不同payload冲突；复发旧poll仍保留原响应和probe。
3. 已接受命令先读C14。未知则保持accepted；`cancel_requested=true`在未Started时生成 `cancelled_not_started`。已Started时仅首次生成 `cancel_rejected_started` 并继续原动作。claimed本身不是取消成功。
4. Started与outbox同事务提交后才建立本进程的 `monotonic_ns` 起点。真实经过profile时长才写Completed及整数实际duration，与outbox同事务。journal内的started决策时刻不是跨进程恢复计时依据。
5. 一个boot共享递增sequence。所有receipt和telemetry按sequence FIFO发送；HTTP全在事务外。超时/连接未知/408/429/5xx使用原bytes重发，250ms后重试；不另领命令、不新建attempt、不再次加工。HTTPS请求2秒超时用于恢复循环，不将超时当成功。
6. 新probe在本进程单写者循环采当前状态并生成新sequence；同probe仅保留第一份原样本，不能追加到旧outbox、换新标识或续期。Host发起/到期时间不由设备或云时间重算。
7. 采集模式按本进程真实monotonic第1秒、第2秒等slot采样。阻塞错过的slot只记 `missed_slots`，不回填假历史；重启不会把历史计数当本轮数据。当前组件只实际验证2秒，不声称10设备×1800秒/18000条已验。

## 本批验证与接收边界

实际结果、命令与日志见 `.work/v2-implementation-20261026/devices/receipt.json`。测试包含发送前落盘、精确重试/FIFO、不可变payload冲突、三个状态事务回滚、取消两侧、probe新采样/去重、跨进程单写者/崩溃留痕、两个独立加工子进程的真实250/400ms等待、真实2秒采集、私有stdin正常idle关闭及HTTP错误/凭据边界。

Native启动器、C19/C20/C10完整生命周期、Sealos HTTP/PostgreSQL认证与持久化、Host probe准入、Engine库存及产量都不由这些组件测试冒充通过；本脚本从不修改物料/资源/接受量。
