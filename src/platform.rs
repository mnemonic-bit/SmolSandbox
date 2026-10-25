use anyhow::{bail, Context, Result};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

pub fn canonical_workspace(path: &Path) -> Result<PathBuf> {
    let path = fs::canonicalize(path)
        .with_context(|| format!("cannot resolve workspace {}", path.display()))?;
    if !path.is_dir() {
        bail!("workspace is not a directory: {}", path.display());
    }
    #[cfg(windows)]
    let path = {
        let text = path.to_str().context("workspace path is not valid UTF-8")?;
        if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
            PathBuf::from(format!(r"\\{unc}"))
        } else {
            PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text))
        }
    };
    path.to_str().context("workspace path is not valid UTF-8")?;
    Ok(path)
}

pub fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("cannot create directory {}", path.display()))?;
    set_private(path, true)
}

pub fn set_private(path: &Path, directory: bool) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
        )
        .with_context(|| format!("cannot restrict permissions on {}", path.display()))?;
    }
    #[cfg(windows)]
    {
        use std::{ffi::c_void, os::windows::ffi::OsStrExt, ptr};
        #[link(name = "advapi32")]
        extern "system" {
            fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text: *const u16,
                revision: u32,
                descriptor: *mut *mut c_void,
                size: *mut u32,
            ) -> i32;
            fn SetFileSecurityW(path: *const u16, information: u32, descriptor: *mut c_void)
                -> i32;
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn LocalFree(memory: *mut c_void) -> *mut c_void;
        }
        let descriptor_text: Vec<u16> = if directory {
            "D:P(A;OICI;FA;;;OW)\0"
        } else {
            "D:P(A;;FA;;;OW)\0"
        }
        .encode_utf16()
        .collect();
        let filename: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut descriptor = ptr::null_mut();
        unsafe {
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                descriptor_text.as_ptr(),
                1,
                &mut descriptor,
                ptr::null_mut(),
            ) == 0
            {
                return Err(std::io::Error::last_os_error())
                    .context("cannot prepare private Windows permissions");
            }
            let result = SetFileSecurityW(filename.as_ptr(), 0x80000004, descriptor);
            let error = std::io::Error::last_os_error();
            LocalFree(descriptor);
            if result == 0 {
                return Err(error)
                    .with_context(|| format!("cannot restrict permissions on {}", path.display()));
            }
        }
    }
    Ok(())
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("destination has no parent directory")?;
    private_dir(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    set_private(temporary.path(), false)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("cannot publish {}", path.display()))?;
    sync_directory(parent)?;
    Ok(())
}

pub fn publish_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("destination has no parent directory")?;
    private_dir(parent)?;
    publish_new_file(path, bytes)
}

pub fn publish_new_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("destination has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("cannot create directory {}", parent.display()))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    set_private(temporary.path(), false)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(path)
        .map_err(|error| error.error)
        .with_context(|| {
            format!(
                "cannot create {} without replacing an existing file",
                path.display()
            )
        })?;
    sync_directory(parent)?;
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub struct FileLock {
    file: File,
}
impl FileLock {
    pub fn acquire(path: &Path) -> Result<Self> {
        Self::acquire_timeout(path, Duration::from_secs(60))
    }
    fn acquire_timeout(path: &Path, timeout: Duration) -> Result<Self> {
        private_dir(path.parent().context("lock has no parent directory")?)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        set_private(path, false)?;
        let start = Instant::now();
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self { file }),
                Err(error)
                    if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() =>
                {
                    #[cfg(feature = "test-adapter")]
                    if let Some(marker) = std::env::var_os("SMOLVM_TEST_LOCK_CONTENDED") {
                        fs::write(marker, path.to_string_lossy().as_bytes())?;
                    }
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("cannot acquire lock {}", path.display()))
                }
            }
            if start.elapsed() >= timeout {
                bail!(
                    "timed out after {} seconds waiting for lock {}",
                    timeout.as_secs(),
                    path.display()
                );
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}
impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

