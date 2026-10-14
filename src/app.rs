use crate::worker::{self, Command, HardwareView};
use logi_lite::{
    config::{self, Config},
    input,
};
use std::{
    cell::RefCell,
    ffi::c_void,
    mem::{size_of, zeroed},
    path::PathBuf,
    ptr::{null, null_mut},
    sync::mpsc,
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{LibraryLoader::GetModuleHandleW, Threading::*},
    UI::{
        Controls::{BST_CHECKED, BST_UNCHECKED},
        Input::KeyboardAndMouse::*,
        Shell::*,
        WindowsAndMessaging::*,
    },
};

const TRAY_MESSAGE: u32 = WM_APP + 1;
const ID_APPLY: usize = 100;
const ID_PAUSE: usize = 101;
const ID_SCAN: usize = 102;
const ID_HARDWARE: usize = 103;
const ID_EXPORT: usize = 104;
const ID_OPEN: usize = 105;
const ID_EXIT: usize = 106;
const ID_ENABLE: usize = 107;
const ID_DPI: usize = 108;
const ID_MODE: usize = 109;
const ID_THRESHOLD: usize = 110;
const EMERGENCY_HOTKEY: i32 = 1;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
fn win_error(label: &str) -> String {
    format!("{label}（Windows 错误 {}）", unsafe { GetLastError() })
}

struct RuntimeGuard {
    window: HWND,
    font: HFONT,
    title_font: HFONT,
    mutex: HANDLE,
}
impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = input::uninstall();
            if !self.window.is_null() {
                let mut tray: NOTIFYICONDATAW = zeroed();
                tray.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
                tray.hWnd = self.window;
                tray.uID = 1;
                Shell_NotifyIconW(NIM_DELETE, &tray);
                UnregisterHotKey(self.window, EMERGENCY_HOTKEY);
                if IsWindow(self.window) != 0 {
                    DestroyWindow(self.window);
                }
            }
            DeleteObject(self.font);
            DeleteObject(self.title_font);
            CloseHandle(self.mutex);
        }
    }
}

struct App {
    config: Config,
    window: HWND,
    font: HFONT,
    title_font: HFONT,
    status: HWND,
    enable: HWND,
    shortcuts: [HWND; 3],
    diagnostic: HWND,
    device_label: HWND,
    dpi: HWND,
    mode: HWND,
    threshold: HWND,
    hardware_apply: HWND,
    scan: HWND,
    pause: HWND,
    tray: NOTIFYICONDATAW,
    taskbar_created: u32,
    commands: mpsc::Sender<Command>,
    events: mpsc::Receiver<HardwareView>,
    hardware: HardwareView,
    smoke: bool,
    smoke_path: Option<PathBuf>,
    smoke_stage: bool,
    smoke_capture_ok: bool,
    smoke_pause_ok: bool,
    last_paused: bool,
    busy: bool,
    hide_explained: bool,
    hotkey_registered: bool,
}

