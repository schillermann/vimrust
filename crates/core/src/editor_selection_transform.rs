pub(super) trait SelectionTransform {
    fn transform(&self, input: &str) -> String;
}

pub(super) struct KebabCaseTransform;

impl SelectionTransform for KebabCaseTransform {
    fn transform(&self, input: &str) -> String {
        let mut output = String::new();
        let mut prev_separator = true;

        for ch in input.chars() {
            if ch.is_ascii_alphanumeric() {
                if !output.is_empty() && prev_separator {
                    output.push('-');
                }
                output.push(ch.to_ascii_lowercase());
                prev_separator = false;
            } else {
                prev_separator = true;
            }
        }

        output
    }
}
