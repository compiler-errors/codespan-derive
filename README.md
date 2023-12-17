# codespan-derive

Derive macro for ergonomically creating a Diagnostic from an error macro

## Usage

1. Add `#[derive(IntoDiagnostic)]` onto your error macro type.
2. Add a `#[file_id(Type)]` to signal what the `FileId` generic type of the `Diagnostic` will be. If your `FileId` type requires a lifetime, you can use `'a`.
3. Add a `#[severity(Ident)]` to denote the severity of the diagnostic. `Ident` should be one of the variants of `codespan_reporting::Severity`.
4. Tag every variant with a `#[message = ""]` signalling what the error message should read.
5. Span-like values that implement `IntoLabel` can be tagged with `#[primary]` or `#[secondary]` to be marked in the generated error, with an optional message like `#[primary = ""]`. Optional labels can be specified with `Option<T>`, and varying lists of labels can be specified with `Vec<T>`.

```rust
#[derive(IntoDiagnostic)]
#[file_id(SomeFileIdType)]
#[severity(Error)]
enum Error {
  #[message = "Compiler found the number `{0}` is too large"]
  NumberTooLarge(usize),

  #[message = "Cannot parse string {string}"]
  BadString {
    string: String,
    #[primary = "The bad string appears here"]
    span: Span,
    #[secondary = "The bad string also potentially appears here"]
    span_2: Option<Span>,
  },
}
```

Then handle it somewhere like:

```rust
if let Some(err) = result {
  // IntoDiagnostic derived from macro
  let diagnostic = err.into_diagnostic();

  // Basic codespan-diagnostic printing to terminal
  let writer = StandardStream::stderr(ColorChoice::Always);
  let config = codespan_reporting::term::Config::default();
  term::emit(&mut writer.lock(), &config, &files, &diagnostic)?;
}
```
