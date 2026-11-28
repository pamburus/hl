use std::{process::Child, time::Duration};

/// Returns the CPU time `child` has used so far, in user and system mode together.
pub fn cpu_time(child: &Child) -> Duration {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", child.id())).unwrap();
    // The name of the process, in parentheses, may contain spaces, so the fields are counted after it, from the state,
    // which is field 3; user time is field 14 and system time field 15, in clock ticks.
    let fields: Vec<&str> = stat[stat.rfind(')').unwrap() + 1..].split_whitespace().collect();
    let ticks: u64 = fields[11].parse::<u64>().unwrap() + fields[12].parse::<u64>().unwrap();
    // SAFETY: `sysconf` takes no pointers.
    let per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
    Duration::from_secs_f64(ticks as f64 / per_second as f64)
}
