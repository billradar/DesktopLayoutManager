#![cfg_attr(not(windows), allow(dead_code))]
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_SNAPSHOT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SNAPSHOT_ICONS: usize = 100_000;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Snapshot {
    schema_version: u32,
    product: String,
    #[serde(default)]
    captured_at_unix_ms: u128,
    machine: String,
    #[serde(alias = "WindowsVersion")]
    os: String,
    screen_width: i32,
    screen_height: i32,
    icons: Vec<Icon>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Icon {
    original_index: i32,
    name: String,
    x: i32,
    y: i32,
    #[serde(default)]
    kind: ItemKind,
    #[serde(default, rename = "IsFileOrFolder", skip_serializing)]
    legacy_is_file_or_folder: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ItemKind {
    Shortcut,
    File,
    Folder,
    System,
    #[default]
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Move {
    index: i32,
    name: String,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
fn local_app_data() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}
fn default_dir() -> PathBuf {
    local_app_data().join("DesktopLayoutManager")
}
fn default_backup_dir() -> PathBuf {
    default_dir().join("Backups")
}
fn timestamp() -> String {
    format!("{}", now_ms())
}
fn save_snapshot(path: &Path, snapshot: &Snapshot) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed),
    ));
    let bytes = serde_json::to_vec_pretty(snapshot).map_err(io::Error::other)?;
    let write_result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    write_result?;
    Ok(())
}
fn load_snapshot(path: &Path) -> io::Result<Snapshot> {
    let size = fs::metadata(path)?.len();
    if size > MAX_SNAPSHOT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "布局文件超过 64 MiB 安全上限。",
        ));
    }
    let bytes = fs::read(path)?;
    let mut snapshot: Snapshot = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if snapshot.schema_version != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("不支持的布局版本：{}。", snapshot.schema_version),
        ));
    }
    if snapshot.icons.len() > MAX_SNAPSHOT_ICONS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "布局文件中的图标数量超过安全上限。",
        ));
    }
    for icon in &mut snapshot.icons {
        if icon.kind == ItemKind::Unknown && icon.legacy_is_file_or_folder {
            icon.kind = ItemKind::File;
        }
    }
    Ok(snapshot)
}
fn shell_system_name(name: &str) -> bool {
    let n = name.trim().to_lowercase();
    [
        "this pc",
        "computer",
        "此电脑",
        "my computer",
        "network",
        "网络",
        "recycle bin",
        "回收站",
        "control panel",
        "控制面板",
        "user's files",
        "用户的文件",
        "homegroup",
        "家庭组",
    ]
    .iter()
    .any(|s| n == *s)
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::{
        ffi::c_void,
        mem::{size_of, zeroed},
        ptr::{null, null_mut},
    };
    type Hwnd = *mut c_void;
    type Handle = *mut c_void;
    type Lparam = isize;
    type Wparam = usize;
    type Lresult = isize;
    const PROCESS_VM_OPERATION: u32 = 0x0008;
    const PROCESS_VM_READ: u32 = 0x0010;
    const PROCESS_VM_WRITE: u32 = 0x0020;
    const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
    const MEM_COMMIT: u32 = 0x1000;
    const MEM_RESERVE: u32 = 0x2000;
    const MEM_RELEASE: u32 = 0x8000;
    const PAGE_READWRITE: u32 = 0x04;
    const LVM_FIRST: u32 = 0x1000;
    const LVM_GETITEMCOUNT: u32 = LVM_FIRST + 4;
    const LVM_SETITEMPOSITION: u32 = LVM_FIRST + 15;
    const LVM_GETITEMPOSITION: u32 = LVM_FIRST + 16;
    const LVM_GETITEMTEXTW: u32 = LVM_FIRST + 115;
    const GWL_STYLE: i32 = -16;
    const LVS_AUTOARRANGE: isize = 0x0100;
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Point {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct LvItemW {
        mask: u32,
        i_item: i32,
        i_sub_item: i32,
        state: u32,
        state_mask: u32,
        psz_text: *mut u16,
        cch_text_max: i32,
        i_image: i32,
        l_param: isize,
        i_indent: i32,
        i_group_id: i32,
        c_columns: u32,
        pu_columns: *mut u32,
        pi_col_fmt: *mut i32,
        i_group: i32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class: *const u16, name: *const u16) -> Hwnd;
        fn FindWindowExW(parent: Hwnd, after: Hwnd, class: *const u16, name: *const u16) -> Hwnd;
        fn EnumWindows(
            cb: Option<unsafe extern "system" fn(Hwnd, Lparam) -> i32>,
            param: Lparam,
        ) -> i32;
        fn GetWindowThreadProcessId(hwnd: Hwnd, pid: *mut u32) -> u32;
        fn SendMessageW(hwnd: Hwnd, msg: u32, w: Wparam, l: Lparam) -> Lresult;
        fn GetClientRect(hwnd: Hwnd, r: *mut Rect) -> i32;
        fn GetWindowLongPtrW(hwnd: Hwnd, index: i32) -> isize;
        fn IsWindow(hwnd: Hwnd) -> i32;
        fn GetSystemMetrics(index: i32) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
        fn VirtualAllocEx(
            process: Handle,
            address: *mut c_void,
            size: usize,
            kind: u32,
            protect: u32,
        ) -> *mut c_void;
        fn VirtualFreeEx(process: Handle, address: *mut c_void, size: usize, kind: u32) -> i32;
        fn ReadProcessMemory(
            process: Handle,
            address: *const c_void,
            buffer: *mut c_void,
            size: usize,
            read: *mut usize,
        ) -> i32;
        fn WriteProcessMemory(
            process: Handle,
            address: *mut c_void,
            buffer: *const c_void,
            size: usize,
            written: *mut usize,
        ) -> i32;
    }
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    struct Desktop {
        hwnd: Hwnd,
        process: Handle,
        pid: u32,
    }
    impl Drop for Desktop {
        fn drop(&mut self) {
            unsafe {
                if !self.process.is_null() {
                    CloseHandle(self.process);
                }
            }
        }
    }
    unsafe extern "system" fn enum_top(top: Hwnd, param: Lparam) -> i32 {
        let found = param as *mut Hwnd;
        let view = FindWindowExW(top, null_mut(), wide("SHELLDLL_DefView").as_ptr(), null());
        if !view.is_null() {
            let list = FindWindowExW(
                view,
                null_mut(),
                wide("SysListView32").as_ptr(),
                wide("FolderView").as_ptr(),
            );
            if !list.is_null() {
                *found = list;
                return 0;
            }
        }
        1
    }
    fn connect() -> io::Result<Desktop> {
        unsafe {
            let mut list = FindWindowExW(
                FindWindowW(wide("Progman").as_ptr(), null()),
                null_mut(),
                wide("SHELLDLL_DefView").as_ptr(),
                null(),
            );
            if !list.is_null() {
                list = FindWindowExW(
                    list,
                    null_mut(),
                    wide("SysListView32").as_ptr(),
                    wide("FolderView").as_ptr(),
                );
            }
            if list.is_null() {
                let mut found: Hwnd = std::ptr::null_mut();
                let p = &mut found as *mut Hwnd as Lparam;
                EnumWindows(Some(enum_top), p);
                list = found;
            }
            if list.is_null() {
                return Err(io::Error::other(
                    "找不到 Windows 桌面 ListView。请确认 Explorer 桌面已加载。",
                ));
            }
            let mut pid = 0;
            GetWindowThreadProcessId(list, &mut pid);
            if pid == 0 {
                return Err(io::Error::other("无法读取 Explorer 进程标识。"));
            }
            let process = OpenProcess(
                PROCESS_VM_OPERATION
                    | PROCESS_VM_READ
                    | PROCESS_VM_WRITE
                    | PROCESS_QUERY_INFORMATION,
                0,
                pid,
            );
            if process.is_null() {
                return Err(io::Error::other(
                    "无法打开 Explorer 进程。请确保本程序与 Explorer 的权限级别一致。",
                ));
            }
            Ok(Desktop {
                hwnd: list,
                process,
                pid,
            })
        }
    }
    struct RemoteBuffer<'a> {
        desktop: &'a Desktop,
        address: *mut c_void,
    }
    impl<'a> RemoteBuffer<'a> {
        fn new(desktop: &'a Desktop, size: usize) -> io::Result<Self> {
            Ok(Self {
                desktop,
                address: desktop.alloc(size)?,
            })
        }
        fn as_ptr(&self) -> *mut c_void {
            self.address
        }
    }
    impl Drop for RemoteBuffer<'_> {
        fn drop(&mut self) {
            self.desktop.remote_free(self.address);
        }
    }
    impl Desktop {
        fn alloc(&self, size: usize) -> io::Result<*mut c_void> {
            unsafe {
                let p = VirtualAllocEx(
                    self.process,
                    null_mut(),
                    size,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_READWRITE,
                );
                if p.is_null() {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(p)
                }
            }
        }
        fn write<T>(&self, remote: *mut c_void, value: &T) -> io::Result<()> {
            unsafe {
                let mut n = 0;
                if WriteProcessMemory(
                    self.process,
                    remote,
                    value as *const T as *const c_void,
                    size_of::<T>(),
                    &mut n,
                ) == 0
                    || n != size_of::<T>()
                {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(())
                }
            }
        }
        fn read<T: Copy>(&self, remote: *mut c_void) -> io::Result<T> {
            unsafe {
                let mut out: T = zeroed();
                let mut n = 0;
                if ReadProcessMemory(
                    self.process,
                    remote,
                    &mut out as *mut T as *mut c_void,
                    size_of::<T>(),
                    &mut n,
                ) == 0
                    || n != size_of::<T>()
                {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(out)
                }
            }
        }
        fn remote_free(&self, p: *mut c_void) {
            unsafe {
                if !p.is_null() {
                    VirtualFreeEx(self.process, p, 0, MEM_RELEASE);
                }
            }
        }
        fn read_icons(&self) -> io::Result<Vec<Icon>> {
            unsafe {
                let mut current_pid = 0u32;
                if IsWindow(self.hwnd) == 0
                    || GetWindowThreadProcessId(self.hwnd, &mut current_pid) == 0
                    || current_pid != self.pid
                {
                    return Err(io::Error::other("桌面视图已重新创建，请重试。"));
                }
                let count = SendMessageW(self.hwnd, LVM_GETITEMCOUNT, 0, 0);
                if count < 0 || count as usize > MAX_SNAPSHOT_ICONS {
                    return Err(io::Error::other("桌面图标数量异常，已停止读取。"));
                }
                let count = count as i32;
                let pm = RemoteBuffer::new(self, size_of::<Point>())?;
                let im = RemoteBuffer::new(self, size_of::<LvItemW>())?;
                let tm = RemoteBuffer::new(self, 2048)?;
                let mut out = Vec::with_capacity(count as usize);
                let result = (|| {
                    for i in 0..count {
                        let p = Point::default();
                        self.write(pm.as_ptr(), &p)?;
                        if SendMessageW(
                            self.hwnd,
                            LVM_GETITEMPOSITION,
                            i as usize,
                            pm.as_ptr() as Lparam,
                        ) == 0
                        {
                            return Err(io::Error::other(format!(
                                "无法读取第 {} 个桌面图标的位置。",
                                i + 1
                            )));
                        }
                        let p: Point = self.read(pm.as_ptr())?;
                        let item = LvItemW {
                            mask: 0,
                            i_item: i,
                            i_sub_item: 0,
                            state: 0,
                            state_mask: 0,
                            psz_text: tm.as_ptr() as *mut u16,
                            cch_text_max: 1024,
                            i_image: 0,
                            l_param: 0,
                            i_indent: 0,
                            i_group_id: 0,
                            c_columns: 0,
                            pu_columns: null_mut(),
                            pi_col_fmt: null_mut(),
                            i_group: 0,
                        };
                        self.write(im.as_ptr(), &item)?;
                        SendMessageW(
                            self.hwnd,
                            LVM_GETITEMTEXTW,
                            i as usize,
                            im.as_ptr() as Lparam,
                        );
                        let mut chars = vec![0u16; 1024];
                        let mut got = 0usize;
                        if ReadProcessMemory(
                            self.process,
                            tm.as_ptr(),
                            chars.as_mut_ptr() as *mut c_void,
                            chars.len() * 2,
                            &mut got,
                        ) == 0
                        {
                            return Err(io::Error::last_os_error());
                        }
                        if got < 2 {
                            return Err(io::Error::other("Explorer 返回了不完整的图标名称数据。"));
                        }
                        let end = chars.iter().position(|c| *c == 0).unwrap_or(chars.len());
                        let name = String::from_utf16_lossy(&chars[..end]);
                        out.push(Icon {
                            original_index: i,
                            name,
                            x: p.x,
                            y: p.y,
                            kind: ItemKind::Unknown,
                            legacy_is_file_or_folder: false,
                        });
                    }
                    Ok(())
                })();
                result?;
                Ok(out)
            }
        }
        fn set_pos(&self, index: i32, x: i32, y: i32) -> io::Result<()> {
            if index < 0 || x < 0 || y < 0 || x > u16::MAX as i32 || y > u16::MAX as i32 {
                return Err(io::Error::other(
                    "目标图标或坐标超出 Win32 ListView 消息范围。",
                ));
            }
            unsafe {
                let mut current_pid = 0u32;
                if IsWindow(self.hwnd) == 0
                    || GetWindowThreadProcessId(self.hwnd, &mut current_pid) == 0
                    || current_pid != self.pid
                {
                    return Err(io::Error::other("桌面视图已失效或 Explorer 已重启。"));
                }
                let packed = ((y as u32 & 0xffff) << 16) | (x as u32 & 0xffff);
                let result = SendMessageW(
                    self.hwnd,
                    LVM_SETITEMPOSITION,
                    index as usize,
                    packed as isize,
                );
                if result == 0 {
                    return Err(io::Error::other(
                        "Explorer 拒绝了图标位置更改。请确认已关闭自动排列，并重试。",
                    ));
                }
                Ok(())
            }
        }
        fn find_icon_index(&self, name: &str, x: i32, y: i32) -> io::Result<i32> {
            let icons = self.read_icons()?;
            let matches: Vec<&Icon> = icons
                .iter()
                .filter(|icon| icon.name.eq_ignore_ascii_case(name) && icon.x == x && icon.y == y)
                .collect();
            match matches.as_slice() {
                [icon] => Ok(icon.original_index),
                [] => Err(io::Error::other(format!(
                    "桌面图标“{name}”已不在预期位置 ({x}, {y})。"
                ))),
                _ => Err(io::Error::other(format!(
                    "有多个桌面图标“{name}”位于预期位置 ({x}, {y})。"
                ))),
            }
        }
        fn auto_arrange(&self) -> bool {
            unsafe { GetWindowLongPtrW(self.hwnd, GWL_STYLE) & LVS_AUTOARRANGE != 0 }
        }
        fn dimensions(&self) -> (i32, i32) {
            unsafe {
                let mut r = Rect::default();
                GetClientRect(self.hwnd, &mut r);
                (r.right - r.left, r.bottom - r.top)
            }
        }
    }
    fn desktop_dirs() -> Vec<PathBuf> {
        let mut v = Vec::new();
        if let Some(p) = env::var_os("USERPROFILE") {
            let root = PathBuf::from(p);
            v.push(root.join("Desktop"));
            v.push(root.join("OneDrive").join("Desktop"));
        }
        if let Some(p) = env::var_os("PUBLIC") {
            v.push(PathBuf::from(p).join("Desktop"));
        }
        v
    }
    fn classify(icons: &mut [Icon]) {
        use std::collections::{HashMap, HashSet};
        let mut map: HashMap<String, Vec<ItemKind>> = HashMap::new();
        let mut seen_paths = HashSet::new();
        for dir in desktop_dirs() {
            let entries = match fs::read_dir(dir) {
                Ok(x) => x,
                Err(_) => continue,
            };
            for e in entries.flatten() {
                let path = e.path();
                let identity = fs::canonicalize(&path)
                    .unwrap_or_else(|_| path.clone())
                    .to_string_lossy()
                    .to_lowercase();
                if !seen_paths.insert(identity) {
                    continue;
                }
                let label = path
                    .file_stem()
                    .or_else(|| path.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let full = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let kind = if path.is_dir() {
                    ItemKind::Folder
                } else if matches!(
                    path.extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase()
                        .as_str(),
                    "lnk" | "url"
                ) {
                    ItemKind::Shortcut
                } else {
                    ItemKind::File
                };
                let has_distinct_full_name = full != label;
                map.entry(label.clone()).or_default().push(kind);
                if has_distinct_full_name {
                    map.entry(full).or_default().push(kind);
                }
            }
        }
        for icon in icons.iter_mut() {
            if shell_system_name(&icon.name) {
                icon.kind = ItemKind::System;
                continue;
            }
            let k = icon.name.to_lowercase();
            if let Some(v) = map.get(&k) {
                if v.len() == 1 {
                    icon.kind = v[0];
                } else {
                    icon.kind = ItemKind::Unknown;
                }
            } else {
                icon.kind = ItemKind::Unknown;
            }
        }
    }
    fn snapshot_live() -> io::Result<(Desktop, Snapshot)> {
        let d = connect()?;
        let (w, h) = d.dimensions();
        let mut icons = d.read_icons()?;
        classify(&mut icons);
        let snap = Snapshot {
            schema_version: 1,
            product: "DesktopLayoutManager Rust".into(),
            captured_at_unix_ms: now_ms(),
            machine: env::var("COMPUTERNAME").unwrap_or_default(),
            os: env::var("OS").unwrap_or_else(|_| "Windows".into()),
            screen_width: w,
            screen_height: h,
            icons,
        };
        Ok((d, snap))
    }
    fn same_layout(a: &Snapshot, b: &Snapshot) -> bool {
        a.machine == b.machine
            && a.screen_width == b.screen_width
            && a.screen_height == b.screen_height
            && a.icons == b.icons
    }
    fn infer_spacing(vals: &[i32]) -> Option<i32> {
        let mut v = vals.to_vec();
        v.sort_unstable();
        v.dedup();
        v.windows(2)
            .map(|w| w[1] as i64 - w[0] as i64)
            .filter(|d| (40..=512).contains(d))
            .min()
            .map(|d| d as i32)
    }
    fn nearest_grid_index(coord: i32, origin: i32, spacing: i32) -> i32 {
        let delta = coord as i64 - origin as i64;
        ((delta + spacing as i64 / 2).div_euclid(spacing as i64)) as i32
    }
    fn on_grid(coord: i32, origin: i32, spacing: i32) -> bool {
        let cell = nearest_grid_index(coord, origin, spacing);
        (coord as i64 - (origin as i64 + cell as i64 * spacing as i64)).abs() <= 12
    }
    fn plan_fill(d: &Desktop, s: &Snapshot) -> io::Result<Vec<Move>> {
        unsafe {
            if GetSystemMetrics(80) > 1 {
                return Err(io::Error::other(
                    "检测到多个显示器。当前版本暂不支持多显示器补位，为避免跨屏移动已拒绝执行。",
                ));
            }
        }
        if d.auto_arrange() {
            return Err(io::Error::other("Windows 桌面已启用自动排列，拒绝执行以避免 Explorer 覆盖目标位置。请关闭自动排列后重试。"));
        }
        if s.icons.is_empty() {
            return Ok(Vec::new());
        }
        if s.icons.iter().any(|i| i.kind == ItemKind::Unknown) {
            let names = s
                .icons
                .iter()
                .filter(|i| i.kind == ItemKind::Unknown)
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>()
                .join("、");
            return Err(io::Error::other(format!("存在无法可靠识别身份的桌面项目，按安全策略拒绝整理：{names}。快照仍可正常保存；请确认项目类型后再尝试补位。")));
        }
        let (width, height) = d.dimensions();
        if width <= 0 || height <= 0 {
            return Err(io::Error::other(
                "无法读取有效的桌面工作区尺寸，已停止补位。",
            ));
        }
        if s.icons.iter().any(|i| {
            i.x < 0
                || i.y < 0
                || i.x >= width
                || i.y >= height
                || i.x > u16::MAX as i32
                || i.y > u16::MAX as i32
        }) {
            return Err(io::Error::other(
                "存在超出桌面工作区或 Win32 ListView 坐标范围的图标，已停止补位。",
            ));
        }
        let xs: Vec<_> = s.icons.iter().map(|i| i.x).collect();
        let ys: Vec<_> = s.icons.iter().map(|i| i.y).collect();
        let dx =
            infer_spacing(&xs).ok_or_else(|| io::Error::other("无法可靠推断水平网格间距。"))?;
        let dy =
            infer_spacing(&ys).ok_or_else(|| io::Error::other("无法可靠推断垂直网格间距。"))?;
        let ox = *xs.iter().min().unwrap_or(&0);
        let oy = *ys.iter().min().unwrap_or(&0);
        let mut occupied_cells = std::collections::HashSet::new();
        for i in &s.icons {
            if !on_grid(i.x, ox, dx) || !on_grid(i.y, oy, dy) {
                return Err(io::Error::other(format!(
                    "图标“{}”不在推断网格上，拒绝自动整理。",
                    i.name
                )));
            }
            let cell = (
                nearest_grid_index(i.x, ox, dx),
                nearest_grid_index(i.y, oy, dy),
            );
            if !occupied_cells.insert(cell) {
                return Err(io::Error::other(format!(
                    "多个图标占据同一个推断网格单元（{}，{}），拒绝自动整理。",
                    cell.0, cell.1
                )));
            }
        }
        let first_reserved = s
            .icons
            .iter()
            .filter(|i| matches!(i.kind, ItemKind::File | ItemKind::Folder))
            .map(|i| nearest_grid_index(i.x, ox, dx))
            .min();
        let mut shortcuts: Vec<&Icon> = s
            .icons
            .iter()
            .filter(|i| {
                i.kind == ItemKind::Shortcut
                    && first_reserved
                        .map(|c| nearest_grid_index(i.x, ox, dx) < c)
                        .unwrap_or(true)
            })
            .collect();
        shortcuts.sort_by_key(|i| {
            (
                nearest_grid_index(i.x, ox, dx),
                nearest_grid_index(i.y, oy, dy),
                i.original_index,
            )
        });
        if shortcuts.is_empty() {
            if s.icons.iter().any(|i| i.kind == ItemKind::Shortcut) {
                return Err(io::Error::other("当前快捷方式都位于真实文件或文件夹保留列的右侧；按安全策略不跨越保留列移动图标。"));
            }
            return Ok(Vec::new());
        }
        let max_col = first_reserved.unwrap_or_else(|| {
            s.icons
                .iter()
                .map(|i| nearest_grid_index(i.x, ox, dx))
                .max()
                .unwrap_or(0)
                + 1
        });
        let rows = ((height as i64 - 1 - oy as i64).max(0) / dy as i64 + 1).max(1) as i32;
        let protected_cells: std::collections::HashSet<(i32, i32)> = s
            .icons
            .iter()
            .filter(|i| i.kind != ItemKind::Shortcut)
            .map(|i| {
                (
                    nearest_grid_index(i.x, ox, dx),
                    nearest_grid_index(i.y, oy, dy),
                )
            })
            .collect();
        let mut targets = Vec::new();
        for col in 0..max_col {
            for row in 0..rows {
                let x = ox as i64 + col as i64 * dx as i64;
                let y = oy as i64 + row as i64 * dy as i64;
                if x >= 0
                    && x < width as i64
                    && y >= 0
                    && y < height as i64
                    && !protected_cells.contains(&(col, row))
                {
                    targets.push((x as i32, y as i32));
                }
            }
        }
        if targets.len() < shortcuts.len() {
            return Err(io::Error::other(
                "保留区域内可用网格不足以容纳快捷方式，未执行任何移动。",
            ));
        }
        let mut moves = Vec::new();
        for (icon, (x, y)) in shortcuts.into_iter().zip(targets) {
            if icon.x != x || icon.y != y {
                moves.push(Move {
                    index: icon.original_index,
                    name: icon.name.clone(),
                    from_x: icon.x,
                    from_y: icon.y,
                    to_x: x,
                    to_y: y,
                });
            }
        }
        Ok(moves)
    }
    fn backup_named(snapshot: &Snapshot, prefix: &str) -> io::Result<PathBuf> {
        let path = default_backup_dir().join(format!(
            "{prefix}-{}-{}-{}.json",
            timestamp(),
            std::process::id(),
            TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        save_snapshot(&path, snapshot)?;
        Ok(path)
    }

    fn backup(snapshot: &Snapshot) -> io::Result<PathBuf> {
        backup_named(snapshot, "before-fill")
    }

    fn plan_restore(saved: &Snapshot, current: &Snapshot) -> (Vec<Move>, Vec<String>) {
        use std::collections::HashSet;

        let mut used = HashSet::new();
        let mut moves = Vec::new();
        let mut unmatched = Vec::new();
        for target in &saved.icons {
            let matches: Vec<&Icon> = current
                .icons
                .iter()
                .filter(|icon| {
                    icon.name.eq_ignore_ascii_case(&target.name)
                        && !used.contains(&icon.original_index)
                })
                .collect();
            if matches.len() == 1 {
                let icon = matches[0];
                used.insert(icon.original_index);
                if matches!(
                    target.kind,
                    ItemKind::File | ItemKind::Folder | ItemKind::System
                ) {
                    unmatched.push(format!("{}（快照标记为受保护类型，未移动）", target.name));
                } else if icon.kind != ItemKind::Shortcut {
                    unmatched.push(format!("{}（当前类型受保护，未移动）", target.name));
                } else if target.x < 0
                    || target.y < 0
                    || target.x >= current.screen_width
                    || target.y >= current.screen_height
                    || target.x > u16::MAX as i32
                    || target.y > u16::MAX as i32
                {
                    unmatched.push(format!("{}（快照坐标超出当前桌面安全范围）", target.name));
                } else if icon.x != target.x || icon.y != target.y {
                    moves.push(Move {
                        index: icon.original_index,
                        name: icon.name.clone(),
                        from_x: icon.x,
                        from_y: icon.y,
                        to_x: target.x,
                        to_y: target.y,
                    });
                }
            } else if matches.is_empty() {
                unmatched.push(format!("{}（当前缺失）", target.name));
            } else {
                unmatched.push(format!("{}（名称不唯一）", target.name));
            }
        }
        let mut blocked = HashSet::new();
        loop {
            let active: HashSet<i32> = moves
                .iter()
                .filter(|movement| !blocked.contains(&movement.index))
                .map(|movement| movement.index)
                .collect();
            let mut conflicts = HashSet::new();
            let mut destinations: std::collections::HashMap<(i32, i32), Vec<i32>> =
                std::collections::HashMap::new();
            for movement in moves
                .iter()
                .filter(|movement| active.contains(&movement.index))
            {
                destinations
                    .entry((movement.to_x, movement.to_y))
                    .or_default()
                    .push(movement.index);
                let occupied_by_stationary = current.icons.iter().any(|icon| {
                    icon.original_index != movement.index
                        && icon.x == movement.to_x
                        && icon.y == movement.to_y
                        && !active.contains(&icon.original_index)
                });
                if occupied_by_stationary {
                    conflicts.insert(movement.index);
                }
            }
            for owners in destinations.values().filter(|owners| owners.len() > 1) {
                conflicts.extend(owners.iter().copied());
            }
            if conflicts.is_empty() {
                break;
            }
            for &index in &conflicts {
                if let Some(movement) = moves.iter().find(|movement| movement.index == index) {
                    unmatched.push(format!(
                        "{}（恢复目标与受保护或未移动图标冲突，已跳过）",
                        movement.name
                    ));
                }
            }
            blocked.extend(conflicts);
        }
        moves.retain(|movement| !blocked.contains(&movement.index));
        (moves, unmatched)
    }

    fn plan_restore_shortcuts(saved: &Snapshot, current: &Snapshot) -> (Vec<Move>, Vec<String>) {
        let mut shortcuts = saved.clone();
        shortcuts
            .icons
            .retain(|icon| icon.kind == ItemKind::Shortcut);
        plan_restore(&shortcuts, current)
    }

    fn apply_moves(desktop: &Desktop, moves: &[Move]) -> io::Result<()> {
        let mut completed: Vec<&Move> = Vec::new();
        for movement in moves {
            let source_index =
                match desktop.find_icon_index(&movement.name, movement.from_x, movement.from_y) {
                    Ok(index) => index,
                    Err(error) => {
                        let error = io::Error::other(format!(
                            "移动前无法重新识别图标“{}”：{error}。桌面已发生变化。",
                            movement.name
                        ));
                        return Err(rollback_after_failure(desktop, &completed, movement, error));
                    }
                };
            if let Err(error) = desktop.set_pos(source_index, movement.to_x, movement.to_y) {
                return Err(rollback_after_failure(desktop, &completed, movement, error));
            }
            completed.push(movement);
            if let Err(error) =
                desktop.find_icon_index(&movement.name, movement.to_x, movement.to_y)
            {
                return Err(rollback_after_failure(desktop, &completed, movement, error));
            }
        }
        Ok(())
    }

    fn rollback_after_failure(
        desktop: &Desktop,
        completed: &[&Move],
        failed: &Move,
        error: io::Error,
    ) -> io::Error {
        let mut rollback_failures = Vec::new();
        for previous in completed.iter().rev() {
            let rollback = desktop
                .find_icon_index(&previous.name, previous.to_x, previous.to_y)
                .and_then(|index| desktop.set_pos(index, previous.from_x, previous.from_y));
            if let Err(rollback_error) = rollback {
                rollback_failures.push(format!("{}：{}", previous.name, rollback_error));
            }
        }
        let rollback_summary = if rollback_failures.is_empty() {
            "先前已移动的图标已回滚。".to_owned()
        } else {
            format!(
                "部分回滚失败：{}。请立即检查桌面并使用备份恢复。",
                rollback_failures.join("；")
            )
        };
        io::Error::other(format!(
            "移动“{}”失败：{error} {}",
            failed.name, rollback_summary
        ))
    }

    fn execute_fill(desktop: &Desktop, snapshot: &Snapshot, moves: &[Move]) -> io::Result<PathBuf> {
        let backup_path = backup(snapshot)?;
        if let Err(error) = apply_moves(desktop, moves) {
            return Err(io::Error::other(format!(
                "{error}\n执行前备份：{}",
                backup_path.display()
            )));
        }
        Ok(backup_path)
    }

    fn execute_restore(
        desktop: &Desktop,
        current: &Snapshot,
        moves: &[Move],
    ) -> io::Result<PathBuf> {
        if desktop.auto_arrange() {
            return Err(io::Error::other(
                "Windows 桌面已启用自动排列，无法安全恢复手动坐标。请关闭自动排列后重试。",
            ));
        }
        let backup_path = backup_named(current, "before-restore")?;
        if let Err(error) = apply_moves(desktop, moves) {
            return Err(io::Error::other(format!(
                "{error}\n恢复前备份：{}",
                backup_path.display()
            )));
        }
        Ok(backup_path)
    }

    fn required<'a>(
        request: &'a serde_json::Value,
        key: &str,
    ) -> Result<&'a serde_json::Value, String> {
        request
            .get(key)
            .ok_or_else(|| format!("请求缺少字段：{key}"))
    }

    fn path_arg(request: &serde_json::Value) -> Result<PathBuf, String> {
        let value = required(request, "path")?;
        let path = value
            .as_str()
            .ok_or_else(|| "文件路径必须是文本。".to_owned())?;
        if path.trim().is_empty() {
            return Err("文件路径不能为空。".to_owned());
        }
        Ok(PathBuf::from(path))
    }

    fn decode<T: serde::de::DeserializeOwned>(
        request: &serde_json::Value,
        key: &str,
    ) -> Result<T, String> {
        serde_json::from_value(required(request, key)?.clone())
            .map_err(|error| format!("无法读取请求中的 {key}：{error}"))
    }

    fn dispatch(request: serde_json::Value) -> Result<serde_json::Value, String> {
        use serde_json::json;

        let action = request
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "请求缺少操作名称。".to_owned())?;
        match action {
            "read" => {
                let (_desktop, snapshot) = snapshot_live().map_err(|error| error.to_string())?;
                Ok(json!({ "snapshot": snapshot }))
            }
            "save_snapshot" => {
                let path = path_arg(&request)?;
                let (_desktop, snapshot) = snapshot_live().map_err(|error| error.to_string())?;
                save_snapshot(&path, &snapshot).map_err(|error| error.to_string())?;
                Ok(json!({ "snapshot": snapshot, "path": path.to_string_lossy() }))
            }
            "load_snapshot" => {
                let path = path_arg(&request)?;
                let snapshot = load_snapshot(&path).map_err(|error| error.to_string())?;
                Ok(json!({ "snapshot": snapshot, "path": path.to_string_lossy() }))
            }
            "preview_fill" => {
                let (desktop, snapshot) = snapshot_live().map_err(|error| error.to_string())?;
                let moves = plan_fill(&desktop, &snapshot).map_err(|error| error.to_string())?;
                Ok(json!({ "snapshot": snapshot, "moves": moves }))
            }
            "execute_fill" => {
                let expected_snapshot: Snapshot = decode(&request, "expectedSnapshot")?;
                let expected_moves: Vec<Move> = decode(&request, "expectedMoves")?;
                let (desktop, current) = snapshot_live().map_err(|error| error.to_string())?;
                let current_moves =
                    plan_fill(&desktop, &current).map_err(|error| error.to_string())?;
                if !same_layout(&expected_snapshot, &current) || expected_moves != current_moves {
                    return Err("确认期间桌面布局发生变化。为防止移动错误图标，本次操作已取消，请重新预览。".to_owned());
                }
                if current_moves.is_empty() {
                    return Ok(
                        json!({ "snapshot": current, "moves": current_moves, "backupPath": null }),
                    );
                }
                let backup_path = execute_fill(&desktop, &current, &current_moves)
                    .map_err(|error| error.to_string())?;
                Ok(json!({
                    "snapshot": current,
                    "moves": current_moves,
                    "backupPath": backup_path.to_string_lossy()
                }))
            }
            "preview_restore" => {
                let saved: Snapshot = decode(&request, "savedSnapshot")?;
                let (_desktop, current) = snapshot_live().map_err(|error| error.to_string())?;
                let (moves, skipped) = plan_restore(&saved, &current);
                Ok(json!({ "snapshot": current, "moves": moves, "skipped": skipped }))
            }
            "preview_undo" => {
                let path = path_arg(&request)?;
                let saved = load_snapshot(&path).map_err(|error| error.to_string())?;
                let (_desktop, current) = snapshot_live().map_err(|error| error.to_string())?;
                let (moves, skipped) = plan_restore_shortcuts(&saved, &current);
                Ok(
                    json!({ "snapshot": current, "savedSnapshot": saved, "moves": moves, "skipped": skipped }),
                )
            }
            "execute_restore" | "execute_undo" => {
                let saved: Snapshot = decode(&request, "savedSnapshot")?;
                let expected_current: Snapshot = decode(&request, "expectedSnapshot")?;
                let expected_moves: Vec<Move> = decode(&request, "expectedMoves")?;
                let expected_skipped: Vec<String> = decode(&request, "expectedSkipped")?;
                let (desktop, current) = snapshot_live().map_err(|error| error.to_string())?;
                let (moves, skipped) = if action == "execute_undo" {
                    plan_restore_shortcuts(&saved, &current)
                } else {
                    plan_restore(&saved, &current)
                };
                if !same_layout(&expected_current, &current)
                    || moves != expected_moves
                    || skipped != expected_skipped
                {
                    return Err("确认期间桌面布局发生变化。为防止恢复到错误位置，本次操作已取消，请重新预览。".to_owned());
                }
                if action == "execute_undo" && !skipped.is_empty() {
                    return Err(format!("无法唯一匹配以下快捷方式：{}", skipped.join("、")));
                }
                if moves.is_empty() {
                    return Ok(
                        json!({ "snapshot": current, "moves": moves, "skipped": skipped, "backupPath": null }),
                    );
                }
                let backup_path = execute_restore(&desktop, &current, &moves)
                    .map_err(|error| error.to_string())?;
                Ok(json!({
                    "snapshot": current,
                    "moves": moves,
                    "skipped": skipped,
                    "backupPath": backup_path.to_string_lossy()
                }))
            }
            "export" => {
                let path = path_arg(&request)?;
                let (_desktop, snapshot) = snapshot_live().map_err(|error| error.to_string())?;
                if path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
                {
                    fn csv_field(value: &str) -> String {
                        format!("\"{}\"", value.replace('"', "\"\""))
                    }
                    let mut lines = vec!["Index,Name,X,Y".to_owned()];
                    lines.extend(snapshot.icons.iter().map(|icon| {
                        format!(
                            "{},{},{},{}",
                            icon.original_index,
                            csv_field(&icon.name),
                            icon.x,
                            icon.y
                        )
                    }));
                    fs::write(&path, format!("\u{feff}{}", lines.join("\r\n")))
                        .map_err(|error| error.to_string())?;
                } else {
                    save_snapshot(&path, &snapshot).map_err(|error| error.to_string())?;
                }
                Ok(json!({ "count": snapshot.icons.len(), "path": path.to_string_lossy() }))
            }
            _ => Err(format!("不支持的操作：{action}")),
        }
    }

    #[cfg(windows)]
    #[no_mangle]
    pub unsafe extern "C" fn dlm_call(request: *const std::ffi::c_char) -> *mut std::ffi::c_char {
        use std::ffi::{CStr, CString};
        use std::panic::{catch_unwind, AssertUnwindSafe};

        let result = catch_unwind(AssertUnwindSafe(|| {
            if request.is_null() {
                return Err("请求为空。".to_owned());
            }
            let text = CStr::from_ptr(request)
                .to_str()
                .map_err(|error| format!("请求不是有效 UTF-8：{error}"))?;
            let value: serde_json::Value = serde_json::from_str(text)
                .map_err(|error| format!("请求 JSON 格式错误：{error}"))?;
            dispatch(value)
        }));
        let response = match result {
            Ok(Ok(data)) => serde_json::json!({ "ok": true, "data": data }),
            Ok(Err(error)) => serde_json::json!({ "ok": false, "error": error }),
            Err(_) => {
                serde_json::json!({ "ok": false, "error": "桌面操作发生内部错误，未能完成。" })
            }
        };
        CString::new(response.to_string())
            .unwrap_or_else(|_| {
                CString::new("{\"ok\":false,\"error\":\"响应编码失败。\"}").unwrap()
            })
            .into_raw()
    }

    #[cfg(windows)]
    #[no_mangle]
    pub unsafe extern "C" fn dlm_free_string(value: *mut std::ffi::c_char) {
        if !value.is_null() {
            drop(std::ffi::CString::from_raw(value));
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn icon(name: &str, index: i32, x: i32, kind: ItemKind) -> Icon {
            Icon {
                original_index: index,
                name: name.into(),
                x,
                y: 20,
                kind,
                legacy_is_file_or_folder: false,
            }
        }

        fn snapshot(icons: Vec<Icon>) -> Snapshot {
            Snapshot {
                schema_version: 1,
                product: "test".into(),
                captured_at_unix_ms: 0,
                machine: "test".into(),
                os: "Windows".into(),
                screen_width: 1920,
                screen_height: 1080,
                icons,
            }
        }

        #[test]
        fn restore_only_moves_uniquely_matched_shortcuts() {
            let saved = snapshot(vec![
                icon("Shortcut", 0, 10, ItemKind::Unknown),
                icon("Document", 1, 40, ItemKind::Unknown),
                icon("System", 2, 70, ItemKind::System),
                icon("ProtectedInSave", 3, 100, ItemKind::File),
            ]);
            let current = snapshot(vec![
                icon("shortcut", 4, 30, ItemKind::Shortcut),
                icon("Document", 5, 60, ItemKind::File),
                icon("System", 6, 90, ItemKind::System),
                icon("ProtectedInSave", 7, 120, ItemKind::Shortcut),
            ]);

            let (moves, skipped) = plan_restore(&saved, &current);

            assert_eq!(moves.len(), 1);
            assert_eq!(moves[0].name, "shortcut");
            assert_eq!(moves[0].from_x, 30);
            assert_eq!(moves[0].to_x, 10);
            assert_eq!(skipped.len(), 3);
        }

        #[test]
        fn undo_ignores_items_that_fill_never_moves() {
            let saved = snapshot(vec![
                icon("Shortcut", 0, 10, ItemKind::Shortcut),
                icon("Document", 1, 40, ItemKind::File),
                icon("Folder", 2, 70, ItemKind::Folder),
                icon("Recycle Bin", 3, 100, ItemKind::System),
            ]);
            let current = snapshot(vec![
                icon("Shortcut", 0, 30, ItemKind::Shortcut),
                icon("Document", 1, 40, ItemKind::File),
                icon("Folder", 2, 70, ItemKind::Folder),
                icon("Recycle Bin", 3, 100, ItemKind::System),
            ]);

            let (moves, skipped) = plan_restore_shortcuts(&saved, &current);

            assert_eq!(moves.len(), 1);
            assert_eq!(moves[0].name, "Shortcut");
            assert!(skipped.is_empty());
        }

        #[test]
        fn restore_skips_targets_occupied_by_stationary_protected_icons() {
            let saved = snapshot(vec![icon("Shortcut", 0, 10, ItemKind::Unknown)]);
            let current = snapshot(vec![
                icon("Shortcut", 1, 30, ItemKind::Shortcut),
                icon("Document", 2, 10, ItemKind::File),
            ]);

            let (moves, skipped) = plan_restore(&saved, &current);

            assert!(moves.is_empty());
            assert!(skipped.iter().any(|item| item.contains("目标与受保护")));
        }

        #[test]
        fn restore_skips_coordinates_outside_the_current_desktop() {
            let saved = snapshot(vec![icon("Shortcut", 0, 2000, ItemKind::Unknown)]);
            let current = snapshot(vec![icon("Shortcut", 1, 30, ItemKind::Shortcut)]);

            let (moves, skipped) = plan_restore(&saved, &current);

            assert!(moves.is_empty());
            assert!(skipped.iter().any(|item| item.contains("超出当前桌面")));
        }

        #[test]
        fn restore_allows_shortcut_position_cycles() {
            let saved = snapshot(vec![
                icon("A", 0, 30, ItemKind::Unknown),
                icon("B", 1, 10, ItemKind::Unknown),
            ]);
            let current = snapshot(vec![
                icon("A", 2, 10, ItemKind::Shortcut),
                icon("B", 3, 30, ItemKind::Shortcut),
            ]);

            let (moves, skipped) = plan_restore(&saved, &current);

            assert_eq!(moves.len(), 2);
            assert!(skipped.is_empty());
        }

        #[test]
        fn grid_rounding_keeps_small_coordinate_jitter_in_protected_cell() {
            assert_eq!(nearest_grid_index(211, 100, 110), 1);
            assert!(on_grid(211, 100, 110));
            assert_eq!(nearest_grid_index(223, 100, 110), 1);
            assert!(!on_grid(223, 100, 110));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_snapshot() -> Snapshot {
        Snapshot {
            schema_version: 1,
            product: "DesktopLayoutManager Rust".into(),
            captured_at_unix_ms: now_ms(),
            machine: "test-machine".into(),
            os: "Windows 11".into(),
            screen_width: 1920,
            screen_height: 1080,
            icons: Vec::new(),
        }
    }

    #[test]
    fn reads_original_csharp_snapshot_shape() {
        let json = r#"{
            "SchemaVersion": 1,
            "Product": "Desktop Layout Manager",
            "CapturedAt": "2026-10-18T12:00:00+08:00",
            "Machine": "test-machine",
            "WindowsVersion": "Windows 11",
            "ScreenWidth": 1920,
            "ScreenHeight": 1080,
            "Monitors": [],
            "Icons": [{"OriginalIndex": 0, "Name": "Shortcut", "X": 12, "Y": 24}]
        }"#;
        let snapshot: Snapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snapshot.schema_version, 1);
        assert_eq!(snapshot.os, "Windows 11");
        assert_eq!(snapshot.icons[0].kind, ItemKind::Unknown);
    }

    #[test]
    fn snapshot_save_replaces_existing_file_and_roundtrips() {
        let path = env::temp_dir().join(format!(
            "desktop-layout-manager-test-{}-{}.json",
            std::process::id(),
            TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let mut first = empty_snapshot();
        first.product = "first".into();
        save_snapshot(&path, &first).unwrap();
        let mut second = empty_snapshot();
        second.product = "second".into();
        save_snapshot(&path, &second).unwrap();
        let loaded = load_snapshot(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(loaded.product, "second");
    }

    #[test]
    fn snapshot_loader_rejects_unknown_schema_version() {
        let path = env::temp_dir().join(format!(
            "desktop-layout-manager-invalid-{}-{}.json",
            std::process::id(),
            TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let mut snapshot = empty_snapshot();
        snapshot.schema_version = 2;
        fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
        let result = load_snapshot(&path);
        let _ = fs::remove_file(&path);
        assert!(result.is_err());
    }
}
