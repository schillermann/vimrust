use std::env;

fn main() -> std::io::Result<()> {
    let arg_path = env::args().skip(1).next();
    let file_path = match arg_path {
        Some(path) => vimrust_protocol::FilePath::Provided { path },
        None => vimrust_protocol::FilePath::Missing,
    };
    vimrust_core::serve_stdio(file_path)
}
