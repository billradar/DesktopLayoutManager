#![cfg_attr(not(windows), allow(dead_code))]

#[cfg(windows)]
mod app {
    use std::{ffi::{c_char, CStr, CString}, mem::zeroed, ptr::null_mut};
    use windows_sys::Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        UI::Controls::Dialogs::{GetOpenFileNameW, GetSaveFileNameW, OPENFILENAMEW, OFN_FILEMUSTEXIST, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST},
        Graphics::Gdi::{GetStockObject, DEFAULT_GUI_FONT},
        System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
        UI::WindowsAndMessaging::*,
    };

    const ID_REFRESH: usize = 101;
    const ID_PREVIEW: usize = 102;
    const ID_EXECUTE: usize = 103;
    const ID_SAVE: usize = 104;
    const ID_OPEN: usize = 105;
    const ID_RESTORE: usize = 106;
    const ID_EXPORT: usize = 107;
    const ID_UNDO: usize = 108;
    const ID_OUTPUT: usize = 201;
    static mut OUTPUT: HWND = 0 as HWND;
    static mut CORE: Option<CoreApi> = None;
    static mut LAST_PREVIEW: Option<(String, String)> = None;
    static mut LOADED_SNAPSHOT: Option<String> = None;
    static mut LAST_BACKUP: Option<String> = None;

    type CallFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;
    type FreeFn = unsafe extern "C" fn(*mut c_char);
    #[derive(Clone, Copy)]
    struct CoreApi { call: CallFn, free: FreeFn }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn load_core() -> Result<CoreApi, String> {
        let dll_name = wide("desktop_layout_manager.dll");
        let module = LoadLibraryW(dll_name.as_ptr());
        if module.is_null() {
            return Err("无法加载 desktop_layout_manager.dll。请确保它与 EXE 位于同一目录。".into());
        }
        let call = GetProcAddress(module, b"dlm_call\0".as_ptr());
        let free = GetProcAddress(module, b"dlm_free_string\0".as_ptr());
        if call.is_none() || free.is_none() {
            return Err("Rust 核心 DLL 缺少所需的导出函数。".into());
        }
        Ok(CoreApi {
            call: std::mem::transmute(call.unwrap()),
            free: std::mem::transmute(free.unwrap()),
        })
    }

    unsafe fn request(action: &str, extra: Option<&str>) -> Result<serde_json::Value, String> {
        let api = CORE.ok_or("Rust 核心尚未初始化。")?;
        let json = match extra {
            Some(extra) => format!(r#"{{"action":"{}",{}}}"#, action, extra.trim_start_matches('{').trim_end_matches('}')),
            None => format!(r#"{{"action":"{}"}}"#, action),
        };
        let input = CString::new(json).map_err(|e| e.to_string())?;
        let output = (api.call)(input.as_ptr());
        if output.is_null() { return Err("Rust 核心没有返回结果。".into()); }
        let response = CStr::from_ptr(output).to_string_lossy().into_owned();
        (api.free)(output);
        let value: serde_json::Value = serde_json::from_str(&response).map_err(|e| e.to_string())?;
        if value["ok"] != true {
            return Err(value["error"].as_str().unwrap_or("桌面操作失败。").to_string());
        }
        Ok(value["data"].clone())
    }

    unsafe fn set_output(text: &str) {
        if OUTPUT.is_null() { return; }
        let text = wide(text);
        SetWindowTextW(OUTPUT, text.as_ptr());
    }

    unsafe fn render_snapshot(data: &serde_json::Value) -> String {
        let snapshot = &data["snapshot"];
        let icons = snapshot["Icons"].as_array().or_else(|| snapshot["icons"].as_array());
        let Some(icons) = icons else { return serde_json::to_string_pretty(data).unwrap_or_default(); };
        let mut out = format!("桌面布局管理器 · 原生 Win32\r\n项目数量：{}\r\n\r\n{:<5} {:<34} {:>7} {:>7}  类型\r\n{}\r\n",
            icons.len(), "序号", "名称", "X", "Y", "-".repeat(76));
        for (i, item) in icons.iter().enumerate() {
            let name = item["Name"].as_str().or(item["name"].as_str()).unwrap_or("");
            let x = item["X"].as_i64().or(item["x"].as_i64()).unwrap_or_default();
            let y = item["Y"].as_i64().or(item["y"].as_i64()).unwrap_or_default();
            let kind = item["Kind"].as_str().or(item["kind"].as_str()).unwrap_or("unknown");
            out.push_str(&format!("{:<5} {:<34} {:>7} {:>7}  {}\r\n", i + 1, name.chars().take(34).collect::<String>(), x, y, kind));
        }
        out
    }


    unsafe fn pick_path(save: bool, csv: bool) -> Option<String> {
        let mut buffer = vec![0u16; 32768];
        let filter = if csv {
            wide("CSV 文件 (*.csv)\0*.csv\0所有文件 (*.*)\0*.*\0")
        } else {
            wide("JSON 布局快照 (*.json)\0*.json\0所有文件 (*.*)\0*.*\0")
        };
        let title = wide(if save { "保存桌面布局" } else { "打开桌面布局" });
        let default_ext = wide(if csv { "csv" } else { "json" });
        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = 0 as HWND;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = buffer.as_mut_ptr();
        ofn.nMaxFile = buffer.len() as u32;
        ofn.lpstrTitle = title.as_ptr();
        ofn.lpstrDefExt = default_ext.as_ptr();
        ofn.Flags = OFN_PATHMUSTEXIST | if save { OFN_OVERWRITEPROMPT } else { OFN_FILEMUSTEXIST };
        let accepted = if save { GetSaveFileNameW(&mut ofn) } else { GetOpenFileNameW(&mut ofn) };
        if accepted == 0 { return None; }
        let len = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..len]))
    }

    fn json_prop(key: &str, value: &serde_json::Value) -> String {
        format!("\\"{}\\":{}", key, value)
    }

    unsafe fn do_action(id: usize) {
        let result = match id {
            ID_REFRESH => request("read", None).map(|data| {
                LAST_PREVIEW = None;
                render_snapshot(&data)
            }),
            ID_PREVIEW => request("preview_fill", None).map(|data| {
                let snapshot = serde_json::to_string(&data["snapshot"]).unwrap_or_default();
                let moves = serde_json::to_string(&data["moves"]).unwrap_or_else(|_| "[]".into());
                LAST_PREVIEW = Some((snapshot, moves));
                let count = data["moves"].as_array().map_or(0, Vec::len);
                format!("补位预览：将移动 {count} 个项目。\r\n确认前会重新检查桌面状态。\r\n\r\n{}", serde_json::to_string_pretty(&data["moves"]).unwrap_or_default())
            }),
            ID_EXECUTE => {
                if let Some((snapshot, moves)) = LAST_PREVIEW.clone() {
                    let count = serde_json::from_str::<serde_json::Value>(&moves)
                        .ok().and_then(|v| v.as_array().map(Vec::len)).unwrap_or(0);
                    let prompt = wide(&format!("即将移动 {count} 个桌面快捷方式。执行前会自动备份；确认执行？"));
                    if MessageBoxW(0 as HWND, prompt.as_ptr(), wide("确认执行补位").as_ptr(), MB_ICONWARNING | MB_YESNO | MB_DEFBUTTON2) != IDYES {
                        set_output("已取消操作；桌面未移动。");
                        return;
                    }
                    let extra = format!(r#""expectedSnapshot":{},"expectedMoves":{}"#, snapshot, moves);
                    request("execute_fill", Some(&extra)).map(|data| format!("执行完成。\r\n自动备份：{}\r\n移动项目：{}", data["backupPath"].as_str().unwrap_or("(无)"), data["moves"].as_array().map_or(0, Vec::len)))
                } else { Err("请先点击“预览补位”，确认移动计划后再执行。".into()) }
            }
            ID_SAVE => {
                if let Some(path) = pick_path(true, false) {
                    let extra = format!("\\"path\\":{}", serde_json::to_string(&path).unwrap_or_else(|_| "\\"\\"".into()));
                    request("save_snapshot", Some(&extra)).map(|data| {
                        LOADED_SNAPSHOT = serde_json::to_string(&data["snapshot"]).ok();
                        format!("快照已保存：{}\r\n项目数：{}", data["path"].as_str().unwrap_or(&path), data["snapshot"]["Icons"].as_array().map_or(0, Vec::len))
                    })
                } else { Ok("已取消保存。".into()) }
            },
            ID_OPEN => {
                if let Some(path) = pick_path(false, false) {
                    let extra = format!("\\"path\\":{}", serde_json::to_string(&path).unwrap_or_else(|_| "\\"\\"".into()));
                    request("load_snapshot", Some(&extra)).map(|data| {
                        LOADED_SNAPSHOT = serde_json::to_string(&data["snapshot"]).ok();
                        format!("已打开快照：{}\r\n\r\n{}", data["path"].as_str().unwrap_or(&path), render_snapshot(&data))
                    })
                } else { Ok("已取消打开。".into()) }
            },
            ID_RESTORE => {
                if let Some(saved) = LOADED_SNAPSHOT.clone() {
                    let preview_extra = format!("\\"savedSnapshot\\":{}", saved);
                    request("preview_restore", Some(&preview_extra)).and_then(|plan| {
                        let moves = plan["moves"].as_array().map_or(0, Vec::len);
                        if moves == 0 { return Ok(format!("无需恢复。跳过项目：{}", serde_json::to_string_pretty(&plan["skipped"]).unwrap_or_default())); }
                        let prompt = wide(&format!("将恢复 {moves} 个项目的位置。继续？"));
                        if MessageBoxW(0 as HWND, prompt.as_ptr(), wide("确认恢复布局").as_ptr(), MB_ICONWARNING | MB_YESNO | MB_DEFBUTTON2) != IDYES {
                            return Ok("已取消恢复；桌面未移动。".into());
                        }
                        let extra = format!("\\"savedSnapshot\\":{},\\"expectedSnapshot\\":{},\\"expectedMoves\\":{},\\"expectedSkipped\\":{}",
                            saved, plan["snapshot"], plan["moves"], plan["skipped"]);
                        request("execute_restore", Some(&extra)).map(|data| {
                            LAST_BACKUP = data["backupPath"].as_str().map(str::to_owned);
                            format!("恢复完成。\r\n自动备份：{}\r\n恢复项目：{}", data["backupPath"].as_str().unwrap_or("(无)"), data["moves"].as_array().map_or(0, Vec::len))
                        })
                    })
                } else { Err("请先点击“打开快照”，选择要恢复的布局文件。".into()) }
            },
            ID_EXPORT => {
                if let Some(path) = pick_path(true, true) {
                    let extra = format!("\\"path\\":{}", serde_json::to_string(&path).unwrap_or_else(|_| "\\"\\"".into()));
                    request("export", Some(&extra)).map(|data| format!("已导出 {} 个项目：{}", data["count"].as_u64().unwrap_or(0), data["path"].as_str().unwrap_or(&path)))
                } else { Ok("已取消导出。".into()) }
            },
            ID_UNDO => {
                if let Some(path) = LAST_BACKUP.clone() {
                    let extra = format!("\\"path\\":{}", serde_json::to_string(&path).unwrap_or_else(|_| "\\"\\"".into()));
                    request("preview_undo", Some(&extra)).and_then(|plan| {
                        let moves = plan["moves"].as_array().map_or(0, Vec::len);
                        if moves == 0 { return Ok("没有可撤销的移动。".into()); }
                        let prompt = wide(&format!("将撤销最近一次操作并移动 {moves} 个快捷方式。继续？"));
                        if MessageBoxW(0 as HWND, prompt.as_ptr(), wide("确认撤销").as_ptr(), MB_ICONWARNING | MB_YESNO | MB_DEFBUTTON2) != IDYES {
                            return Ok("已取消撤销；桌面未移动。".into());
                        }
                        let extra = format!("\\"savedSnapshot\\":{},\\"expectedSnapshot\\":{},\\"expectedMoves\\":{},\\"expectedSkipped\\":{}",
                            plan["savedSnapshot"], plan["snapshot"], plan["moves"], plan["skipped"]);
                        request("execute_undo", Some(&extra)).map(|data| format!("撤销完成。\r\n恢复项目：{}", data["moves"].as_array().map_or(0, Vec::len)))
                    })
                } else { Err("当前会话还没有可撤销的操作。".into()) }
            },
            _ => return,
        };
        match result { Ok(text) => set_output(&text), Err(error) => set_output(&format!("操作失败：{error}")) }
    }

    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_CREATE => {
                let buttons = [
                    (ID_REFRESH, "读取桌面"),
                    (ID_PREVIEW, "预览补位"),
                    (ID_EXECUTE, "执行补位"),
                    (ID_SAVE, "保存快照"),
                    (ID_OPEN, "打开快照"),
                    (ID_RESTORE, "恢复布局"),
                    (ID_EXPORT, "导出"),
                    (ID_UNDO, "撤销上次操作"),
                ];
                for (i, (id, label)) in buttons.iter().enumerate() {
                    let title = wide(label);
                    CreateWindowExW(0, wide("BUTTON").as_ptr(), title.as_ptr(),
                        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON, 12 + (i as i32) * 122, 12, 112, 30,
                        hwnd, *id as _, GetModuleHandleW(null_mut()), null_mut());
                }
                OUTPUT = CreateWindowExW(WS_EX_CLIENTEDGE, wide("EDIT").as_ptr(), wide("正在初始化原生 Win32 界面……").as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_VSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY,
                    12, 54, 1110, 650, hwnd, ID_OUTPUT as _, GetModuleHandleW(null_mut()), null_mut());
                let font = GetStockObject(DEFAULT_GUI_FONT);
                SendMessageW(OUTPUT, WM_SETFONT, font as usize, 1);
                match load_core() {
                    Ok(api) => { CORE = Some(api); do_action(ID_REFRESH); }
                    Err(error) => set_output(&error),
                }
                0
            }
            WM_COMMAND => { let id = (wparam & 0xffff) as usize; do_action(id); 0 }
            WM_SIZE => {
                if !OUTPUT.is_null() {
                    let width = (lparam as u32 & 0xffff) as i32;
                    let height = ((lparam as u32 >> 16) & 0xffff) as i32;
                    MoveWindow(OUTPUT, 12, 54, (width - 24).max(300), (height - 66).max(200), 1);
                }
                0
            }
            WM_DESTROY => { PostQuitMessage(0); 0 }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    pub fn run() {
        unsafe {
            let instance: HINSTANCE = GetModuleHandleW(null_mut());
            let class_name = wide("DesktopLayoutManagerNativeWin32");
            let mut wc: WNDCLASSW = zeroed();
            wc.style = CS_HREDRAW | CS_VREDRAW;
            wc.lpfnWndProc = Some(wnd_proc);
            wc.hInstance = instance;
            wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
            wc.hbrBackground = (COLOR_WINDOW + 1) as _;
            wc.lpszClassName = class_name.as_ptr();
            if RegisterClassW(&wc) == 0 {
                MessageBoxW(0 as HWND, wide("无法注册 Win32 窗口类。").as_ptr(), wide("桌面布局管理器").as_ptr(), MB_ICONERROR);
                return;
            }
            let hwnd = CreateWindowExW(0, class_name.as_ptr(), wide("桌面布局管理器 — 原生 Win32").as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE, CW_USEDEFAULT, CW_USEDEFAULT, 1160, 760,
                0 as HWND, 0 as _, instance, null_mut());
            if hwnd.is_null() { return; }
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
            let mut msg: MSG = zeroed();
            while GetMessageW(&mut msg, 0 as HWND, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

#[cfg(windows)]
fn main() { app::run(); }

#[cfg(not(windows))]
fn main() { eprintln!("DesktopLayoutManager is a Windows-only Win32 application."); }
