pub use codespan_derive_proc::IntoDiagnostic;
pub use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle};

pub trait IntoDiagnostic {
    type FileId;

    fn into_diagnostic(&self) -> Diagnostic<Self::FileId>;
}

pub trait IntoLabel {
    type FileId;

    fn into_label(&self, style: LabelStyle) -> Label<Self::FileId>;
}
