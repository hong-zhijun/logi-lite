# macOS 鼠标按键映射动作与配置值

更新日期：2026-10-15。适用：本项目 Logi Lite Mac 版；参考本机 Options+ 2.5.926888。

## 当前 Logi Lite 动作与配置值

点击鼠标上的后退侧键、前进侧键、中键或顶部按钮，选择动作后点击“保存并应用”。选择“更多”后，先选大类再选具体动作；选择“键盘快捷键”后，点击只读输入框直接录制，不支持手动输入。配置文件中的 `back`、`forward`、`middle`、`top` 使用下表字符串。

| 动作 | 填写的值 | 说明 |
|---|---|---|
| 保留鼠标原行为 | `默认` | 不拦截该按键 |
| 禁用按键 | `禁用` | 拦截该按键，不执行动作 |
| 打开 App 应用列表 | `打开 App` | 直接打开系统应用；旧系统回退到 Launchpad |
| 打开调度中心 | `调度中心` | 直接打开系统应用，无需设置快捷键 |
| 水平滚动 | `horizontalScroll.back`／`horizontalScroll.forward` 布尔值 | 前进／后退侧键各自独立开关；按住侧键＋滚轮横向滚动，中键无此功能 |
| 打开指定 App | `打开应用:/Applications/Example.app` | 从原生文件选择器选择 .app，显示真实应用图标和名称；已打开时激活，路径失效时提示错误 |
| 系统设置 | `系统设置` | 按系统 bundle ID 打开 |
| 计算器 | `计算器` | 按系统 bundle ID 打开 |
| 锁定屏幕 | `Ctrl+Cmd+Q` | 使用 macOS 对应快捷键 |
| 音量加／减 | `音量增加`、`音量减小` | 发送系统媒体键 |
| 静音 | `静音切换` | 发送系统静音键 |
| 播放／暂停 | `播放或暂停` | 交由 macOS 当前媒体播放器处理 |
| 上／下一首 | `上一首`、`下一首` | 交由 macOS 当前媒体播放器处理 |
| 亮度加／减 | `亮度增加`、`亮度减小` | 依赖系统支持的内建或外接显示器 |
| 复制 | `Cmd+C` | 发送快捷键，由当前应用处理 |
| 粘贴 | `Cmd+V` | 同上 |
| 剪切 | `Cmd+X` | 由当前应用决定是否支持 |
| 撤销 | `Cmd+Z` | 同上 |
| 重做 | `Cmd+Shift+Z` | 部分软件使用其他快捷键 |
| 全选 | `Cmd+A` | 由当前应用处理 |
| 保存 | `Cmd+S` | 由当前应用处理 |
| 打印 | `Cmd+P` | 由当前应用处理 |
| 关闭当前窗口／标签页 | `Cmd+W` | 具体行为由当前应用决定 |
| 最小化当前窗口 | `Cmd+M` | 当前应用支持时生效 |
| 切换应用 | `Cmd+Tab` | 发送一次按下和释放，不会持续按住应用切换器 |
| 浏览器后退 | `Cmd+Left`（取决于浏览器） | 部分浏览器支持，部分不支持或将它用于移动光标。优先保留侧键默认行为，或使用目标浏览器已设置的快捷键 |
| 浏览器前进 | `Cmd+Right`（取决于浏览器） | 同上，焦点位于文本框时可能移动光标 |
| 新建标签页 | `Cmd+T` | 浏览器等支持标签页的应用 |
| 关闭标签页 | `Cmd+W` | 由当前应用处理 |
| 恢复关闭的标签页 | `Cmd+Shift+T` | 浏览器支持时生效 |
| 下一个标签页 | `Ctrl+Tab` | 由当前应用处理 |
| 上一个标签页 | `Ctrl+Shift+Tab` | 由当前应用处理 |
| 刷新页面 | `Cmd+R` | 由当前应用处理 |
| 聚焦搜索 | `Cmd+Space` | 要求系统对应快捷键已启用 |
| 查看当前应用的窗口 | `Ctrl+Down` | 要求系统“应用程序窗口”快捷键已启用 |
| 调度中心的快捷键替代方式 | `Ctrl+Up` | 要求系统对应快捷键已启用；推荐直接选 `调度中心` |
| 切换到左侧桌面 | `Ctrl+Left` | 要求系统对应快捷键已启用，并存在相邻桌面 |
| 切换到右侧桌面 | `Ctrl+Right` | 同上 |
| 显示桌面 | `F11` | 要求系统“显示桌面”快捷键设为 F11 并已启用 |
| 截取全屏 | `Cmd+Shift+3` | 要求系统对应快捷键已启用 |
| 区域截图 | `Cmd+Shift+4` | 同上 |
| 打开截屏／录屏工具栏 | `Cmd+Shift+5` | 同上 |
| 表情与符号 | `Ctrl+Cmd+Space` | 要求系统对应快捷键可用 |

