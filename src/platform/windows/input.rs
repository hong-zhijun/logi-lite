//! Opt-in global mouse-button fallback. It deliberately does not identify devices
//! or send any HID/HID++ commands. Install and uninstall on the UI message thread.
use crate::config::{Action, Chord, Config, parse_action, validate};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
};
use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

static PAUSED: AtomicBool = AtomicBool::new(false);
static MAPPED: AtomicU64 = AtomicU64::new(0);
static FAILURES: AtomicU64 = AtomicU64::new(0);
static PRESSED: [ButtonLatch; 3] = [const { ButtonLatch(AtomicU8::new(0)) }; 3];
static SIDE_HELD: [AtomicBool; 2] = [const { AtomicBool::new(false) }; 2];
static STATE: Mutex<State> = Mutex::new(State {
    hook: 0,
    enabled: false,
    actions: [Action::Default, Action::Default, Action::Default],
    horizontal: [false, false],
    pending_release: Vec::new(),
});

struct State {
    hook: usize,
    enabled: bool,
    actions: [Action; 3],
    horizontal: [bool; 2],
    pending_release: Vec<u16>,
}

// 0 = released, 1 = physical DOWN passed through, 2 = DOWN consumed.
// Keeping this outside the settings mutex preserves paired UP even if settings
// are being changed concurrently. A fail-open DOWN is latched as passthrough.
struct ButtonLatch(AtomicU8);
impl ButtonLatch {
    fn begin(&self) -> Option<bool> {
        match self
            .0
            .compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => None,
            Err(value) => Some(value == 2),
        }
    }
    fn consume(&self) {
        self.0.store(2, Ordering::Relaxed);
    }
    fn finish(&self) -> bool {
        self.0.swap(0, Ordering::Relaxed) == 2
    }
}

pub fn set_config(config: &Config) -> Result<(), String> {
    validate(config)?;
    let actions = [
        parse_action(&config.back)?,
        parse_action(&config.forward)?,
        parse_action(&config.middle)?,
    ];
    let mut state = STATE.lock().map_err(|_| "输入引擎状态不可用")?;
    state.enabled = config.enabled;
    state.actions = actions;
    state.horizontal = [
        config.horizontal_scroll.back,
        config.horizontal_scroll.forward,
    ];
    // Paired UP decisions remain unchanged while the user is holding a button.
    Ok(())
}

pub fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::Relaxed);
}
pub fn is_paused() -> bool {
    PAUSED.load(Ordering::Relaxed)
}

pub fn stats() -> String {
    format!(
        "已处理 {} 次按键；发送失败 {} 次；{}",
        MAPPED.load(Ordering::Relaxed),
        FAILURES.load(Ordering::Relaxed),
        if is_paused() { "已暂停" } else { "就绪" }
    )
}

/// Trigger one configured shortcut from a non-mouse input source such as the
/// HID++ top wheel-mode button. Default and disabled actions intentionally do
/// nothing; the caller owns any device diversion state.
pub fn trigger_action(text: &str) -> Result<(), String> {
    let action = parse_action(text)?;
    if let Action::Shortcut(chord) = action {
        let mut pending = Vec::new();
        if send_chord(&chord, &mut pending) {
            Ok(())
        } else {
            Err("快捷键发送失败".into())
        }
    } else {
        Ok(())
    }
}

pub fn install() -> Result<(), String> {
    let mut state = STATE.lock().map_err(|_| "输入引擎状态不可用")?;
    if state.hook != 0 {
        return Ok(());
    }
    let hook = unsafe {
        SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_hook),
            GetModuleHandleW(std::ptr::null()),
            0,
        )
    };
    if hook.is_null() {
        return Err(format!(
            "安装鼠标监听失败：{}",
            std::io::Error::last_os_error()
        ));
    }
    state.hook = hook as usize;
    for latch in &PRESSED {
        latch.finish();
    }
    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    let mut state = STATE.lock().map_err(|_| "输入引擎状态不可用")?;
    retry_release(&mut state.pending_release);
    if state.hook != 0 {
        if unsafe { UnhookWindowsHookEx(state.hook as HHOOK) } == 0 {
            return Err(format!(
                "移除鼠标监听失败：{}",
                std::io::Error::last_os_error()
            ));
        }
        state.hook = 0;
    }
    for latch in &PRESSED {
        latch.finish();
    }
    if !state.pending_release.is_empty() {
        return Err("系统拒绝释放模拟按键；请按一下并松开 Ctrl/Alt/Shift/Win，再退出".into());
    }
    Ok(())
}