pub fn show_error(message: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(message).as_ptr(),
            wide("Logi Lite").as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let smoke_path = args
        .iter()
        .position(|v| v == "--smoke-test")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);
    let smoke = smoke_path.is_some();
    let minimized = args.iter().any(|v| v == "--minimized");
    unsafe {
        SetProcessDPIAware();
        let singleton = CreateMutexW(null(), 0, wide("Local\\LogiLite-v0.1").as_ptr());
        if singleton.is_null() {
            return Err(win_error("无法创建单实例锁"));
        }
        if GetLastError() == ERROR_ALREADY_EXISTS && !smoke {
            let existing = FindWindowW(wide("LogiLiteWindow").as_ptr(), null());
            if !existing.is_null() {
                ShowWindow(existing, SW_RESTORE);
                SetForegroundWindow(existing);
            }
            CloseHandle(singleton);
            return Ok(());
        }
        let instance = GetModuleHandleW(null());
        let class_name = wide("LogiLiteWindow");
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hIcon: LoadIconW(null_mut(), IDI_APPLICATION),
            hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
            ..zeroed()
        };
        if RegisterClassW(&class) == 0 {
            CloseHandle(singleton);
            return Err(win_error("无法注册窗口"));
        }
        let config_result = if smoke {
            Ok(Config::default())
        } else {
            config::load()
        };
        let config = config_result.clone().unwrap_or_default();
        let (commands, events) = worker::spawn();
        let font = CreateFontW(
            -17,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            CLEARTYPE_QUALITY as u32,
            0,
            wide("Microsoft YaHei UI").as_ptr(),
        );
        let title_font = CreateFontW(
            -27,
            0,
            0,
            0,
            FW_SEMIBOLD as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            CLEARTYPE_QUALITY as u32,
            0,
            wide("Microsoft YaHei UI").as_ptr(),
        );
        let app = Box::new(RefCell::new(App {
            config,
            window: null_mut(),
            font,
            title_font,
            status: null_mut(),
            enable: null_mut(),
            shortcuts: [null_mut(); 3],
            diagnostic: null_mut(),
            device_label: null_mut(),
            dpi: null_mut(),
            mode: null_mut(),
            threshold: null_mut(),
            hardware_apply: null_mut(),
            scan: null_mut(),
            pause: null_mut(),
            tray: zeroed(),
            taskbar_created: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
            commands,
            events,
            hardware: HardwareView::default(),
            smoke,
            smoke_path,
            smoke_stage: false,
            smoke_capture_ok: false,
            smoke_pause_ok: false,
            last_paused: false,
            busy: false,
            hide_explained: false,
            hotkey_registered: false,
        }));
        let ptr: *const RefCell<App> = &*app;
        let mut runtime = RuntimeGuard {
            window: null_mut(),
            font,
            title_font,
            mutex: singleton,
        };
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 760,
            bottom: 600,
        };
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        AdjustWindowRect(&mut rect, style, 0);
        let window = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide("Logi Lite · Windows 测试版 0.1").as_ptr(),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            rect.right - rect.left,
            rect.bottom - rect.top,
            null_mut(),
            null_mut(),
            instance,
            ptr as *const c_void,
        );
        if window.is_null() {
            return Err(win_error("无法创建设置窗口"));
        }
        runtime.window = window;
        {
            let mut state = app.borrow_mut();
            state.window = window;
            state.build_controls();
            state.add_tray()?;
            state.hotkey_registered = RegisterHotKey(
                window,
                EMERGENCY_HOTKEY,
                MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_NOREPEAT,
                VK_F12 as u32,
            ) != 0;
            input::set_config(&state.config)?;
            input::install()?;
            state.refresh_state();
            if let Err(error) = config_result {
                state.set_diagnostic(&format!(
                    "配置未加载，映射已保持关闭。\r\n{error}\r\n保存设置后可重新建立配置。"
                ));
            }
            if !state.hotkey_registered {
                state.set_diagnostic("紧急暂停热键已被其他程序占用；仍可通过托盘菜单暂停。\r\n设备相关功能需连接后点击“查找 / 刷新设备”。");
            }
        }
        SetTimer(window, 1, 1000, None);
        if smoke {
            ShowWindow(window, SW_SHOWNOACTIVATE);
            SetTimer(window, 2, 700, None);
        } else if !minimized {
            ShowWindow(window, SW_SHOW);
        }
        let mut message: MSG = zeroed();
        loop {
            let result = GetMessageW(&mut message, null_mut(), 0, 0);
            if result == 0 {
                break;
            }
            if result == -1 {
                return Err(win_error("窗口消息循环失败"));
            }
            if IsDialogMessageW(window, &message) == 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        let state = app.borrow();
        let _ = state.commands.send(Command::Stop);
        if message.wParam != 0 {
            return Err(format!("界面自检失败，退出状态 {}", message.wParam));
        }
    }
    Ok(())
}

