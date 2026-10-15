# MX Anywhere 3 顶部按钮与 HID++ 映射

调查日期：2026-10-15。目标是滚轮后方的实体顶部按钮，不是按下滚轮的中键。

## 本机 Options+ 的配置证据

目标 agent 为 `/Library/Application Support/Logitech.localized/LogiOptionsPlus/logioptionsplus_agent.app/Contents/MacOS/logioptionsplus_agent`，SHA-256：`7d71273c6c3310edaebee729bf8f1f255c541f08651331bd6068dbd7a7ff3fc9`。

REA `inspect_macho` Evidence：`ev_ac55cfc923d4e38511653b3dc30f3e56801ba11971d1822e45d60b96cf30ffff`。这是静态架构／导入导出清单，不是调用跟踪，也没有建立下述函数的执行路径。完整本地符号表另经 `xcrun nm -arch arm64 -n` 核对，含 `Feature1B04SpecialKeys::GetCount`、`GetCtrlIdInfo`、`GetCtrlIdReporting`、`SetCtrlIdReporting` 与 `KeyReport` 订阅方法。未启动 Options+，未请求动态追踪。

设备资源目录为 `/Library/Application Support/Logi/LogiOptionsPlus/depots/805973/mx_anywhere_3/`。REA 惰性资源清单 parent Evidence：`ev_1a4ca2a309c900179dc766328d9fb1f6d64d27afc7820b1f535ff85fdada565c`，目录摘要 `a23f2d04fb0e4420d875934ff1eec09f3745a9e6e1dfecadfcc4de9eda0b8e71`。此目录只有 JSON／图片，没有 JavaScript 模块；资源语义来自对以下 JSON 的格式化读取，而非 JavaScript 图或运行时行为：

| 文件 | SHA-256 | 关键内容 |
|---|---|---|
| `core_metadata.json` | `fdff0fb48d0cde9d15cd9ba7e25facd68a374017a610bd843a1ff07039c83e01` | 顶部 slot 为 `mx-anywhere-3-6b025_c196`，名称 `SLOT_NAME_MODESHIFT_BUTTON`，根名 `TOP_BUTTON`；中键另为 `_c82`。顶部标记 x=75、y=34，中键 x=71、y=16。 |
| `default_configurations.json` | `dfa6c3de5fa7449d80dcb4d270460eac1aa8b996ba6f9fbe21a2ccd035cf0d50` | `SPECIAL_KEYS_1B04.control_id_mappings["196"]` 默认 `card_global_presets_mode_shift`，`taskId:157`，`macro:{}`；中键 CID 82，后退 83，前进 86。 |

因此本机 Options+ 资源将顶部按钮标识为 CID **196 / 0x00C4**，默认动作为切换滚轮模式。这里没有把鼠标图片中的百分比标记坐标当作 USB 报告坐标，也没有假定它是 CGEvent 按钮 5。

## 公开协议与交叉核对

