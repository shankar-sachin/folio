//! The stroke icon set shared by the reader and the workbench (24×24, currentColor).

pub fn path(name: &str) -> &'static str {
    match name {
        "files" => r#"<path d="M9 7V5.5A2.5 2.5 0 0 1 11.5 3H16l4 4v9.5a2.5 2.5 0 0 1-2.5 2.5H16"/><path d="M4 10.5A2.5 2.5 0 0 1 6.5 8H11l4 4v6.5a2.5 2.5 0 0 1-2.5 2.5h-6A2.5 2.5 0 0 1 4 18.5z"/>"#,
        "search" => r#"<circle cx="11" cy="11" r="6.5"/><path d="m20 20-4.2-4.2"/>"#,
        "outline" => r#"<path d="M4 6h16M8 12h12M12 18h8"/><circle cx="4.5" cy="12" r=".6"/><circle cx="8.5" cy="18" r=".6"/>"#,
        "layers" => r#"<path d="m12 3 9 4.8-9 4.8-9-4.8z"/><path d="m3 12.2 9 4.8 9-4.8"/><path d="m3 16.6 9 4.8 9-4.8" opacity=".5"/>"#,
        "settings" => r#"<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2.2"/><circle cx="9" cy="17" r="2.2"/>"#,
        "sun" => r#"<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2M12 19.5v2M4.6 4.6 6 6M18 18l1.4 1.4M2.5 12h2M19.5 12h2M4.6 19.4 6 18M18 6l1.4-1.4"/>"#,
        "moon" => r#"<path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a7 7 0 0 0 10.5 10.5z"/>"#,
        "chevron" => r#"<path d="m9.5 6 6 6-6 6"/>"#,
        "chevronDown" => r#"<path d="m6 9.5 6 6 6-6"/>"#,
        "folder" => r#"<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5v7a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 16.5z"/>"#,
        "folderOpen" => r#"<path d="M3 17V7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h6.5A2.5 2.5 0 0 1 20 9.5V10"/><path d="M3 17.5 5.6 11a1.6 1.6 0 0 1 1.5-1H21l-2.8 7.5a2.3 2.3 0 0 1-2.1 1.5H5a2 2 0 0 1-2-1.5z"/>"#,
        "file" => r#"<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/>"#,
        "md" => r#"<rect x="2.5" y="5.5" width="19" height="13" rx="2.5"/><path d="M6 15V9l2.5 3L11 9v6M16 9v6M13.8 13l2.2 2.2 2.2-2.2"/>"#,
        "x" => r#"<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>"#,
        "plus" => r#"<path d="M12 5v14M5 12h14"/>"#,
        "filePlus" => r#"<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/><path d="M12 11.5v6M9 14.5h6"/>"#,
        "folderPlus" => r#"<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5v7a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 16.5z"/><path d="M12 9.8v6M9 12.8h6"/>"#,
        "refresh" => r#"<path d="M20 12a8 8 0 1 1-2.6-5.9"/><path d="M20 4v5h-5"/>"#,
        "collapse" => r#"<rect x="4" y="4" width="16" height="16" rx="3"/><path d="M8.5 12h7"/>"#,
        "book" => r#"<path d="M3 5.5h6A3 3 0 0 1 12 8.5V20a2.3 2.3 0 0 0-2.3-2.3H3z"/><path d="M21 5.5h-6a3 3 0 0 0-3 3V20a2.3 2.3 0 0 1 2.3-2.3H21z"/>"#,
        "columns" => r#"<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M12 4v16"/>"#,
        "code" => r#"<path d="m8.5 8-4 4 4 4M15.5 8l4 4-4 4"/>"#,
        "more" => r#"<circle cx="5.5" cy="12" r="1.1"/><circle cx="12" cy="12" r="1.1"/><circle cx="18.5" cy="12" r="1.1"/>"#,
        "sidebar" => r#"<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M9.5 4v16"/>"#,
        "sidebarFill" => r#"<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M9.5 4v16"/><path d="M3.8 5.2h5v13.6h-5z" fill="currentColor" stroke="none" opacity=".35"/>"#,
        "up" => r#"<path d="M12 19V5M6 11l6-6 6 6"/>"#,
        "down" => r#"<path d="M12 5v14M6 13l6 6 6-6"/>"#,
        "copy" => r#"<rect x="8.5" y="8.5" width="12" height="12" rx="2.5"/><path d="M15.5 8.5V6a2.5 2.5 0 0 0-2.5-2.5H6A2.5 2.5 0 0 0 3.5 6v7A2.5 2.5 0 0 0 6 15.5h2.5"/>"#,
        "check" => r#"<path d="m5 12.5 4.5 4.5L19 7.5"/>"#,
        "link" => r#"<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>"#,
        "trash" => r#"<path d="M4 7h16M10 11v6M14 11v6M6 7l1 12.5A1.6 1.6 0 0 0 8.6 21h6.8a1.6 1.6 0 0 0 1.6-1.5L18 7M9 7V4h6v3"/>"#,
        "edit" => r#"<path d="M4 20h4L19.5 8.5a2.1 2.1 0 0 0-3-3L5 17z"/>"#,
        "hash" => r#"<path d="M5 9h14M5 15h14M10 4 8 20M16 4l-2 16"/>"#,
        "command" => r#"<path d="M9 6.5A2.5 2.5 0 1 0 6.5 9H17.5A2.5 2.5 0 1 0 15 6.5v11a2.5 2.5 0 1 0 2.5-2.5h-11A2.5 2.5 0 1 0 9 17.5z"/>"#,
        "keyboard" => r#"<rect x="2.5" y="6" width="19" height="12" rx="2.5"/><path d="M6.5 10h.01M10 10h.01M14 10h.01M17.5 10h.01M7.5 14h9"/>"#,
        "zen" => r#"<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>"#,
        "dot" => r#"<circle cx="12" cy="12" r="4" fill="currentColor" stroke="none"/>"#,
        "warning" => r#"<path d="M12 4 2.8 19.5h18.4z"/><path d="M12 10v4.2M12 17h.01"/>"#,
        "info" => r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v5.5M12 7.8h.01"/>"#,
        "bulb" => r#"<path d="M9 18h6M10 21h4M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2V16h5v-.1c0-.8.4-1.5 1-2A6 6 0 0 0 12 3z"/>"#,
        "alert" => r#"<circle cx="12" cy="12" r="9"/><path d="M12 7.5v5.5M12 16.4h.01"/>"#,
        "shield" => r#"<path d="M12 3 4.5 6v5.5c0 4.6 3.2 8.4 7.5 9.5 4.3-1.1 7.5-4.9 7.5-9.5V6z"/>"#,
        "history" => r#"<path d="M3.5 12a8.5 8.5 0 1 0 2.5-6"/><path d="M3.5 4v4h4M12 7.5V12l3 2"/>"#,
        "export" => r#"<path d="M12 3v12M7 8l5-5 5 5"/><path d="M5 14v4.5A2.5 2.5 0 0 0 7.5 21h9a2.5 2.5 0 0 0 2.5-2.5V14"/>"#,
        "finder" => r#"<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M12 4c-1.5 3-1.8 7-1 10.5M8 9v1.5M16 9v1.5M8 15c2.4 1.6 5.6 1.6 8 0"/>"#,
        "wrap" => r#"<path d="M4 6h16M4 12h13a3 3 0 0 1 0 6h-4M4 18h5"/><path d="m14.5 16-2 2 2 2"/>"#,
        _ => "",
    }
}

/// `<svg class="i …">` markup for an icon.
pub fn icon(name: &str, class: &str) -> String {
    let cls = if class.is_empty() { String::new() } else { format!(" {class}") };
    format!("<svg class=\"i{cls}\" viewBox=\"0 0 24 24\" aria-hidden=\"true\">{}</svg>", path(name))
}
