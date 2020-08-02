use error_chain::error_chain;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        ParseIntError(::std::num::ParseIntError);
        Boxed(::std::boxed::Box<dyn std::error::Error + std::marker::Send>);
    }

    errors {
        FileNotFoundError(filename: String) {
            description("file not found"),
            display("file {:?} not found", filename)
        }
    }
}
