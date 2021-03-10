pub use proc::IntoDiagnostic;

pub mod __reexport {
    pub use codespan_reporting::diagnostic::{Diagnostic, Label};
}

pub trait IntoDiagnostic {
    type FileId;

    fn into_diagnostic(&self) -> __reexport::Diagnostic<Self::FileId>;
}

pub trait IntoLabel {
    type FileId;

    fn into_label(&self) -> __reexport::Label<Self::FileId>;
}
