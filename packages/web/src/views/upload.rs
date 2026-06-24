use api::models::{ImportResult, ImportSource};
use api::{import_csv, preview_csv};
use base64::Engine as _;
use dioxus::prelude::*;

#[component]
pub fn Upload() -> Element {
    let mut source = use_signal(|| ImportSource::Amex);
    // Content of the file the user has selected but not yet uploaded.
    // For CSV sources this is the raw UTF-8 text; for Klarna it is base64-encoded PDF bytes.
    let mut pending_content: Signal<Option<String>> = use_signal(|| None);
    // Preview counts returned by `preview_csv` (before the real import).
    let mut preview: Signal<Option<ImportResult>> = use_signal(|| None);
    // Actual import counts returned by `import_csv` after confirmation.
    let mut result: Signal<Option<ImportResult>> = use_signal(|| None);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut loading = use_signal(|| false);
    let mut loading_msg: Signal<&'static str> = use_signal(|| "");

    // When the source dropdown changes, update the source and re-run the preview
    // if a file has already been loaded, so the counts reflect the new format.
    let on_source_change = move |evt: Event<FormData>| async move {
        result.set(None);
        error.set(None);
        let new_source = match evt.value().as_str() {
            "nordea" => ImportSource::Nordea,
            "klarna" => ImportSource::Klarna,
            _ => ImportSource::Amex,
        };
        source.set(new_source.clone());

        if let Some(content) = pending_content() {
            preview.set(None);
            loading.set(true);
            loading_msg.set("Analysing…");
            match preview_csv(new_source, content).await {
                Ok(p) => preview.set(Some(p)),
                Err(e) => {
                    error.set(Some(e.to_string()));
                    pending_content.set(None);
                }
            }
            loading.set(false);
        }
    };

    // File selected: read its content, then call preview_csv to show counts
    // without touching the database yet.
    let on_file_change = move |evt: Event<FormData>| async move {
        error.set(None);
        preview.set(None);
        result.set(None);
        pending_content.set(None);

        let files = evt.files();
        let file = match files.into_iter().next() {
            Some(f) => f,
            None => return,
        };

        // Klarna PDFs are binary — read as bytes and base64-encode for transport.
        // All other sources are UTF-8 CSV text — read directly as a string.
        let content = if source() == ImportSource::Klarna {
            match file.read_bytes().await {
                Ok(bytes) => base64::engine::general_purpose::STANDARD.encode(&bytes),
                Err(e) => {
                    error.set(Some(format!("Could not read file: {e}")));
                    return;
                }
            }
        } else {
            match file.read_string().await {
                Ok(c) => c,
                Err(e) => {
                    error.set(Some(format!("Could not read file: {e}")));
                    return;
                }
            }
        };

        // Store the content before attempting preview so that if parsing fails
        // for the wrong source, switching source can re-analyse the same file.
        pending_content.set(Some(content.clone()));
        loading.set(true);
        loading_msg.set("Analysing…");
        match preview_csv(source(), content).await {
            Ok(p) => preview.set(Some(p)),
            Err(e) => error.set(Some(e.to_string())),
        }
        loading.set(false);
    };

    // Upload button: commit the already-read file content to the database.
    let on_upload = move |_| async move {
        let content = match pending_content.write().take() {
            Some(c) => c,
            None => return,
        };
        error.set(None);
        preview.set(None);
        loading.set(true);
        loading_msg.set("Importing…");
        match import_csv(source(), content).await {
            Ok(r) => result.set(Some(r)),
            Err(e) => error.set(Some(e.to_string())),
        }
        loading.set(false);
    };

    // File input accept attribute depends on the selected source.
    let file_accept = if source() == ImportSource::Klarna {
        "application/pdf,.pdf"
    } else {
        ".csv,text/csv"
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
                        onchange: on_source_change,
                        option { value: "amex",   "American Express (Amex)" }
                        option { value: "nordea", "Nordea" }
                        option { value: "klarna", "Klarna (Monthly invoice PDF)" }
                    }
                }

                // File picker — triggers a preview (not the import) on selection
                div {
                    label { class: "form-label", "File" }
                    input {
                        r#type: "file",
                        accept: file_accept,
                        disabled: loading(),
                        class: "input-std input-std--full",
                        onchange: on_file_change,
                    }
                    p { class: "field-hint",
                        if preview().is_some() {
                            "File ready — review the preview below and click Upload."
                        } else if source() == ImportSource::Klarna {
                            "Select the Klarna Monthly invoice PDF to preview what will be imported."
                        } else {
                            "Select a CSV file to preview what will be imported."
                        }
                    }
                }

                if loading() {
                    p { class: "text-loading", "{loading_msg}" }
                }
            }

            if let Some(e) = error() {
                p { class: "form-error", style: "margin-top: 16px;", "{e}" }
            }

            // Preview box — shown after file selection, before the user confirms
            if let Some(p) = preview() {
                div {
                    class: "upload-preview",
                    p { class: "upload-preview__title", "Ready to import" }
                    ul {
                        class: "upload-preview__list",
                        li { "{p.imported} new transactions" }
                        li { "{p.skipped} duplicates (already in database)" }
                        if p.pending > 0 {
                            li { "{p.pending} pending (no date)" }
                        }
                    }
                }
            }

            // Success box — shown after a successful import
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

            button {
                class: "btn-primary",
                style: "margin-top: 20px;",
                disabled: loading() || pending_content().is_none(),
                onclick: on_upload,
                "Upload"
            }
        }
    }
}
