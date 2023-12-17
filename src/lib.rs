pub use codespan_derive_proc::IntoDiagnostic;
pub use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle, Severity};

pub trait IntoDiagnostic {
    type FileId<'a>: 'a
    where
        Self: 'a;

    fn into_diagnostic(&self) -> Diagnostic<Self::FileId<'_>>;
}

pub trait IntoLabels {
    type FileId<'a>: 'a
    where
        Self: 'a;

    fn into_labels(&self, style: LabelStyle) -> Vec<Label<Self::FileId<'_>>>;
}

/// Impl for optional labels
impl<T: IntoLabels> IntoLabels for Option<T> {
    type FileId<'a> = T::FileId<'a> where T: 'a;

    fn into_labels(&self, style: LabelStyle) -> Vec<Label<Self::FileId<'_>>> {
        self.iter()
            .flat_map(|x| x.into_labels(style))
            .collect()
    }
}

/// Impl for multiple labels
impl<T: IntoLabels> IntoLabels for Vec<T> {
    type FileId<'a> = T::FileId<'a> where T: 'a;

    fn into_labels(&self, style: LabelStyle) -> Vec<Label<Self::FileId<'_>>> {
        self.iter()
            .flat_map(|x| x.into_labels(style))
            .collect()
    }
}
