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
            class: "view view--narrow",
            h1 { class: "view__title", "Upload transactions" }

            div {
                style: "display: flex; flex-direction: column; gap: 16px;",

                // Source selector
                div {
                    label { class: "form-label", "Bank / Source" }
                    select {
                        class: "input-std input-std--full",
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
                    label { class: "form-label", "CSV file" }
                    input {
                        r#type: "file",
                        accept: ".csv,text/csv",
                        disabled: loading(),
                        class: "input-std input-std--full",
                        onchange: on_file_change,
                    }
                    p { class: "field-hint", "Selecting a file will start the import automatically." }
                }

                if loading() {
                    p { class: "text-loading", "Importing…" }
                }
            }

            if let Some(e) = error() {
                p { class: "form-error", style: "margin-top: 16px;", "{e}" }
            }

            if let Some(r) = result() {
                div {
                    class: "upload-result",
                    p { class: "upload-result__title", "Import complete" }
                    ul {
                        class: "upload-result__list",
                        li { "{r.imported} transactions imported" }
                        li { "{r.skipped} duplicates skipped" }
                        li { "{r.pending} pending (no date)" }
                    }
                }
            }
        }
    }
}