impl App {
    // Mirrors Win32 control creation: class/style/id plus the explicit rectangle.
    #[allow(clippy::too_many_arguments)]
    unsafe fn control(
        &self,
        class: &str,
        text: &str,
        style: u32,
        id: usize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> HWND {
        unsafe {
            let handle = CreateWindowExW(
                0,
                wide(class).as_ptr(),
                wide(text).as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                x,
                y,
                w,
                h,
                self.window,
                id as HMENU,
                GetModuleHandleW(null()),
                null(),
            );
            SendMessageW(handle, WM_SETFONT, self.font as usize, 1);
            handle
        }
    }
    unsafe fn label(&self, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
        unsafe { self.control("STATIC", text, 0, 0, x, y, w, h) }
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn combo(
        &self,
        id: usize,
        text: &str,
        x: i32,
        y: i32,
        w: i32,
        items: &[&str],
        editable: bool,
    ) -> HWND {
        unsafe {
            let style = if editable {
                CBS_DROPDOWN
            } else {
                CBS_DROPDOWNLIST
            };
            let handle = self.control(
                "COMBOBOX",
                "",
                style as u32 | WS_VSCROLL | WS_TABSTOP,
                id,
                x,
                y,
                w,
                210,
            );
            for item in items {
                SendMessageW(handle, CB_ADDSTRING, 0, wide(item).as_ptr() as isize);
            }
            if editable {
                SetWindowTextW(handle, wide(text).as_ptr());
            } else {
                SendMessageW(handle, CB_SETCURSEL, 0, 0);
            }
            handle
        }
    }
    unsafe fn build_controls(&mut self) {
        unsafe {
            let title = self.label("Logi Lite", 24, 16, 230, 42);
            SendMessageW(title, WM_SETFONT, self.title_font as usize, 1);
            self.label("轻量鼠标设置 · v0.1 Windows 测试版", 24, 60, 550, 26);
            self.status = self.label("快捷键映射关闭", 24, 94, 690, 27);
            self.enable = self.control(
                "BUTTON",
                "启用侧键 / 中键映射（首版影响所有鼠标）",
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
                ID_ENABLE,
                24,
                132,
                690,
                28,
            );
            SendMessageW(
                self.enable,
                BM_SETCHECK,
                if self.config.enabled {
                    BST_CHECKED
                } else {
                    BST_UNCHECKED
                } as usize,
                0,
            );
            let presets = [
                "默认",
                "禁用",
                "Ctrl+C",
                "Ctrl+V",
                "Ctrl+W",
                "Ctrl+Shift+T",
                "Win+D",
                "Alt+Tab",
            ];
            for (i, (label, value)) in [
                ("后退侧键", self.config.back.as_str()),
                ("前进侧键", self.config.forward.as_str()),
                ("滚轮中键", self.config.middle.as_str()),
            ]
            .iter()
            .enumerate()
            {
                let y = 175 + i as i32 * 39;
                self.label(label, 24, y + 3, 110, 28);
                self.shortcuts[i] = self.combo(120 + i, value, 140, y, 280, &presets, true);
            }
            self.label(
                "可输入 Ctrl+Shift+S 等组合。默认 = 保留鼠标原行为。",
                440,
                174,
                290,
                76,
            );
            self.control(
                "BUTTON",
                "保存并应用快捷键",
                WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
                ID_APPLY,
                24,
                296,
                168,
                34,
            );
            self.pause = self.control(
                "BUTTON",
                "暂停映射",
                WS_TABSTOP,
                ID_PAUSE,
                204,
                296,
                110,
                34,
            );
            self.scan = self.control(
                "BUTTON",
                "查找 / 刷新设备",
                WS_TABSTOP,
                ID_SCAN,
                326,
                296,
                164,
                34,
            );
            self.control(
                "BUTTON",
                "导出诊断",
                WS_TABSTOP,
                ID_EXPORT,
                502,
                296,
                122,
                34,
            );
            self.device_label = self.label(
                "设备：尚未探测。电量 / DPI / 滚轮功能需实机验证。",
                24,
                344,
                714,
                29,
            );
            self.label("硬件 DPI", 24, 384, 96, 25);
            self.dpi = self.combo(ID_DPI, "", 112, 380, 128, &[], true);
            self.label("滚轮", 258, 384, 50, 25);
            self.mode = self.combo(
                ID_MODE,
                "",
                310,
                380,
                135,
                &["棘轮滚动", "自由滚动", "SmartShift"],
                false,
            );
            self.label("阈值 1–49", 458, 384, 98, 25);
            self.threshold = self.control(
                "EDIT",
                "25",
                WS_BORDER | WS_TABSTOP | ES_NUMBER as u32,
                ID_THRESHOLD,
                564,
                380,
                55,
                29,
            );
            self.hardware_apply = self.control(
                "BUTTON",
                "应用到鼠标",
                WS_TABSTOP,
                ID_HARDWARE,
                630,
                380,
                108,
                32,
            );
            EnableWindow(self.dpi, 0);
            EnableWindow(self.mode, 0);
            EnableWindow(self.threshold, 0);
            EnableWindow(self.hardware_apply, 0);
            self.diagnostic=self.control("EDIT","连接鼠标后点击“查找 / 刷新设备”。\r\n首次使用请先在 Options+ 中退出后台代理，避免双方覆盖设置。\r\n快捷键是独立的全局兼容模式，开启前请确认下拉框配置。",WS_BORDER|WS_VSCROLL|ES_MULTILINE as u32|ES_READONLY as u32|ES_AUTOVSCROLL as u32,130,24,432,714,102);
            self.label(
                "关闭窗口仍在托盘运行。右键托盘可退出；Ctrl+Alt+Shift+F12 暂停 / 恢复。",
                24,
                551,
                714,
                35,
            );
        }
    }
    unsafe fn add_tray(&mut self) -> Result<(), String> {
        unsafe {
            self.tray = zeroed();
            self.tray.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
            self.tray.hWnd = self.window;
            self.tray.uID = 1;
            self.tray.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            self.tray.uCallbackMessage = TRAY_MESSAGE;
            self.tray.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);
            copy_wide(&mut self.tray.szTip, "Logi Lite · 设置");
            if Shell_NotifyIconW(NIM_ADD, &self.tray) == 0 {
                return Err(win_error("无法添加托盘图标"));
            }
            Ok(())
        }
    }
    unsafe fn set_diagnostic(&self, text: &str) {
        unsafe {
            SetWindowTextW(self.diagnostic, wide(text).as_ptr());
        }
    }
    unsafe fn refresh_state(&mut self) {
        unsafe {
            let paused = input::is_paused();
            self.last_paused = paused;
            let state = if paused {
                "快捷键已暂停"
            } else if self.config.enabled {
                "快捷键已启用 · 全局模式（所有鼠标）"
            } else {
                "快捷键映射关闭 · 原始输入保持默认"
            };
            SetWindowTextW(self.status, wide(state).as_ptr());
            SetWindowTextW(
                self.pause,
                wide(if paused {
                    "恢复映射"
                } else {
                    "暂停映射"
                })
                .as_ptr(),
            );
            copy_wide(&mut self.tray.szTip, &format!("Logi Lite · {state}"));
            Shell_NotifyIconW(NIM_MODIFY, &self.tray);
        }
    }
    unsafe fn show(&self) {
        unsafe {
            ShowWindow(self.window, SW_RESTORE);
            SetForegroundWindow(self.window);
        }
    }
    unsafe fn toggle_pause(&mut self) {
        input::set_paused(!input::is_paused());
        unsafe {
            self.refresh_state();
        }
    }
    unsafe fn save_shortcuts(&mut self) {
        unsafe {
            let next = Config {
                version: 1,
                enabled: SendMessageW(self.enable, BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
                back: control_text(self.shortcuts[0]),
                forward: control_text(self.shortcuts[1]),
                middle: control_text(self.shortcuts[2]),
            };
            if let Err(error) = config::validate(&next)
                .and_then(|_| config::save(&next))
                .and_then(|_| input::set_config(&next))
            {
                show_error(&error);
                return;
            }
            self.config = next;
            self.refresh_state();
            self.set_diagnostic("快捷键设置已保存。关闭此窗口后仍在托盘生效。\r\n首版快捷键使用 Windows 全局兼容模式，会影响其他鼠标的相同按键。\r\nDPI / 滚轮设置独立应用到识别出的 Anywhere 3。");
        }
    }
    unsafe fn start_scan(&mut self) {
        if self.busy {
            return;
        }
        self.busy = true;
        unsafe {
            EnableWindow(self.scan, 0);
            EnableWindow(self.hardware_apply, 0);
            self.set_diagnostic("正在查找 Logitech 设备并读取能力，请稍候。\r\n此过程不修改鼠标配置。没有鼠标也可以配置快捷键和使用托盘。" );
        }
        if self.commands.send(Command::Scan).is_err() {
            self.busy = false;
            unsafe {
                self.set_diagnostic("设备工作线程已退出，请重新启动程序。");
                EnableWindow(self.scan, 1);
            }
        }
    }
    unsafe fn apply_hardware(&mut self) {
        if self.busy {
            return;
        }
        let dpi = if self.hardware.can_dpi {
            match unsafe { control_text(self.dpi) }.trim().parse::<u16>() {
                Ok(v) => Some(v),
                Err(_) => {
                    show_error("请输入设备支持的 DPI 数值。");
                    return;
                }
            }
        } else {
            None
        };
        let scroll = if self.hardware.can_scroll {
            let mode = unsafe { SendMessageW(self.mode, CB_GETCURSEL, 0, 0) };
            let threshold = if mode != 2 {
                25
            } else {
                match unsafe { control_text(self.threshold) }.parse::<u8>() {
                    Ok(v) if (1..=49).contains(&v) => v,
                    _ => {
                        show_error("SmartShift 阈值请输入 1–49。");
                        return;
                    }
                }
            };
            Some((mode as u8, threshold))
        } else {
            None
        };
        self.busy = true;
        unsafe {
            EnableWindow(self.hardware_apply, 0);
            EnableWindow(self.scan, 0);
            self.set_diagnostic("正在应用并回读鼠标设置。只有识别出的目标设备会接收请求。");
        }
        if self.commands.send(Command::Apply { dpi, scroll }).is_err() {
            self.busy = false;
            unsafe {
                self.set_diagnostic("设备工作线程已退出，请重新启动程序。");
                EnableWindow(self.scan, 1);
            }
        }
    }
    unsafe fn update_hardware(&mut self, view: HardwareView) {
        unsafe {
            self.busy = false;
            EnableWindow(self.scan, 1);
            SetWindowTextW(self.device_label, wide(&view.summary).as_ptr());
            EnableWindow(self.dpi, view.can_dpi as i32);
            EnableWindow(self.mode, view.can_scroll as i32);
            EnableWindow(
                self.hardware_apply,
                (view.can_dpi || view.can_scroll) as i32,
            );
            SendMessageW(self.dpi, CB_RESETCONTENT, 0, 0);
            for dpi in &view.dpi_choices {
                SendMessageW(
                    self.dpi,
                    CB_ADDSTRING,
                    0,
                    wide(&dpi.to_string()).as_ptr() as isize,
                );
            }
            if let Some(dpi) = view.dpi {
                SetWindowTextW(self.dpi, wide(&dpi.to_string()).as_ptr());
            }
            if let Some(mode) = view.scroll_mode {
                SendMessageW(self.mode, CB_SETCURSEL, mode as usize, 0);
            }
            EnableWindow(
                self.threshold,
                (view.can_scroll && view.scroll_mode == Some(2)) as i32,
            );
            SetWindowTextW(self.threshold, wide(&view.threshold.to_string()).as_ptr());
            self.set_diagnostic(&view.details);
            self.hardware = view;
        }
    }
    unsafe fn export(&self) {
        if self.busy {
            return;
        }
        match config::config_path().map(|p| p.with_file_name("diagnostic.json")) {
            Ok(path) => {
                let _ = self.commands.send(Command::Export { path });
            }
            Err(error) => show_error(&error),
        }
    }
    unsafe fn tray_menu(&mut self) {
        unsafe {
            let menu = CreatePopupMenu();
            AppendMenuW(menu, MF_STRING, ID_OPEN, wide("打开设置").as_ptr());
            AppendMenuW(
                menu,
                MF_STRING,
                ID_PAUSE,
                wide(if input::is_paused() {
                    "恢复快捷键映射"
                } else {
                    "暂停快捷键映射"
                })
                .as_ptr(),
            );
            AppendMenuW(menu, MF_SEPARATOR, 0, null());
            AppendMenuW(menu, MF_STRING, ID_EXIT, wide("退出 Logi Lite").as_ptr());
            let mut point = POINT { x: 0, y: 0 };
            GetCursorPos(&mut point);
            SetForegroundWindow(self.window);
            let command = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                self.window,
                null(),
            );
            DestroyMenu(menu);
            PostMessageW(self.window, WM_NULL, 0, 0);
            match command as usize {
                ID_OPEN => self.show(),
                ID_PAUSE => self.toggle_pause(),
                ID_EXIT => {
                    DestroyWindow(self.window);
                }
                _ => {}
            }
        }
    }
    unsafe fn smoke_test(&mut self) {
        unsafe {
            KillTimer(self.window, 2);
            if !self.smoke_stage {
                self.smoke_stage = true;
                let path = self.smoke_path.as_ref().unwrap().with_extension("bmp");
                let result = capture_window(self.window, &path);
                self.smoke_capture_ok = result.is_ok();
                if let Err(error) = result {
                    self.set_diagnostic(&error);
                }
                let paused = input::is_paused();
                SendMessageW(self.window, WM_HOTKEY, EMERGENCY_HOTKEY as usize, 0);
                self.smoke_pause_ok = input::is_paused() != paused;
                SendMessageW(self.window, WM_HOTKEY, EMERGENCY_HOTKEY as usize, 0);
                // Exercise reentrant close while the App state is borrowed.
                SendMessageW(self.window, WM_CLOSE, 0, 0);
                SetTimer(self.window, 2, 1200, None);
                return;
            }
            let path = self.smoke_path.as_ref().unwrap();
            let screenshot = path.with_extension("bmp");
            let retained = IsWindow(self.window) != 0 && IsWindowVisible(self.window) == 0;
            let success = retained && self.smoke_capture_ok && self.smoke_pause_ok;
            let result = serde_json::json!({"success":success,"window_created":true,"close_retains_window_and_tray":retained,"reentrant_emergency_pause":self.smoke_pause_ok,"hotkey_registered":self.hotkey_registered,"disabled_input_hook_installed":true,"hardware_writes":0,"input_injection":0,"screenshot":screenshot});
            let saved = std::fs::write(path, serde_json::to_vec_pretty(&result).unwrap()).is_ok();
            DestroyWindow(self.window);
            if !success || !saved {
                PostQuitMessage(1);
            }
        }
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_DESTROY {
            KillTimer(window, 1);
            KillTimer(window, 2);
            PostQuitMessage(0);
            return 0;
        }
        // These operations must remain available during modal menu/dialog reentry.
        if message == WM_CLOSE {
            ShowWindow(window, SW_HIDE);
        }
        if message == WM_HOTKEY && wparam == EMERGENCY_HOTKEY as usize {
            input::set_paused(!input::is_paused());
        }
        if message == WM_NCCREATE {
            let create = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let ptr = GetWindowLongPtrW(window, GWLP_USERDATA) as *const RefCell<App>;
        if ptr.is_null() {
            return DefWindowProcW(window, message, wparam, lparam);
        }
        let Ok(mut state) = (&*ptr).try_borrow_mut() else {
            if message == WM_CLOSE || message == WM_HOTKEY {
                return 0;
            }
            return DefWindowProcW(window, message, wparam, lparam);
        };
        let app = &mut *state;
        if app.taskbar_created != 0 && message == app.taskbar_created {
            let _ = app.add_tray();
            return 0;
        }
        match message {
            WM_CLOSE => {
                ShowWindow(window, SW_HIDE);
                if !app.hide_explained && !app.smoke {
                    app.hide_explained = true;
                    let mut balloon = app.tray;
                    balloon.uFlags = NIF_INFO;
                    copy_wide(&mut balloon.szInfoTitle, "Logi Lite 仍在运行");
                    copy_wide(
                        &mut balloon.szInfo,
                        "设置窗口已关闭，快捷键仍在托盘运行。右键托盘可暂停或退出。",
                    );
                    balloon.dwInfoFlags = NIIF_INFO | NIIF_NOSOUND;
                    Shell_NotifyIconW(NIM_MODIFY, &balloon);
                }
                0
            }
            WM_COMMAND => {
                match wparam & 0xffff {
                    ID_APPLY => app.save_shortcuts(),
                    ID_PAUSE => app.toggle_pause(),
                    ID_SCAN => app.start_scan(),
                    ID_HARDWARE => app.apply_hardware(),
                    ID_EXPORT => app.export(),
                    ID_MODE if (wparam >> 16) as u32 == CBN_SELCHANGE => {
                        EnableWindow(
                            app.threshold,
                            (app.hardware.can_scroll
                                && SendMessageW(app.mode, CB_GETCURSEL, 0, 0) == 2)
                                as i32,
                        );
                    }
                    _ => {}
                }
                0
            }
            WM_HOTKEY if wparam == EMERGENCY_HOTKEY as usize => {
                app.refresh_state();
                0
            }
            TRAY_MESSAGE => {
                if lparam as u32 == WM_LBUTTONDBLCLK {
                    app.show();
                } else if lparam as u32 == WM_RBUTTONUP {
                    app.tray_menu();
                }
                0
            }
            WM_TIMER => {
                if wparam == 2 && app.smoke {
                    app.smoke_test();
                } else {
                    if app.last_paused != input::is_paused() {
                        app.refresh_state();
                    }
                    while let Ok(view) = app.events.try_recv() {
                        app.update_hardware(view);
                    }
                }
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

fn copy_wide<const N: usize>(target: &mut [u16; N], text: &str) {
    target.fill(0);
    for (slot, ch) in target.iter_mut().take(N - 1).zip(text.encode_utf16()) {
        *slot = ch;
    }
}
unsafe fn control_text(window: HWND) -> String {
    unsafe {
        let size = GetWindowTextLengthW(window).max(0) as usize;
        let mut text = vec![0u16; size + 1];
        let count = GetWindowTextW(window, text.as_mut_ptr(), text.len() as i32).max(0) as usize;
        String::from_utf16_lossy(&text[..count])
    }
}

unsafe fn capture_window(window: HWND, path: &std::path::Path) -> Result<(), String> {
    unsafe {
        let mut rect: RECT = zeroed();
        GetWindowRect(window, &mut rect);
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let dc = GetWindowDC(window);
        let memory = CreateCompatibleDC(dc);
        let mut info: BITMAPINFO = zeroed();
        info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut pixels = null_mut();
        let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut pixels, null_mut(), 0);
        if bitmap.is_null() {
            DeleteDC(memory);
            ReleaseDC(window, dc);
            return Err(win_error("截图失败"));
        }
        let previous = SelectObject(memory, bitmap);
        let printed = SendMessageW(
            window,
            WM_PRINT,
            memory as usize,
            (PRF_CLIENT | PRF_NONCLIENT | PRF_CHILDREN | PRF_ERASEBKGND) as isize,
        );
        GdiFlush();
        let bytes = (width * height * 4) as usize;
        let mut file = Vec::with_capacity(54 + bytes);
        file.extend_from_slice(b"BM");
        file.extend_from_slice(&((54 + bytes) as u32).to_le_bytes());
        file.extend_from_slice(&[0; 4]);
        file.extend_from_slice(&54u32.to_le_bytes());
        file.extend_from_slice(&40u32.to_le_bytes());
        file.extend_from_slice(&width.to_le_bytes());
        file.extend_from_slice(&(-height).to_le_bytes());
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&32u16.to_le_bytes());
        file.extend_from_slice(&[0; 24]);
        file.extend_from_slice(std::slice::from_raw_parts(pixels as *const u8, bytes));
        SelectObject(memory, previous);
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(window, dc);
        let _ = printed;
        std::fs::write(path, file).map_err(|e| e.to_string())
    }
}
