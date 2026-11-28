/// The standard output is never watched here, as there is no known way to wait for its reader to close it.
pub fn watchable() -> bool {
    false
}

pub fn wait_closed() -> bool {
    false
}
