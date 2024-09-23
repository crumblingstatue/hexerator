use {
    crate::{
        source::{Source, SourceAttributes, SourcePermissions, SourceProvider, SourceState},
        App,
    },
    anyhow::bail,
    egui_sfml::sfml::graphics::Font,
    windows_sys::Win32::System::Threading::*,
};

pub fn load_proc_memory(
    app: &mut App,
    pid: sysinfo::Pid,
    start: usize,
    size: usize,
    _is_write: bool,
    font_size: u16,
    line_spacing: u16,
) -> anyhow::Result<()> {
    let handle;
    unsafe {
        let access =
            PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION;
        handle = windows_sys::Win32::System::Threading::OpenProcess(access, 0, pid.as_u32());
        if handle == 0 {
            bail!("Failed to open process.");
        }
        load_proc_memory_inner(app, handle, start, size, font_size, line_spacing)
    }
}

unsafe fn load_proc_memory_inner(
    app: &mut App,
    handle: windows_sys::Win32::Foundation::HANDLE,
    start: usize,
    size: usize,
    font_size: u16,
    line_spacing: u16,
) -> anyhow::Result<()> {
    read_proc_memory(handle, &mut app.data, start, size)?;
    app.source = Some(Source {
        attr: SourceAttributes {
            permissions: SourcePermissions { write: true },
            stream: false,
        },
        provider: SourceProvider::WinProc {
            handle,
            start,
            size,
        },
        state: SourceState::default(),
    });
    if !app.preferences.keep_meta {
        app.set_new_clean_meta(font_size, line_spacing);
    }
    app.src_args.hard_seek = Some(start);
    app.src_args.take = Some(size);
    Ok(())
}

pub unsafe fn read_proc_memory(
    handle: windows_sys::Win32::Foundation::HANDLE,
    data: &mut Vec<u8>,
    start: usize,
    size: usize,
) -> anyhow::Result<()> {
    let mut n_read: usize = 0;
    data.resize(size, 0);
    if windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory(
        handle,
        start as _,
        data.as_mut_ptr() as *mut std::ffi::c_void,
        size,
        &mut n_read,
    ) == 0
    {
        bail!(
            "Failed to load process memory. Code: {}",
            windows_sys::Win32::Foundation::GetLastError()
        );
    }
    Ok(())
}
