use std::path::Path;

use vimrust_protocol::DocumentFile;

pub trait File {
    fn append_to(&self, target: &mut String);
}

pub trait UiFileSource {
    fn file(&self) -> Box<dyn File>;
}

impl UiFileSource for DocumentFile {
    fn file(&self) -> Box<dyn File> {
        if self.path.is_empty() {
            Box::new(NoFile)
        } else {
            let base = Box::new(TextFile {
                path: self.path.clone(),
            }) as Box<dyn File>;
            MarkdownPath {
                path: self.path.as_str(),
            }
            .decorate(base)
        }
    }
}

pub struct TextFile {
    path: String,
}

impl File for TextFile {
    fn append_to(&self, target: &mut String) {
        target.push_str(self.path.as_str());
    }
}

pub struct NoFile;

impl File for NoFile {
    fn append_to(&self, target: &mut String) {
        target.push_str("[No Filename]");
    }
}

pub struct MarkdownFile {
    inner: Box<dyn File>,
}

impl File for MarkdownFile {
    fn append_to(&self, target: &mut String) {
        self.inner.append_to(target);
    }
}

struct MarkdownPath<'a> {
    path: &'a str,
}

impl<'a> MarkdownPath<'a> {
    fn decorate(&self, inner: Box<dyn File>) -> Box<dyn File> {
        let ext = Path::new(self.path).extension();
        if let Some(ext) = ext {
            if let Some(text) = ext.to_str() {
                let lower = text.to_ascii_lowercase();
                if lower == "md" || lower == "markdown" {
                    return Box::new(MarkdownFile { inner });
                }
            }
        }
        inner
    }
}
