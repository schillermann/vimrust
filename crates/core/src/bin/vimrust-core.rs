use std::env;

fn main() -> std::io::Result<()> {
    let arg_path = env::args().skip(1).next();
    let file_path = match arg_path {
        Some(path) => vimrust_protocol::FilePath::Provided { path },
        None => vimrust_protocol::FilePath::Missing,
    };
    let mut session = vimrust_core::StdioSession::new(file_path);
    session.open()
}