pub fn virtualization_preflight() -> Result<()> {
    if !matches!(
        (std::env::consts::OS, std::env::consts::ARCH),
        ("linux", "x86_64" | "aarch64") | ("macos", "aarch64") | ("windows", "x86_64")
    ) {
        bail!("unsupported SmolVM host: {}/{}; use Linux x86-64/ARM64, macOS ARM64, or Windows x86-64",std::env::consts::OS,std::env::consts::ARCH);
    }
    #[cfg(target_os="linux")]
    OpenOptions::new().read(true).write(true).open("/dev/kvm").context("KVM is unavailable: enable hardware virtualization and grant this user read/write access to /dev/kvm")?;
    #[cfg(target_os = "macos")]
    if !Path::new("/System/Library/Frameworks/Hypervisor.framework").exists() {
        bail!("Hypervisor.framework is unavailable; use a supported macOS host");
    }
    #[cfg(target_os = "windows")]
    unsafe {
        use std::ffi::c_void;
        #[link(name = "kernel32")]
        extern "system" {
            fn LoadLibraryW(name: *const u16) -> *mut c_void;
            fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
            fn FreeLibrary(module: *mut c_void) -> i32;
        }
        let name: Vec<u16> = "WinHvPlatform.dll\0".encode_utf16().collect();
        let library = LoadLibraryW(name.as_ptr());
        if library.is_null() {
            bail!("Windows Hypervisor Platform is unavailable; enable the Windows Hypervisor Platform optional feature");
        }
        let symbol = GetProcAddress(library, c"WHvGetCapability".as_ptr().cast());
        if symbol.is_null() {
            FreeLibrary(library);
            bail!("Windows Hypervisor Platform lacks the required capability interface");
        }
        let capability: unsafe extern "system" fn(u32, *mut u32, u32, *mut u32) -> i32 =
            std::mem::transmute(symbol);
        let mut present = 0u32;
        let mut written = 0u32;
        let status = capability(0, &mut present, 4, &mut written);
        FreeLibrary(library);
        if status < 0 || present == 0 {
            bail!("Windows Hypervisor Platform is unavailable; enable virtualization and the Windows Hypervisor Platform optional feature");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner_only_windows_dacl(sddl: &str, directory: bool) -> bool {
        if !sddl.starts_with("D:") {
            return false;
        }
        let Some((_, ace)) = sddl.split_once('(') else {
            return false;
        };
        let Some(ace) = ace.strip_suffix(')') else {
            return false;
        };
        let fields: Vec<_> = ace.split(';').collect();
        // Check effective owner access and propagation. ID records inheritance
        // provenance and is not required in the ACL returned by the legacy API.
        fields.len() == 6
            && fields[0] == "A"
            && fields[2] == "FA"
            && fields[3].is_empty()
            && fields[4].is_empty()
            && fields[5] == "OW"
            && fields[1]
                .as_bytes()
                .chunks(2)
                .all(|flag| matches!(flag, b"OI" | b"CI" | b"ID"))
            && (!directory || (fields[1].contains("OI") && fields[1].contains("CI")))
    }

    #[test]
    fn windows_dacl_accepts_reported_owner_only_permissions_without_provenance_flag() {
        assert!(owner_only_windows_dacl("D:(A;OICI;FA;;;OW)", true));
        assert!(owner_only_windows_dacl("D:(A;;FA;;;OW)", false));
    }

    #[test]
    fn windows_dacl_accepts_owner_only_permissions_with_provenance_flag() {
        assert!(owner_only_windows_dacl("D:AI(A;OICIID;FA;;;OW)", true));
        assert!(owner_only_windows_dacl("D:AI(A;ID;FA;;;OW)", false));
    }

    #[test]
    fn windows_dacl_rejects_broad_access_and_broken_propagation() {
        for (sddl, directory) in [
            ("D:(A;OICIID;FA;;;WD)", true),
            ("D:(A;OICIID;FA;;;OW)(A;;FR;;;WD)", true),
            ("D:(D;OICIID;FA;;;OW)", true),
            ("D:(A;OICIID;FR;;;OW)", true),
            ("D:(A;OICIIOID;FA;;;OW)", true),
            ("D:(A;OICINPID;FA;;;OW)", true),
            ("D:(A;OIID;FA;;;OW)", true),
            ("D:(A;CIID;FA;;;OW)", true),
            ("D:(A;ID;FA;;;OW)", true),
            ("D:", false),
        ] {
            assert!(!owner_only_windows_dacl(sddl, directory), "{sddl}");
        }
    }
    #[test]
    fn publication_never_clobbers() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config");
        publish_new(&path, b"first").unwrap();
        assert!(publish_new(&path, b"second").is_err());
        assert_eq!(fs::read(path).unwrap(), b"first");
    }
    #[test]
    fn exclusive_lock_times_out_and_releases() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("lock");
        let held = FileLock::acquire(&path).unwrap();
        let started = Instant::now();
        assert!(FileLock::acquire_timeout(&path, Duration::from_millis(20)).is_err());
        assert!(started.elapsed() >= Duration::from_millis(20));
        drop(held);
        FileLock::acquire(&path).unwrap();
    }
    #[test]
    fn concurrent_publication_has_exactly_one_complete_winner() {
        use std::sync::{Arc, Barrier};
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config");
        let barrier = Arc::new(Barrier::new(8));
        let handles: Vec<_> = (0..8)
            .map(|index| {
                let path = path.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    let bytes = vec![index; 65536];
                    barrier.wait();
                    publish_new(&path, &bytes).is_ok()
                })
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .filter_map(|handle| handle.join().ok())
                .filter(|won| *won)
                .count(),
            1
        );
        let bytes = fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 65536);
        assert!(bytes.iter().all(|byte| *byte == bytes[0]));
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }
    #[test]
    fn failed_publication_leaves_existing_destination_and_no_temporary_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config");
        fs::create_dir(&path).unwrap();
        assert!(publish_new(&path, b"new").is_err());
        assert!(path.is_dir());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }
    #[cfg(unix)]
    #[test]
    fn credential_state_has_private_native_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state/config");
        publish_new(&path, b"private").unwrap();
        assert_eq!(
            fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[cfg(windows)]
    #[test]
    fn credential_descendants_inherit_owner_only_windows_permissions() {
        use std::{ffi::c_void, os::windows::ffi::OsStrExt, ptr};
        #[link(name = "advapi32")]
        extern "system" {
            fn GetFileSecurityW(
                path: *const u16,
                information: u32,
                descriptor: *mut c_void,
                length: u32,
                needed: *mut u32,
            ) -> i32;
            fn ConvertSecurityDescriptorToStringSecurityDescriptorW(
                descriptor: *const c_void,
                revision: u32,
                information: u32,
                text: *mut *mut u16,
                length: *mut u32,
            ) -> i32;
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn LocalFree(memory: *mut c_void) -> *mut c_void;
        }
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("private");
        private_dir(&home).unwrap();
        let nested = home.join("nested");
        fs::create_dir(&nested).unwrap();
        let credential = nested.join("credentials.json");
        fs::write(&credential, b"private").unwrap();
        for (path, directory) in [(&nested, true), (&credential, false)] {
            let filename: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let mut needed = 0u32;
            unsafe {
                GetFileSecurityW(filename.as_ptr(), 4, ptr::null_mut(), 0, &mut needed);
            }
            assert!(needed > 0);
            let mut descriptor = vec![0u32; (needed as usize).div_ceil(4)];
            let mut text = ptr::null_mut();
            unsafe {
                assert_ne!(
                    GetFileSecurityW(
                        filename.as_ptr(),
                        4,
                        descriptor.as_mut_ptr().cast(),
                        (descriptor.len() * 4) as u32,
                        &mut needed
                    ),
                    0
                );
                assert_ne!(
                    ConvertSecurityDescriptorToStringSecurityDescriptorW(
                        descriptor.as_ptr().cast(),
                        1,
                        4,
                        &mut text,
                        ptr::null_mut()
                    ),
                    0
                );
                let mut length = 0;
                while *text.add(length) != 0 {
                    length += 1;
                }
                let sddl = String::from_utf16(std::slice::from_raw_parts(text, length)).unwrap();
                LocalFree(text.cast());
                assert!(
                    owner_only_windows_dacl(&sddl, directory),
                    "{}: {sddl}",
                    path.display()
                );
            }
        }
    }
}
