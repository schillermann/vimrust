use rustpages::{App, Output, Page, TextPage};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct UiPage {
    state: Arc<Mutex<String>>,
    path: String,
    query: String,
}

impl UiPage {
    fn new(state: Arc<Mutex<String>>) -> Self {
        Self {
            state,
            path: String::new(),
            query: String::new(),
        }
    }

    fn with_path(&self, path: &str) -> Self {
        let mut next = self.clone();
        next.path = path.to_string();
        next
    }

    fn with_query(&self, query: &str) -> Self {
        let mut next = self.clone();
        next.query = query.to_string();
        next
    }
}

impl Page for UiPage {
    fn with(&self, key: &str, value: &str) -> Box<dyn Page> {
        match key {
            "RustPages-Path" => Box::new(self.with_path(value)),
            "RustPages-Query" => Box::new(self.with_query(value)),
            _ => Box::new(self.clone()),
        }
    }

    fn via(&self, output: Box<dyn Output>) -> Box<dyn Output> {
        match self.path.as_str() {
            "/state" => {
                let buf = self.state.lock().unwrap().clone();
                output.with("RustPages-Body", &buf)
            }
            "/cmd" => {
                if let Some((_, text)) = self.query.split_once("insert=") {
                    self.state.lock().unwrap().push_str(text);
                }
                output.with("RustPages-Body", "ok")
            }
            _ => TextPage::new("not found").via(output),
        }
    }
}

fn main() -> std::io::Result<()> {
    let state = Arc::new(Mutex::new(String::new()));
    App::new(Box::new(UiPage::new(state))).start(8080)
}
