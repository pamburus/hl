/// Ends the process with `code`, as [`std::process::exit`] does.
///
/// This is the only place the process is ended from, so that what must be done before that is done once.
pub fn exit(code: i32) -> ! {
    // On Windows, the coverage data that is written as the process ends is lost when it is ended from a thread other
    // than the main one, such as the one that finds the output closed, so it is written here. The `coverage` option is
    // set by the coverage build.
    #[cfg(all(windows, coverage))]
    {
        unsafe extern "C" {
            fn __llvm_profile_write_file() -> std::ffi::c_int;
        }
        // SAFETY: the profiler runtime that defines it is linked into a build with coverage instrumentation.
        unsafe { __llvm_profile_write_file() };
    }

    #[allow(clippy::disallowed_methods)]
    std::process::exit(code)
}