fn button_event(message: u32, data: u32) -> Option<(usize, bool)> {
    match message {
        WM_MBUTTONDOWN => Some((2, true)),
        WM_MBUTTONUP => Some((2, false)),
        WM_XBUTTONDOWN | WM_XBUTTONUP => {
            let index = match data >> 16 {
                1 => 0,
                2 => 1,
                _ => return None,
            };
            Some((index, message == WM_XBUTTONDOWN))
        }
        _ => None,
    }
}

fn horizontal_event(data: u32) -> Option<i32> {
    (data != 0).then_some((data as i16) as i32)
}

fn send_horizontal(delta: i32) -> bool {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: delta as u32,
                dwFlags: MOUSEEVENTF_HWHEEL,
                time: 0,
                dwExtraInfo: 0x4C4F4749,
            },
        },
    };
    unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) == 1 }
}

unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && lparam != 0 {
        // Windows guarantees a valid MSLLHOOKSTRUCT for a nonnegative hook code.
        let event = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        if event.flags & LLMHF_INJECTED == 0
            && let Some((index, down)) = button_event(wparam as u32, event.mouseData)
        {
            if index < 2 {
                SIDE_HELD[index].store(down, Ordering::Relaxed);
            }
            if !down {
                if PRESSED[index].finish() {
                    return 1;
                }
                return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
            }
            if let Some(consumed) = PRESSED[index].begin() {
                return if consumed {
                    1
                } else {
                    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
                };
            }
            // Fail open if another thread is changing settings. No I/O, waiting,
            // logging, or UI work takes place inside the low-level callback.
            if let Ok(mut state) = STATE.try_lock() {
                retry_release(&mut state.pending_release);
                let action = if state.enabled && !is_paused() && state.pending_release.is_empty() {
                    state.actions[index].clone()
                } else {
                    Action::Default
                };
                let consumed = match action {
                    Action::Default => false,
                    Action::Disabled => true,
                    Action::Shortcut(ref chord) => {
                        let result = send_chord(chord, &mut state.pending_release);
                        if !result {
                            FAILURES.fetch_add(1, Ordering::Relaxed);
                        }
                        result
                    }
                };
                if consumed {
                    PRESSED[index].consume();
                    MAPPED.fetch_add(1, Ordering::Relaxed);
                    return 1;
                }
            }
        } else if event.flags & LLMHF_INJECTED == 0
            && wparam as u32 == WM_MOUSEWHEEL
            && let Some(delta) = horizontal_event(event.mouseData)
            && let Ok(state) = STATE.try_lock()
            && state.enabled
            && !is_paused()
            && ((state.horizontal[0] && SIDE_HELD[0].load(Ordering::Relaxed))
                || (state.horizontal[1] && SIDE_HELD[1].load(Ordering::Relaxed)))
            && send_horizontal(delta)
        {
            return 1;
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

fn extended(key: u16) -> bool {
    matches!(key, 0x21..=0x28 | 0x2E | 0x5B | 0x5C)
}

fn keyboard_input(key: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }
                    | if extended(key) {
                        KEYEVENTF_EXTENDEDKEY
                    } else {
                        0
                    },
                time: 0,
                dwExtraInfo: 0x4C4F4749,
            },
        },
    }
}

// Do not synthesize a release for modifiers the user is already holding.
fn modifier_held(key: u16) -> bool {
    let alternatives: &[i32] = match key {
        0x10 => &[0xA0, 0xA1],
        0x11 => &[0xA2, 0xA3],
        0x12 => &[0xA4, 0xA5],
        0x5B => &[0x5B, 0x5C],
        _ => &[],
    };
    alternatives
        .iter()
        .any(|key| unsafe { GetAsyncKeyState(*key) } < 0)
}

