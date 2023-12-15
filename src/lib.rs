pub use codespan_derive_proc::IntoDiagnostic;
pub use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle};

pub trait IntoDiagnostic {
    type FileId<'a>: 'a
    where
        Self: 'a;

    fn into_diagnostic(&self) -> Diagnostic<Self::FileId<'_>>;
}

pub trait IntoLabel {
    type FileId<'a>: 'a
    where
        Self: 'a;

    fn into_label(&self, style: LabelStyle) -> Label<Self::FileId<'_>>;
}