“更多”的分类为系统功能、编辑、窗口与桌面、浏览器、媒体与截图。菜单会保存对应的动作值或快捷键；快捷键是否执行取决于 macOS 设置、当前应用及焦点。表中以 `Cmd`、`Ctrl` 或 `F11` 等开头的值是快捷键，其余明确列出的中文值是原生动作。不要将未列出的中文描述直接填入配置。

浏览器前进／后退没有本程序保证通用的快捷键值。当前键名解析器不支持 `[`、`]`，也不支持任意标点键；`Option+Left`／`Option+Right` 在 Mac 上经常用于按单词移动光标，不能当作通用浏览器导航键。

### 快捷键语法

- 格式：零个或多个修饰键，加一个普通键，以 `+` 连接，如 `Cmd+Shift+Z`。
- 修饰键：`Cmd`／`Command`／`Win`、`Ctrl`／`Control`、`Option`／`Alt`、`Shift`。Mac 上 `Win` 是 Command 的别名。
- 普通键：`A`–`Z`、`0`–`9`、`F1`–`F20`、`Enter`、`Space`、`Tab`、`Esc`、`Delete`、`Left`、`Right`、`Up`、`Down`、`Home`、`End`、`PageUp`、`PageDown`。
- 快捷键忽略大小写，不能重复同一修饰键。`Delete` 当前映射为向前删除，不是退格。
- 暂不支持录制 `Fn`／地球键、媒体键、快捷键序列或持续按住修饰键；媒体操作从“更多”选择。

配置示例：

```json
{
  "version": 1,
  "enabled": true,
  "back": "打开 App",
  "forward": "调度中心",
  "middle": "默认",
  "horizontalScroll": {"back": false, "forward": false}
}
```

## Options+ 动作参考及待实现项目

下表的英文值是本机 Options+ 前端资源中的动作／图标标识，用于调查与后续开发，**不是已确认的配置协议枚举，也不是 Logi Lite 输入值**。资源包含多种设备和平台；出现一个标识不能证明它在当前鼠标的 Mac 界面中一定可选。

