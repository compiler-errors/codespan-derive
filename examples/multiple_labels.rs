use std::ops::Range;

use codespan_derive::{IntoDiagnostic, IntoLabels, Label, LabelStyle};
use codespan_reporting::{
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};

/// A source span to store a `file:byte range`
struct Span {
    file_id: usize,
    range: Range<usize>,
}

impl IntoLabels for Span {
    type FileId<'a> = usize;

    fn into_labels(&self, style: LabelStyle) -> Vec<Label<Self::FileId<'_>>> {
        vec![Label::new(style, self.file_id, self.range.clone())]
    }
}

#[derive(IntoDiagnostic)]
#[severity(Error)]
#[file_id(usize)]
enum Error {
    #[message = "This is an error: {message}"]
    Example {
        message: &'static str,

        #[primary = "This is a mandatory span"]
        span: Span,

        #[secondary = "This is an optional span"]
        optional_span: Option<Span>,

        #[secondary = "These are multiple spans"]
        multi_span: Vec<Span>,
    },
}

fn main() {
    let mut files: SimpleFiles<&'static str, &'static str> = SimpleFiles::new();
    let file_id = files.add("example.txt", "Test Case");
    let file_id_2 = files.add("example2.txt", "Test Case 2");

    let err = Error::Example {
        message: "This is a stored message",
        span: Span {
            file_id,
            range: 5..9,
        },
        optional_span: None,
        multi_span: vec![
            Span {
                file_id,
                range: 0..1,
            },
            Span {
                file_id: file_id_2,
                range: 1..2,
            },
        ]
    };

    // Basic codespan-diagnostic printing to terminal
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();
    term::emit(&mut writer.lock(), &config, &files, &err.into_diagnostic())
        .expect("Failed to show diagnostic");
}
