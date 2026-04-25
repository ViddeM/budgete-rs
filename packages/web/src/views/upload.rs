use api::import_csv;
use api::models::{CsvSource, ImportResult};
use dioxus::prelude::*;

#[component]
pub fn Upload() -> Element {
    let mut source = use_signal(|| CsvSource::Amex);
    let mut result: Signal<Option<ImportResult>> = use_signal(|| None);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut loading = use_signal(|| false);

    let on_file_change = move |evt: Event<FormData>| async move {
        error.set(None);
        result.set(None);

        let files = evt.files();
        let file = match files.into_iter().next() {
            Some(f) => f,
            None => {
                error.set(Some("No file selected".to_string()));
                return;
            }
        };

        let content = match file.read_string().await {
            Ok(c) => c,
            Err(e) => {
                error.set(Some(format!("Could not read file: {e}")));
                return;
            }
        };

        loading.set(true);
        match import_csv(source(), content).await {
            Ok(r) => result.set(Some(r)),
            Err(e) => error.set(Some(e.to_string())),
        }
        loading.set(false);
    };

    rsx! {
        div {
            style: "padding: 32px; font-family: sans-serif; max-width: 560px;",
            h1 { style: "margin: 0 0 24px; font-size: 1.5rem; color: #111827;", "Upload transactions" }

            div {
                style: "display: flex; flex-direction: column; gap: 16px;",

                // Source selector
                div {
                    label {
                        style: "display: block; font-size: 0.85rem; font-weight: 600; color: #374151; margin-bottom: 4px;",
                        "Bank / Source"
                    }
                    select {
                        style: "width: 100%; padding: 8px 10px; border: 1px solid #d1d5db; border-radius: 8px; font-size: 0.9rem; color: #111827;",
                        onchange: move |evt: Event<FormData>| {
                            source.set(match evt.value().as_str() {
                                "nordea" => CsvSource::Nordea,
                                _ => CsvSource::Amex,
                            });
                        },
                        option { value: "amex",   "American Express (Amex)" }
                        option { value: "nordea", "Nordea" }
                    }
                }

                // File picker — triggers import on file selection
                div {
                    label {
                        style: "display: block; font-size: 0.85rem; font-weight: 600; color: #374151; margin-bottom: 4px;",
                        "CSV file"
                    }
                    input {
                        r#type: "file",
                        accept: ".csv,text/csv",
                        disabled: loading(),
                        style: "width: 100%; font-size: 0.9rem;",
                        onchange: on_file_change,
                    }
                    p {
                        style: "margin: 4px 0 0; font-size: 0.78rem; color: #9ca3af;",
                        "Selecting a file will start the import automatically."
                    }
                }

                if loading() {
                    p { style: "color: #6366f1; font-weight: 600;", "Importing…" }
                }
            }

            if let Some(e) = error() {
                p { style: "margin-top: 16px; color: #dc2626;", "{e}" }
            }

            if let Some(r) = result() {
                div {
                    style: "margin-top: 20px; padding: 16px; background: #f0fdf4; border: 1px solid #bbf7d0; border-radius: 8px;",
                    p { style: "margin: 0; font-weight: 600; color: #166534;", "Import complete" }
                    ul {
                        style: "margin: 8px 0 0; padding-left: 20px; color: #166534; font-size: 0.9rem;",
                        li { "{r.imported} transactions imported" }
                        li { "{r.skipped} duplicates skipped" }
                        li { "{r.pending} pending (no date)" }
                    }
                }
            }
        }
    }
}