| 动作 | Options+ 资源标识 | Logi Lite 当前状态 |
|---|---|---|
| App／Launchpad | `Launchpad` | 已实现：`打开 App` |
| 调度中心 | `MissionControl` | 已实现：`调度中心` |
| 当前应用的所有窗口 | `SLOT_NAME_APP_EXPOSE` | “更多”可选，使用系统快捷键 |
| 显示桌面 | `ShowDesktop` | “更多”可选，使用系统快捷键 |
| 左／右切换桌面 | `DesktopLeft`、`DesktopRight`、`SwitchDesktops` | “更多”可选，使用系统快捷键 |
| 切换应用 | `SwitchApps`、`NavigateApps` | “更多”可选，发送一次快捷键 |
| 最小化／最大化／关闭窗口 | `MinimizeWindow`、`MaximizeWindow`、`CloseWindow` | “更多”提供最小化／关闭；最大化未实现 |
| 打开指定应用 | `OpenApp` | 已实现：选择 .app，保存为 `打开应用:/绝对路径.app` |
| 打开文件／文件夹 | `OpenFile`、`OpenFolder` | 未实现 |
| 打开网页 | 待核对准确标识 | 未实现；上一轮功能概述提及，尚未定位专用入口 |
| 打开系统设置／计算器 | `OpenSettings`、`OpenCalculator` | 已实现：`系统设置`、`计算器` |
| 剪切／粘贴／重做 | `Cut`、`Paste`、`Redo` | 可使用快捷键 |
| 自定义快捷键 | `Shortcut` | 已支持单组快捷键 |
| 后退／前进 | `Back`、`Forward` | 保留默认侧键可沿用应用原行为；无独立导航动作 |
| 新建／关闭标签页 | `NewBrowserTab`、`CloseBrowserTab` | “更多”可选，使用应用快捷键 |
| 切换标签页／刷新 | `NavigateTabs`、`Refresh` | “更多”可选，使用应用快捷键 |
| 播放／暂停 | `Play`、`Pause` | 已实现：`播放或暂停` |
| 上一首／下一首 | `PreviousTrack`、`NextTrack` | 已实现：`上一首`、`下一首` |
| 音量加／减／静音 | `IncreaseVolume`、`DecreaseVolume`、`Mute`、`Unmute` | 已实现音量加减与静音切换 |
| 截图／区域截图 | `ScreenCapture`、`ScreenSnip` | “更多”可选，使用系统快捷键 |
| 听写 | `Dictation`、`SLOT_NAME_DICTATION` | 独立动作及 Fn 双击未实现 |
| 表情菜单 | `EmojisMenu` | “更多”可选，使用系统快捷键 |
| 切换输入语言 | `InputLanguage` | 可映射已启用且语法支持的系统快捷键 |
| 通知中心 | `SLOT_NAME_NOTIFICATION_CENTER` | 未实现独立动作 |
| 勿扰模式 | `DoNotDisturbOn`、`DoNotDisturbOff` | 未实现 |
| 双击／三击 | `DoubleClick`、`TrippleClick` | 未实现；`TrippleClick` 是资源中的原拼写 |
| 模拟左／右／中键 | `LeftClick`、`RightClick`、`MiddleClick` | 未实现独立鼠标动作；中键本身可保留默认 |
| 横向滚动 | `HorizontalScroll`、`ScrollLeft`、`ScrollRight` | 已实现前进／后退独立开关，按住侧键＋滚轮；中键不支持 |
| 智能缩放／缩放 | `SmartZoom`、`ZoomInOut`、`ZoomOut` | 未实现独立动作；部分应用可用快捷键 |
| 改变指针速度 | `ChangePointerSpeed` | 可在硬件设置中改 DPI，尚不能通过侧键切换 |
| 切换滚轮模式 | `ShiftWheelMode` | 可在硬件设置中选模式，尚不能通过侧键切换 |
| 手势按钮 | `Gestures`、`SLOT_NAME_GESTURE_BUTTON` | 实验已移除；见 [手势按钮实验记录](gesture-button-experiment.md) |
| 按应用配置 | 存在 application profiles 相关资源；此处不指定枚举 | 未实现，目前映射全局生效 |
| 连续执行多个动作 | `SmartActions` | 未实现 |

不要把 Options+ 的内部数值 ID 跨功能混用。当前静态证据仅确认 `Input_System` 分发中的 **8 = Launchpad/App、9 = Mission Control**；这两个值属于 Options+ 自己的协议，不可填入 Logi Lite。

## 实现与证据位置

- 当前配置校验：[config.rs](../src/shared/config.rs)。
- Mac 输入解析、系统动作及按键发送：[Input.swift](../src/platform/macos/native/Input.swift)。
- Options+ App／调度中心调用链与系统 bundle ID：[options-system-actions-native.md](options-system-actions-native.md)。
- 其他动作标识来自本机 ASAR 的只读资源分析：`analysis/options-ranges/app.decoded-static.js`，原文件身份和静态解析方法见 [options-range-observations.json](options-range-observations.json)。这些标识仅作为资源存在证据，不表示逐项运行验证。

水平滚动是动作卡片下方的独立开关，不占用侧键的普通点击映射；有滚动时消耗该次侧键点击，没滚动则在松开时执行原动作。旧版 `水平滚动` 动作值会迁移为对应侧键开关＋默认动作，旧中键值恢复默认。Options+ 对照证据见 [水平滚动记录](options-horizontal-scrolling.md)。

本记录不改变用户映射、不配置系统快捷键，也不表示上表所有待实现功能已加入程序。

## 顶部按钮

`top` 指滚轮后方的实体按钮，与 `middle`（按下滚轮）独立。旧配置缺少 `top` 时自动按 `默认` 处理，不改变原映射。顶部按钮可以使用上表的快捷键、指定 App、常用动作与禁用；不提供水平滚动开关。

顶部按钮的 `默认` 在界面显示“切换滚轮模式”，由鼠标固件执行。其他动作通过 HID++ `0x1B04` 临时接管 CID `0x00C4`；关闭窗口后继续工作，暂停、恢复默认、正常退出时释放，重连后重新发现能力。若看到橙色占用提示，先退出 Options+ 后台，必要时关闭再开启鼠标电源。异常强制结束进程无法保证释放临时接管。详情与验证范围见 [顶部按钮记录](options-top-button.md)。
