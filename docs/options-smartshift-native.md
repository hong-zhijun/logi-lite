# Options+ SmartShift 灵敏度与硬件阈值换算

2026-10-14 静态核对本机 Options+ agent 的 arm64 指令。此处未启动、调试或调用 Options+ 后台，未写鼠标设置。

## 结论

Options+ 的用户灵敏度是 **0～100**。本机 agent 在启用 SmartShift 时转换成硬件阈值 **4～50**，数值方向相反：灵敏度越高，硬件触发阈值越低。

```text
s = clamp(userSensitivity, 0, 100)
hardwareThreshold = 50 + roundTiesAwayFromZero((-46.0 * s) / 100.0)
```

硬件回读成用户灵敏度：

```text
h = clamp(hardwareThreshold, 4, 50)
userSensitivity = truncateUnsigned(((h - 50) * 100.0) / -46.0)
```

硬件只有整数阈值，故回读的百分比不保证等于请求值。

| 用户请求灵敏度 | 写入硬件阈值 | Options+ 公式回读灵敏度 |
|---:|---:|---:|
| 0 | 50 | 0 |
| 1 | 50 | 0 |
| 5 | 48 | 4 |
| 25 | 38 | 26 |
| 50 | 27 | 50 |
| 75 | 15 | 76 |
| 100 | 4 | 100 |

原应用若显示硬件阈值 12，它对应 Options+ 回读灵敏度 82；不能直接把 12 称为 Options+ 灵敏度。

## 制品与指令证据

- 制品：`/Library/Application Support/Logitech.localized/LogiOptionsPlus/logioptionsplus_agent.app/Contents/MacOS/logioptionsplus_agent`
- SHA-256：`7d71273c6c3310edaebee729bf8f1f255c541f08651331bd6068dbd7a7ff3fc9`
- 工具：本机 `nm -arch arm64 -n` 与 `xcrun llvm-objdump --macho --arch=arm64 --disassemble --dis-symname SYMBOL PATH`。按两个已定位的 `SettingsManager::read/write` 符号限定反汇编；同名局部符号另有一份拷贝，转换指令一致。
- 完整目标函数旁证：[read 指令](evidence/options-smartshift-read-arm64.asm)、[write 指令](evidence/options-smartshift-write-arm64.asm)。这些是 shell 静态证据，不是 REA 成功返回的 native Evidence。

`SettingsManager::write` 位于 `0x1009c7084`。关键区间：

- `0x1009c70c0–0x1009c70d4`：读取用户字段并限制到 0～100。
- `0x1009c70dc–0x1009c70f0`：将数值乘以 IEEE-754 double `0xc047000000000000`（−46），再除以 `0x4059000000000000`（100）。
- `0x1009c70f4`：`frinta d0,d0`，取最近整数，恰好一半时远离零。
- `0x1009c70f8–0x1009c7104`：加 `0x4049000000000000`（50），转换整数。
- `0x1009c710c`：未启用时使用 `0xff`，不经过灵敏度转换。
- `0x1009c7130–0x1009c7150`：转换的阈值送入设备虚调用。后续重复转换用于写入确认。

`SettingsManager::read` 位于 `0x1009c6ea8`。关键区间：

- `0x1009c6f50–0x1009c6f64`：硬件阈值限制到 4～50。
- `0x1009c6f68–0x1009c6f84`：减 50，乘 100，除 −46。
- `0x1009c6f88`：`fcvtzu w8,d0`，向零截断为无符号整数。
- `0x1009c6f8c`：写入上层 SmartShiftSettings 字段 `+0x10`。write 从相同字段读取，因此这里是上层用户灵敏度。

### raw 50 与锁定段落模式的区别

`0x1009c6f3c–0x1009c6f4c` 的 `cmp mode,2; ccmp threshold,255,...,eq; cset ne` 明确生成 `enabled = !(mode == 2 && threshold == 255)`。因此 `(mode=2,threshold=50)` 属于已启用的 SmartShift，回读用户灵敏度为 0；不能用 `threshold >= 50` 将其识别为锁定段落。`(2,255)` 才走这里的关闭 SmartShift 路径。

下层 `Feature2110SmartShift::GetRatchetControlMode` 的 `0x1003f6824–0x1003f6834` 直接将响应 `byte[0]` 写为 mode、`byte[1]` 写为 threshold；`SetRatchetControlMode` 的 `0x1003f68d0–0x1003f68d4` 直接发送这两个字节，没有隐藏的单位换算。函数旁证：[2110 get](evidence/options-smartshift-2110-get-arm64.asm)、[2110 set](evidence/options-smartshift-2110-set-arm64.asm)。

## 范围与限制

这里证明的是本机 Options+ 通用 SmartShift 设置管理代码的百分比换算。上层滑块 0～100 的证据来自主调查的 app.asar；本文件不另行复查 UI 资源。

不能把 Options+ 的 4～50 输出范围等同于所有硬件/协议允许值的全部范围。停用路径的 `255` 与灵敏度端点 `0 → 50` 是不同路径，软件状态已由读写指令与 2110 协议实现交叉确认。本文未实测滚轮机械行为，未继续反汇编 2111 设备层。

REA `open_binary` 接受了制品并报告 SHA-256，但随后 `read_bytes` 触发 Hopper bridge 启动超时；未获得有效 native Evidence ID。已调用 `close_binary`，服务器仍报告 `cleanup_incomplete`：`hopper-document` 与 `/tmp/rea-CigRLK` 的关闭未获确认。没有为此安装分析引擎或重开目标。