协议字段来自公开的 [HID++ 0x1B04 文档](https://lekensteyn.nl/files/logitech/x1b04_specialkeysmsebuttons.html)，并与 [Solaar hidpp20.py](https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/hidpp20.py)、[special_keys.py](https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/special_keys.py)、[notifications.py](https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/notifications.py) 相互核对。这些是公开协议／开源实现，不能称为 Options+ 原生算法反编译结果。

2026-10-15 获取的文件摘要依次为 `86ae37b86c5016b7d8d53c369b375bca7ac41323bceed7e170d1265826fe4c3d`、`b6dfbd88bba8dcbf6b3eb2de5ea1345c2b02f56a67e23799a1b2a5c7f635aaa7`、`947d5ddc40936af46c563027509b188132dc823af695ab8eb93551a321a4fe54`、`5486692b07b6633362ed1dcd6a9cd33ebffd59c06b7b1fc4f3b69aa3f4077854`。GitHub `master` 是可变引用；以上摘要绑定本次读取内容。

先通过 ROOT 发现 `0x1B04` 的实际 feature index，不能固定写死 feature index。下表函数码不含调用者 software ID：

| 函数 | 函数码 | 请求数据与回包 |
|---|---:|---|
| getCount | `0x00` | 回包首字节为控制项数量。 |
| getCidInfo | `0x10` | 请求一个表索引字节；回包 CID(2)、taskID(2)、flags低位(1)、position(1)、group(1)、groupMask(1)、flags高位(1)。CID 为大端序。 |
| getCidReporting | `0x20` | 请求 CID(2)；回包 CID(2)、flags低位(1)、remapCID(2)，新版本可再有 flags高位。 |
| setCidReporting | `0x30` | CID(2)、flags低位(1)、remapCID(2)。仅改临时 diversion 时不需要高位 flags；`remapCID=0` 表示保持原映射。 |

`getCidInfo` flags 的 `0x20` 表示可临时 diversion，`0x40` 表示可持久 diversion。应实际枚举 CID 和能力，不能仅凭型号允许写入。

`setCidReporting` 的 bit 0 是临时 divert，bit 1 是其 valid 标志。因此启用顶部自定义的最小 payload 是 `00 C4 03 00 00`，清除临时 diversion 是 `00 C4 02 00 00`。不设置 persistent valid、raw XY 或 raw wheel，避免改变其他行为。响应需要与请求关联，并用 `getCidReporting` 验证最终 bit 0。

## 按下、释放与默认动作

1B04 event 0 的 address 为 `0x00`（software ID 为 0）。数据前八字节是四个大端 CID，表示**当前按住的全部 diverted 控制项**。按键释放时它从列表移除；其他键释放会使其位置移动。因此应根据 `0x00C4` 是否存在检测按下沿，不能只读第一个 CID，也不能把每份报告当作一次点击。event 1 是 raw XY，不是按钮报告；请求响应同样不能当作按钮通知。

设备不会因更改 diversion 配置主动发送按键报告。正在按住的按钮保留当前这次按下使用的状态，新 diversion 仅作用于下一次实体按下。重新配置／暂停应清空程序的 held 状态，不能补执行动作。

默认使用固件滚轮模式切换：不进行 diversion，让实体按钮完成原生行为。自定义时临时 diversion 抑制原生行为，由软件处理通知。暂停、恢复默认、正常退出时应显式清除本程序拥有的临时 diversion。仅关闭 IOHID 会话并不保证恢复，协议仅保证发生 HID++ 配置 reset（见 feature 0x0020 策略）时清除临时状态。断线重连后需重新发现 feature、读取能力与 reporting，再决定恢复自定义；关闭应用窗口不应关闭后台通知监听。

若已有 persistent diverted（reporting bit 2），仅清除临时 bit 0 不能恢复原生行为，因为任一 diversion 位均会抑制原生动作。若检测到其他软件拥有的 diversion／remap，应提示冲突或保留其状态，不能无条件清除持久配置。`remapCID=0` 保留原状态；映射回自身 CID 才是恢复原生 remap，两者含义不同。

## 证据边界

已回答：顶部按钮与中键的身份区分、CID、Options+ 默认配置、公开协议的枚举／diversion 字段、通知按下释放语义及恢复要求。没有观察真实用户按下、没有写设备设置、没有运行 Options+ 目标，也没有证明某个固件实际支持该能力。硬件支持及稳定性需要由设备回包和用户实测确认。Logi Lite 的会话组织、重连策略和动作执行是本项目实现选择。

本次 REA native target 已 `close_binary` 成功返回 `result:null`，未启动新的 Hopper 深度会话。

## Logi Lite 实现与本机验证

macOS 配置新增可选 `top`，默认 `默认`；与中键动作独立，旧配置不需要迁移。鼠标图新增顶部热点，复用快捷键、系统动作、App 选择与分类动作编辑器；水平滚动仅属于前进／后退侧键。

所有 HID++ 事务与通知由同一后台线程的 run loop 串行持有，避免刷新 DPI／滚轮时另开会话与通知监听竞争。通知仅接收完整长报告，按 CID 集合的按下沿触发一次。暂停、配置修订、断开与休眠会使排队通知失效；权限撤销检查与重连不依赖窗口轮询。默认不接管，遇到其他软件已有的 temporary／persistent diversion 或改写的 remap 则提示冲突，不清除其配置。

正常暂停、默认、休眠和退出显式清除本程序的 temporary diversion，并读回确认；运行期间恢复失败保留会话每三秒重试。重连只对内存中记录的稳定设备标识恢复本程序的接管，不把其他鼠标当作已拥有的设备；标识不写入诊断或配置。强制终止、断电及不可通信状态无法保证成功释放；若设备保留临时状态，可关闭再开启鼠标电源恢复后重试。

本机只读 `--probe` 成功：MX Anywhere 3、Bluetooth Low Energy、协议4.5；枚举到顶部 CID196、实际 feature index9，支持临时接管。当前 reporting_flags=1；进程清单确认 `logioptionsplus_agent --launchd` 仍在运行。未为了测试而退出用户进程、修改用户按键配置或写入 diversion。该回包证实能力接口可用，不证明实体按下事件或自定义动作已实测。

验证包含：28项 Rust 测试；Swift 配置兼容、解析和输入配对测试；HID++ 顶部枚举、临时接管/释放与读回、无效响应拒绝、CID集合换位、重复报告、释放及非按钮通知；页面两种宽度的布局和新增顶部动作检查。发行包另运行隔离自检，不修改用户配置、不请求权限、不写设备或执行映射动作。

最终构建 `dist/Logi Lite.app` 成功，隔离 Tauri 桌面自检返回 `ok:true`（含7项新增顶部按钮页面检查），配置写入、设备写入、映射启用与权限请求均为 false。发行 zip 已重新生成，严格代码签名验证通过。界面在1080×800及920×800检查，顶部热点与中键分离。
