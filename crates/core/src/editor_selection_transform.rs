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

pub(super) struct SnakeCaseTransform;

impl SelectionTransform for SnakeCaseTransform {
    fn transform(&self, input: &str) -> String {
        let mut output = String::new();
        let mut prev_separator = true;

        for ch in input.chars() {
            if ch.is_ascii_alphanumeric() {
                if !output.is_empty() && prev_separator {
                    output.push('_');
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

pub(super) struct ScreamingSnakeCaseTransform;

impl SelectionTransform for ScreamingSnakeCaseTransform {
    fn transform(&self, input: &str) -> String {
        let mut output = String::new();
        let mut prev_separator = true;

        for ch in input.chars() {
            if ch.is_ascii_alphanumeric() {
                if !output.is_empty() && prev_separator {
                    output.push('_');
                }
                output.push(ch.to_ascii_uppercase());
                prev_separator = false;
            } else {
                prev_separator = true;
            }
        }

        output
    }
}

pub(super) struct CamelCaseTransform;

impl SelectionTransform for CamelCaseTransform {
    fn transform(&self, input: &str) -> String {
        let mut output = String::new();
        let mut prev_separator = true;

        for ch in input.chars() {
            if ch.is_ascii_alphanumeric() {
                if output.is_empty() {
                    output.push(ch.to_ascii_lowercase());
                } else if prev_separator {
                    output.push(ch.to_ascii_uppercase());
                } else {
                    output.push(ch.to_ascii_lowercase());
                }
                prev_separator = false;
            } else {
                prev_separator = true;
            }
        }

        output
    }
}

pub(super) struct PascalCaseTransform;

impl SelectionTransform for PascalCaseTransform {
    fn transform(&self, input: &str) -> String {
        let mut output = String::new();
        let mut prev_separator = true;

        for ch in input.chars() {
            if ch.is_ascii_alphanumeric() {
                if output.is_empty() {
                    output.push(ch.to_ascii_uppercase());
                } else if prev_separator {
                    output.push(ch.to_ascii_uppercase());
                } else {
                    output.push(ch.to_ascii_lowercase());
                }
                prev_separator = false;
            } else {
                prev_separator = true;
            }
        }

        output
    }
}

pub(super) struct TrainCaseTransform;

impl SelectionTransform for TrainCaseTransform {
    fn transform(&self, input: &str) -> String {
        let mut output = String::new();
        let mut prev_separator = true;

        for ch in input.chars() {
            if ch.is_ascii_alphanumeric() {
                if !output.is_empty() && prev_separator {
                    output.push('-');
                }
                if prev_separator || output.is_empty() {
                    output.push(ch.to_ascii_uppercase());
                } else {
                    output.push(ch.to_ascii_lowercase());
                }
                prev_separator = false;
            } else {
                prev_separator = true;
            }
        }

        output
    }
}
