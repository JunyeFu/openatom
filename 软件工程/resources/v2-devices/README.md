# 随包可信设备运行时

CPython **3.14.9 Windows x64 embeddable**，来自[Python官方发布页](https://www.python.org/downloads/release/python-3149/)。实际下载包已按官方SHA-256核对；来源、原校验值和大小见runtime.json。

- python/保留官方嵌入发行文件与LICENSE.txt；licenses/保留原Python许可证和官方SPDX。未安装pip、site-packages或全局环境。
- python/python314._pth固定三行：`python314.zip`、`.`、`../scripts`；没有import site。Native使用`-I -B -X utf8`，不搜索PATH解释器。
- scripts/仅有已交回的virtual_device.py、journal.py、protocol.py和无密钥devices.demo.json。双工序profile为10/15秒；整厂按冻结阶段名义工时的1/30执行、每阶段至少1秒，私有启动参数明确指定，实际回执保留真实经过时间。
- token只经私有stdin。SQLite/boot journal在Native私有appdata，不进入这里或portable。

实际导入检查：Python3.14.9、SQLite3.50.4、OpenSSL3.5.9，及ssl/urllib/threading/json/monotonic/SQLite读写和生产脚本。详细组件结果见 `.work/v2-implementation-20261026/native-device-runtime/receipt.json`，不代Sealos/Host/Engine整链验收。
