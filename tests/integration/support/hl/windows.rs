use std::{os::windows::io::AsRawHandle, process::Child, time::Duration};

use windows_sys::Win32::{Foundation::FILETIME, System::Threading::GetProcessTimes};

/// Returns the CPU time `child` has used so far, in user and system mode together.
pub fn cpu_time(child: &Child) -> Duration {
    let mut times = [FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    }; 4];
    let [creation, exit, kernel, user] = &mut times;
    // SAFETY: the handle is of a child that has not been waited for, and each time is written to a `FILETIME`.
    assert_ne!(
        unsafe { GetProcessTimes(child.as_raw_handle(), creation, exit, kernel, user) },
        0
    );
    // Counted in units of 100 nanoseconds.
    let units = |time: &FILETIME| u64::from(time.dwHighDateTime) << 32 | u64::from(time.dwLowDateTime);
    Duration::from_nanos((units(kernel) + units(user)) * 100)
}
