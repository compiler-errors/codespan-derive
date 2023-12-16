use std::ops::Range;

use codespan_derive::{IntoDiagnostic, IntoLabel, Label, LabelStyle};
use codespan_reporting::{
    files::{self, Files, SimpleFile},
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};

/// Example implementation of `Files` that treats a &str path as the file ID
///
/// For the sake of brevity, this implementation returns the same file regardless
/// of ID
struct ExampleFiles {
    file: SimpleFile<String, String>,
}

impl<'a> Files<'a> for ExampleFiles {
    /// The file ID for this Files impl is a reference (eg. for an FS path)
    type FileId = &'a str;
    type Name = &'a str;
    type Source = &'a str;

    fn name(&'a self, id: Self::FileId) -> Result<Self::Name, files::Error> {
        Ok(id)
    }

    fn source(&'a self, _: Self::FileId) -> Result<Self::Source, files::Error> {
        Ok(self.file.source())
    }

    fn line_index(&'a self, _: Self::FileId, byte_index: usize) -> Result<usize, files::Error> {
        self.file.line_index((), byte_index)
    }

    fn line_range(
        &'a self,
        _: Self::FileId,
        line_index: usize,
    ) -> Result<Range<usize>, files::Error> {
        self.file.line_range((), line_index)
    }
}

/// This span owns its file ID rather than keep a reference
struct Span {
    file_id: String,
    range: Range<usize>,
}

impl IntoLabel for Span {
    type FileId<'a> = &'a str;

    fn into_label(&self, style: LabelStyle) -> Label<Self::FileId<'_>> {
        Label::new(style, self.file_id.as_str(), self.range.clone())
    }
}

#[derive(IntoDiagnostic)]
// codespan-derive provides a lifetime argument 'a for reference-type file IDs
#[file_id(&'a str)]
#[severity(Error)]
enum Error {
    #[message = "This is an error: {message}"]
    Example {
        message: &'static str,

        #[primary = "This is a primary span"]
        primary_span: Span,

        #[secondary = "This is a secondary span"]
        secondary_span: Span,
    },
}

fn main() {
    let files = ExampleFiles {
        file: SimpleFile::new("empty.txt".into(), "Test Case".into()),
    };

    let err = Error::Example {
        message: "This is a stored message",
        primary_span: Span {
            file_id: "example1.txt".to_string(),
            range: 5..9,
        },
        secondary_span: Span {
            file_id: "example2.txt".to_string(),
            range: 0..4,
        },
    };

    // Basic codespan-diagnostic printing to terminal
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();
    term::emit(&mut writer.lock(), &config, &files, &err.into_diagnostic())
        .expect("Failed to show diagnostic");
}
