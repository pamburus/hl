use thiserror::Error;

/// ConsoleError is an error which may occur in initialization of windows console.
#[derive(Error, Debug)]
pub enum ConsoleError {
    #[error("failed to get standard output handle: {0}")]
    FailedToGetStandardOutputHandle(std::io::Error),
    #[error("failed to set console mode: {0}")]
    FailedToSetConsoleMode(std::io::Error),
}

#[cfg(windows)]
pub fn enable_ansi_support() -> Result<(), ConsoleError> {
    use winapi_util::console::Console;

    let mut console = Console::stdout().map_err(ConsoleError::FailedToGetStandardOutputHandle)?;
    console.set_virtual_terminal_processing(true).map_err(ConsoleError::FailedToSetConsoleMode)?;

    Ok(())
}

#[cfg(not(windows))]
pub fn enable_ansi_support() -> Result<(), ConsoleError> {
    Ok(())
}
