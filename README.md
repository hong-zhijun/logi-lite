# Logi Lite

面向 Logitech MX Anywhere 3 的轻量鼠标增强工具，使用 Rust 共享协议代码。macOS 0.2 使用 Tauri 2 / WKWebView 界面，Swift / AppKit、CGEventTap 和 IOKit 后端；Windows 目前保留 Win32 界面，新界面的 Windows 适配尚未实现。基础鼠标输入由系统驱动处理，Logi Lite 提供快捷键映射与 HID++ 设备设置。

**当前提供 Windows x64 和 macOS 原生测试版。Mac 蓝牙 MX Anywhere 3 已读取电量和设备能力；用户已验证侧键映射、禁用、窗口关闭后常驻、配置保留、DPI、自由／段落滚动及蓝牙重连。SmartShift 阈值的实际效果尚未确认。**

## 功能

| 功能 | 当前状态 |
|---|---|
| Windows 托盘 / Mac 菜单栏，关闭窗口后继续运行 | 已实现 |
| 两个侧键、中键设为快捷键、默认或禁用 | 已实现，全局模式 |
| 本地保存配置，启动时恢复快捷键 | 已实现 |
| 暂停、恢复、退出和紧急暂停热键 | 已实现 |
| Logitech HID 端点识别与诊断导出 | 已实现 |
| 电量、硬件 DPI、滚轮设置读取 | 实验性，待真实设备验证 |
| DPI、自由／棘轮滚动、SmartShift 调节 | 实验性，待真实设备验证 |
| macOS 菜单栏、权限引导、输入与蓝牙 HID++ 通信 | 已实现测试版；只读通信已实测 |

Windows 快捷键映射首次启动默认关闭；macOS 首次完成授权后默认启用，之后保留手动暂停偏好。两平台均使用全局鼠标监听，**会影响所有鼠标的对应侧键和中键**；硬件 DPI 和滚轮设置只对识别出的目标设备开放。

暂不提供顶部模式键重映射、按应用配置、Flow、云同步、复杂宏或开机自启动。

## 构建与运行

需要 Windows x64、支持 Rust 2024 edition 和 let chains 的 Rust 工具链（Rust 1.88 或更新版本），以及 Visual Studio Build Tools 的 C++ 构建工具和 Windows SDK。项目使用 MSVC 工具链，已在 Rust 1.97.1 上构建验证。

```powershell
git clone https://github.com/hong-zhijun/logi-lite.git
cd logi-lite
cargo build --release --locked --target-dir target
.\target\release\logi-lite.exe
```

首次构建需要下载 Cargo 依赖；依赖缓存后可加 `--offline`。Windows x64 构建配置使用静态 CRT，运行时无需 Python、Qt 或 Electron。

构建产物与本地便携包不纳入 Git，克隆仓库后需自行构建。

## macOS 构建与测试

需要 macOS 13 或更新版本、Rust 1.88+ 和 Xcode Command Line Tools。只需编译时具备这些工具；打包后的应用使用系统框架，不依赖 Options+ 或后台 Python / Electron。

```bash
./scripts/build-macos.sh
open "dist/Logi Lite.app"
```

应用路径为 `dist/Logi Lite.app`。脚本构建当前 Mac 的原生架构，并进行本地 ad-hoc 签名；不是已公证的互联网发行包。先构建应用后，`cargo run --release` 也会启动它。首次构建会下载 Tauri 依赖；界面静态资源直接嵌入应用，运行时不启动本地网页服务器。

1. 测试前自行退出 Options+ 后台代理，避免同时处理按键。仅关闭其设置窗口不等于退出后台。
2. 权限不足时，设备配置区显示遮罩和“去授权”入口，完成辅助功能和输入监控授权后自动解锁；设置页始终可访问，必要时退出重开。首次授权完成后默认启用映射；之后会记住手动暂停，权限暂时撤销时停止监听，重新授权后恢复原先启用状态。
3. 程序自动识别直接通过蓝牙连接的 MX Anywhere 3，读取电量、DPI、滚轮模式及支持范围。首次探测、重连和通信恢复均为只读，不自动重写上次硬件设置。连接元数据每 4 秒检查一次；未成功通信时，窗口可见期间最多每 30 秒自动重试一次。
4. 鼠标首页点击图中的后退、前进、中键标记；选择默认、禁用、打开 App、调度中心或键盘快捷键，点击“保存并应用”。快捷键支持手输或录制。设置页开关可以启用／暂停已保存的映射。映射作用于所有鼠标。
5. 点击同一鼠标图中的“指针与滚轮”标记，调整 DPI、自由／段落／SmartShift 模式及灵敏度，点击“应用到鼠标”；后端重新验证目标和范围、仅写入修改项，再回读实际结果。后台读取不会覆盖尚未提交的编辑。断开或读取失败时禁用写入，重连后自动读取，也可点击右上角刷新按钮重新读取状态。按钮用橙色提示待应用，绿色勾号表示已保存／已同步。
6. MX Anywhere 3 的真实 DPI 为 200–4000，步进 50；Options+ 的指针速度百分比是不同刻度。SmartShift 灵敏度为 0–100%，越高越容易进入自由滚动，回读百分比可能因硬件量化略有变化。以设备读取能力为准，证据见 [设置范围](docs/settings-ranges.md)。
7. 点击右上角圆形设置按钮管理权限、手动刷新和导出诊断。关闭窗口后仍在菜单栏运行，可重新打开、启用／暂停和退出。紧急暂停为 `Cmd+Ctrl+Shift+F12`，部分键盘需要 Fn；手动或紧急暂停会保存关闭映射；退出只停止监听，保留启用偏好供下次打开恢复。

