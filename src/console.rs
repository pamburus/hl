use thiserror::Error;

/// ConsoleError is an error which may occur in initialization of windows console.
#[derive(Error, Debug)]
pub enum ConsoleError {
    #[error("failed to get standard output handle: error {0}")]
    FailedToGetStandardOutputHandle(u32),
    #[error("failed to get console mode: error {0}")]
    FailedToGetConsoleMode(u32),
    #[error("failed to set console mode: error {0}")]
    FailedToSetConsoleMode(u32),
}

#[cfg(windows)]
pub fn enable_ansi_support() -> Result<(), ConsoleError> {
    use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::STD_OUTPUT_HANDLE;

    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    unsafe {
        let std_out_handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if std_out_handle == INVALID_HANDLE_VALUE {
            return Err(ConsoleError::FailedToGetStandardOutputHandle(GetLastError()));
        }
        let mut console_mode: u32 = 0;
        if GetConsoleMode(std_out_handle, &mut console_mode) == 0 {
            return Err(ConsoleError::FailedToGetConsoleMode(GetLastError()));
        }

        if console_mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING == 0 {
            if SetConsoleMode(std_out_handle, console_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) == 0 {
                return Err(ConsoleError::FailedToSetConsoleMode(GetLastError()));
            }
        }
    }

    return Ok(());
}

#[cfg(not(windows))]
pub fn enable_ansi_support() -> Result<(), ConsoleError> {
    return Ok(());
}
