use std::{
    process::{Child, Command},
    time::Duration,
};

/// Returns the CPU time `child` has used so far, in user and system mode together.
pub fn cpu_time(child: &Child) -> Duration {
    let output = Command::new("ps")
        .args(["-o", "time=", "-p", &child.id().to_string()])
        .output()
        .unwrap();
    // Printed as `[hours:]minutes:seconds.hundredths`.
    let time = String::from_utf8(output.stdout).unwrap();
    let seconds = time
        .trim()
        .split(':')
        .fold(0.0, |total, part| total * 60.0 + part.parse::<f64>().unwrap());
    Duration::from_secs_f64(seconds)
}