Mac 的按键映射是全局模式，影响所有鼠标的对应侧键和中键；不支持按应用配置或顶部模式键重映射。当前 Mac 硬件后端针对直接连接的蓝牙 MX Anywhere 3 系列，不提供 Unifying / Bolt 接收器槽位选择；多只目标鼠标同时连接时拒绝自动选择写入对象。

Mac 配置路径：`~/Library/Application Support/LogiLite/config.json`。读取或校验失败会保留原文件，显示错误并关闭映射。点击保存或对已启用配置进行安全暂停时会原子替换配置。新界面每个用户仅允许一个实例运行。硬件设置不在启动/重连时自动重写。

```bash
# 只列端点和权限状态；不发送硬件报告、不申请权限
"dist/Logi Lite.app/Contents/MacOS/LogiLite" --diagnose diagnostic.json
# 只读查询真实硬件；不申请权限、不写设置
"dist/Logi Lite.app/Contents/MacOS/LogiLite" --probe probe.json
# 窗口/菜单栏自检；不加载或保存配置、不启动映射、不发送硬件报告
"dist/Logi Lite.app/Contents/MacOS/LogiLite" --smoke-test smoke.json
./scripts/check-macos.sh
```

已安装 Options+ 的只读分析及证据见 [Mac 分析记录](docs/options-macos-analysis.md)。Logi Lite 控制逻辑使用独立实现，没有复制官方程序代码或依赖其私有服务。新界面鼠标图取自本机 Options+，素材来源与分发限制见 [UI 记录](docs/tauri-macos-preview.md)。

## 使用方法

### 快捷键

1. 启动程序。测试映射或硬件设置前，先退出 Options+ 的后台代理，避免两套软件同时处理按键或覆盖设备设置。仅关闭官方窗口不一定退出后台。
2. 选择两个侧键和滚轮中键的动作，勾选启用，再点击“保存并应用快捷键”。
3. 可先用 `Ctrl+C`、`Ctrl+V` 在记事本测试，中键保持“默认”。
4. 点击窗口 × 后程序仍在通知区运行。双击托盘图标打开设置，右键可暂停、恢复或彻底退出。

支持 `Ctrl`、`Alt`、`Shift`、`Win` 与字母、数字、F1–F24、常用导航键的组合，例如 `Ctrl+Shift+S`、`Alt+Left`、`Win+Tab`。每次点击执行一次完整快捷键；不支持用按住鼠标按钮持续按住键盘键。

紧急暂停／恢复：**`Ctrl+Alt+Shift+F12`**。热键被占用时仍可使用托盘菜单。

### 设备设置

连接鼠标后点击“查找 / 刷新设备”。程序先读取协议和能力，成功识别后才开放对应控件。

- **电量**：手动刷新更新。只返回档位的设备不会显示虚构的精确百分比。
- **DPI**：使用设备返回的支持值、范围和步长，区别于 Windows 指针速度。
- **滚轮模式**：自由滚动、棘轮滚动或 SmartShift。
- **SmartShift 阈值**：允许 1–49，仅在 SmartShift 模式使用。保留设备现有滚轮力度，不修改高分辨率滚轮报告模式。

刷新只读取状态；点击“应用到鼠标”才写设置。应用前重新确认型号与功能索引，写入后回读确认。多项设置中途失败可能已有部分生效，以回读状态为准。

## 兼容性与限制

| 平台／设备 | 说明 |
|---|---|
| Windows x64 | 已构建并完成桌面自检 |
| macOS 13+ | 原生应用；已在本机 Apple Silicon / macOS 27.0.1 构建与蓝牙只读通信验证；其他系统/芯片未实测 |
| MX Anywhere 3 | 首要目标；普通版通常使用 Unifying 或 Bluetooth LE |
| MX Anywhere 3S／商务版 | 识别逻辑可匹配，通常涉及 Bolt；功能和端点仍需实测 |

