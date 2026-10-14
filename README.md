# Logi Lite

面向 Logitech MX Anywhere 3 的轻量鼠标增强工具，使用 Rust 和系统原生界面实现。基础鼠标输入由 Windows 自带 HID 驱动处理，Logi Lite 提供快捷键映射与 HID++ 设备设置。

**当前版本为 Windows x64 测试版。macOS 后端尚未完成，真实鼠标通信与物理按键行为仍待实机验证。**

## 功能

| 功能 | 当前状态 |
|---|---|
| 托盘常驻，关闭设置窗口后继续运行 | 已实现 |
| 两个侧键、中键设为快捷键、默认或禁用 | 已实现，全局模式 |
| 本地保存配置，启动时恢复快捷键 | 已实现 |
| 暂停、恢复、退出和紧急暂停热键 | 已实现 |
| Logitech HID 端点识别与诊断导出 | 已实现 |
| 电量、硬件 DPI、滚轮设置读取 | 实验性，待真实设备验证 |
| DPI、自由／棘轮滚动、SmartShift 调节 | 实验性，待真实设备验证 |
| macOS 菜单栏、输入与设备通信 | 待实现 |

快捷键映射首次启动默认关闭。首版使用 Windows 全局鼠标监听，**会影响所有鼠标的对应侧键和中键**；硬件 DPI 和滚轮设置只对识别出的目标设备开放。

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
| macOS | 目前只有配置存储适配边界，没有可用 Mac 应用 |
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
      └─ storage.rs        macOS 配置存储
```

两套平台在 `main` 分支共同维护，通过目录和条件编译区分系统实现。共享快捷键解析目前保留 Windows 键名和内部键码约定，Mac 输入后端仍需要适配。Windows 首版保存在 [`windows-v0.1.0`](https://github.com/hong-zhijun/logi-lite/tree/windows-v0.1.0) 标签。

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
