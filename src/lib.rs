pub use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle};
pub use proc::IntoDiagnostic;

pub trait IntoDiagnostic {
    type FileId;

    fn into_diagnostic(&self) -> Diagnostic<Self::FileId>;
}

pub trait IntoLabel {
    type FileId;

    fn into_label(&self, style: LabelStyle) -> Label<Self::FileId>;
}