HID++ 通信依赖系统暴露可访问的厂商 HID collection。若蓝牙只暴露专有 GATT 通路，当前版本可能无法读取或修改硬件设置；可尝试平时使用的接收器。未识别到目标时不会开放硬件写入。

Windows `SendInput` 受权限限制，普通权限程序向管理员窗口、UAC 或部分游戏发送快捷键可能不生效。其他鼠标映射软件与 Options+ 也可能影响结果。

彻底退出后，自定义快捷键停止处理，基础输入继续由系统提供。硬件设置持久性取决于固件，省电、关机或重连可能恢复默认。本版不在启动或重连时自动重写硬件设置，需要刷新后重新应用。

## 配置与诊断

Windows 配置目录：`%LOCALAPPDATA%\LogiLite`。

| 文件 | 用途 |
|---|---|
| `config.json` | 快捷键配置，启动时自动加载 |
| `hardware-settings.json` | 已成功应用并回读的硬件设置记录 |
| `diagnostic.json` | 点击“导出诊断”保存的端点、能力和错误 |

暂停状态只影响当前运行。诊断不记录鼠标轨迹或输入内容；分享前可检查设备路径等元数据。

命令行只读端点枚举：

```powershell
.\target\release\logi-lite.exe --diagnose .\diagnostic.json
```

## 开发与验证

```powershell
cargo fmt --all --check
cargo clippy --all-targets --locked --target-dir target -- -D warnings
cargo test --locked --target-dir target
```

目前 23 项自动测试通过，覆盖配置校验、配对按键状态、模拟输入释放、HID++ 响应匹配、迟到包、超时、设备能力、DPI 和 SmartShift 协议。Windows 桌面自检验证了托盘创建、关闭窗口后常驻、重入暂停和退出；也运行过只读 HID 元数据枚举。

**这些结果不等于真实鼠标兼容性验证。** 电量、DPI、滚轮和物理侧键的端到端效果仍需连接鼠标测试。

无设备连接、映射关闭的短时样本约为 11.6 MiB 工作集、2.1 MiB 私有提交量、5 个线程。这不是连接设备后的性能保证，也不是与 Options+ 的完整对照测试。

桌面自检会短暂创建窗口、安装默认关闭的输入监听，保存截图并自动退出，不发送 HID++ 请求：

```powershell
.\target\release\logi-lite.exe --smoke-test .\smoke-test.json
```

## 目录结构

```text
src/
├─ main.rs                 平台入口
├─ lib.rs                  模块导出
├─ shared/
│  ├─ config.rs            配置模型、校验与保存流程
│  └─ hidpp.rs             与系统无关的 HID++ 协议
└─ platform/
   ├─ windows/
   │  ├─ app.rs            窗口与托盘
   │  ├─ input.rs          Windows 输入映射
   │  ├─ devices.rs        HID 枚举与异步通信
   │  ├─ worker.rs         设备工作线程
   │  └─ storage.rs        Windows 配置路径与文件替换
   └─ macos/
      ├─ mod.rs            macOS 适配入口
      ├─ bridge.rs         Rust / Swift JSON 与 HID++ 回调桥接
      ├─ storage.rs        macOS 配置存储
      └─ native/           AppKit 界面、事件监听与 IOHID 蓝牙通信
```

两套平台在 `main` 分支共同维护，通过目录和条件编译区分系统实现。共享配置校验在 Mac 支持 Cmd / Command、Option / Alt、Ctrl / Control、Shift，Win 为 Cmd 别名；Mac 输入后端独立转换到 macOS 虚拟键码，支持 F1–F20。Windows 首版保存在 [`windows-v0.1.0`](https://github.com/hong-zhijun/logi-lite/tree/windows-v0.1.0) 标签。

`target/`、`dist/` 与本地调查目录 `analysis/` 均被 Git 忽略。

## 协议参考

HID++ 客户端根据公开协议消费实现中的数据格式独立编写，参考：

- [Microsoft HID 架构](https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/hid-architecture)
- [Solaar：报告封装、响应匹配与错误](https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/base.py)
- [Solaar：功能发现与电量格式](https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/hidpp20.py)
- [Solaar：DPI 与 SmartShift 设置](https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/settings_templates.py)
- [libratbag：MX Anywhere 3 定义](https://github.com/libratbag/libratbag/blob/master/data/devices/logitech-MX-Anywhere-3.device)

## 许可证

[MIT](LICENSE)。Logi Lite 是独立项目，与 Logitech 无隶属关系。