fn chord_plan(chord: &Chord, held: impl Fn(u16) -> bool) -> Vec<(u16, bool)> {
    let owned: Vec<_> = chord
        .modifiers
        .iter()
        .copied()
        .filter(|key| !held(*key))
        .collect();
    let mut plan: Vec<_> = owned.iter().map(|key| (*key, false)).collect();
    plan.push((chord.key, false));
    plan.push((chord.key, true));
    plan.extend(owned.iter().rev().map(|key| (*key, true)));
    plan
}

fn outstanding(plan: &[(u16, bool)], sent: usize) -> Vec<u16> {
    let mut pressed = Vec::new();
    for &(key, up) in plan.iter().take(sent) {
        if up {
            pressed.retain(|value| *value != key);
        } else {
            pressed.push(key);
        }
    }
    pressed.reverse();
    pressed
}

fn send_chord(chord: &Chord, pending: &mut Vec<u16>) -> bool {
    // A held target key belongs to the user, so leave both it and the mouse alone.
    if unsafe { GetAsyncKeyState(chord.key as i32) } < 0 {
        return false;
    }
    let plan = chord_plan(chord, modifier_held);
    let inputs: Vec<_> = plan
        .iter()
        .map(|&(key, up)| keyboard_input(key, up))
        .collect();
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    } as usize;
    if sent == inputs.len() {
        return true;
    }
    *pending = outstanding(&plan, sent);
    retry_release(pending);
    false
}

fn retry_release(pending: &mut Vec<u16>) {
    // A bounded rollback also covers a rare partially accepted SendInput array.
    for _ in 0..3 {
        if pending.is_empty() {
            break;
        }
        let releases: Vec<_> = pending
            .iter()
            .map(|key| keyboard_input(*key, true))
            .collect();
        let count = unsafe {
            SendInput(
                releases.len() as u32,
                releases.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            )
        } as usize;
        pending.drain(..count.min(pending.len()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_modifiers_are_never_released_by_plan() {
        let chord = Chord {
            modifiers: vec![0x11, 0x10, 0x5B],
            key: b'C' as u16,
        };
        let plan = chord_plan(&chord, |key| key == 0x11 || key == 0x5B);
        assert_eq!(
            plan,
            vec![(0x10, false), (0x43, false), (0x43, true), (0x10, true)]
        );
    }
    #[test]
    fn partial_send_releases_only_keys_actually_injected() {
        let plan = chord_plan(
            &Chord {
                modifiers: vec![0x11, 0x10],
                key: 0x43,
            },
            |_| false,
        );
        assert!(outstanding(&plan, 0).is_empty());
        assert_eq!(outstanding(&plan, 1), vec![0x11]);
        assert_eq!(outstanding(&plan, 3), vec![0x43, 0x10, 0x11]);
        assert_eq!(outstanding(&plan, 4), vec![0x10, 0x11]);
        assert!(outstanding(&plan, plan.len()).is_empty());
    }
    #[test]
    fn paired_up_latch_survives_pause_or_profile_change() {
        let consumed = ButtonLatch(AtomicU8::new(0));
        assert_eq!(consumed.begin(), None);
        consumed.consume();
        // Profile/pause only affect selecting an action for a new DOWN; repeats
        // and the matching UP use the independent latch.
        assert_eq!(consumed.begin(), Some(true));
        assert!(consumed.finish());
        assert!(!consumed.finish());
        let passthrough = ButtonLatch(AtomicU8::new(0));
        assert_eq!(passthrough.begin(), None);
        assert_eq!(passthrough.begin(), Some(false));
        assert!(!passthrough.finish());
    }
    #[test]
    fn only_supported_physical_buttons_are_recognized() {
        assert_eq!(button_event(WM_XBUTTONDOWN, 1 << 16), Some((0, true)));
        assert_eq!(button_event(WM_XBUTTONUP, 2 << 16), Some((1, false)));
        assert_eq!(button_event(WM_MBUTTONUP, 0), Some((2, false)));
        assert_eq!(button_event(WM_LBUTTONDOWN, 0), None);
        assert_eq!(button_event(WM_XBUTTONDOWN, 3 << 16), None);
    }
}
