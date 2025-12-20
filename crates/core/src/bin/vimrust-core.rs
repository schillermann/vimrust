use std::env;

fn main() -> std::io::Result<()> {
    let file_path = env::args().skip(1).next();
    vimrust_core::serve_stdio(file_path)
}
