//! The command table (palette, shortcuts page, native menu) and fuzzy matching.

use folio_core::esc;

pub struct Command {
    pub id: &'static str,
    pub title: &'static str,
    pub key: &'static str,
    pub group: &'static str,
    pub icon: &'static str,
    pub hidden: bool,
}

const fn c(id: &'static str, title: &'static str, key: &'static str, group: &'static str, icon: &'static str) -> Command {
    Command { id, title, key, group, icon, hidden: false }
}
const fn h(id: &'static str, title: &'static str, key: &'static str, group: &'static str) -> Command {
    Command { id, title, key, group, icon: "command", hidden: true }
}

pub const COMMANDS: &[Command] = &[
    c("newFile", "New File", "⌘N", "File", "filePlus"),
    c("newFileInFolder", "New File in Workspace…", "", "File", "filePlus"),
    c("newFolder", "New Folder in Workspace…", "", "File", "folderPlus"),
    c("open", "Open…", "⌘O", "File", "files"),
    c("addFolder", "Add Folder to Workspace…", "⌥⌘O", "Workspace", "folderPlus"),
    c("save", "Save", "⌘S", "File", "command"),
    c("saveAs", "Save As…", "⇧⌘S", "File", "command"),
    c("saveAll", "Save All", "⌥⌘S", "File", "command"),
    c("revert", "Revert File", "", "File", "history"),
    c("reveal", "Reveal in Finder", "⌥⌘R", "File", "finder"),
    c("copyPath", "Copy Path of Active File", "", "File", "copy"),
    c("exportHtml", "Export as HTML…", "", "File", "export"),
    c("copyHtml", "Copy Rendered HTML", "", "File", "copy"),
    c("closeTab", "Close Tab", "⌘W", "Tabs", "x"),
    c("closeAll", "Close All Tabs", "⌥⌘W", "Tabs", "x"),
    c("closeOthers", "Close Other Tabs", "", "Tabs", "x"),
    c("closeSaved", "Close Saved Tabs", "", "Tabs", "x"),
    c("reopenTab", "Reopen Closed Tab", "⇧⌘T", "Tabs", "history"),
    c("nextTab", "Next Tab", "⇧⌘]", "Tabs", "command"),
    c("prevTab", "Previous Tab", "⇧⌘[", "Tabs", "command"),
    c("quickOpen", "Go to File…", "⌘P", "Navigate", "search"),
    c("palette", "Show All Commands", "⇧⌘P", "Navigate", "command"),
    c("gotoHeading", "Go to Heading…", "⇧⌘O", "Navigate", "hash"),
    c("gotoLine", "Go to Line…", "⌃G", "Navigate", "hash"),
    c("find", "Find", "⌘F", "Edit", "search"),
    h("findNext", "Find Next", "⌘G", "Edit"),
    h("findPrev", "Find Previous", "⇧⌘G", "Edit"),
    h("closeFind", "Close Find", "", "Edit"),
    c("searchView", "Search in Workspace", "⇧⌘F", "View", "search"),
    c("bold", "Toggle Bold", "⌘B", "Edit", "command"),
    c("italic", "Toggle Italic", "⌘I", "Edit", "command"),
    c("link", "Insert Link", "⌘K", "Edit", "link"),
    c("explorerView", "Show Explorer", "⇧⌘E", "View", "files"),
    c("outlineView", "Show Outline", "⇧⌘U", "View", "outline"),
    c("workspacesView", "Show Workspaces", "⇧⌘K", "View", "layers"),
    c("toggleSidebar", "Toggle Sidebar", "⌘\\", "View", "sidebar"),
    c("read", "View: Live (edit in place)", "⌘1", "View", "book"),
    c("split", "View: Split Editor & Preview", "⌘2", "View", "columns"),
    c("source", "View: Source", "⌘3", "View", "code"),
    c("togglePreview", "Toggle Live / Source", "⇧⌘V", "View", "command"),
    h("cycleMode", "Cycle View Mode", "", "View"),
    c("zoomIn", "Increase Text Size", "⌘=", "View", "command"),
    c("zoomOut", "Decrease Text Size", "⌘-", "View", "command"),
    c("zoomReset", "Reset Text Size", "⌘0", "View", "command"),
    c("wrap", "Toggle Word Wrap", "⌥Z", "View", "wrap"),
    c("lineNumbers", "Toggle Line Numbers", "", "View", "command"),
    c("toggleSync", "Toggle Sync Scrolling", "", "View", "command"),
    c("zen", "Toggle Zen Mode", "⌃⌘J", "View", "zen"),
    c("theme", "Toggle Light / Dark Theme", "⌥⌘T", "Appearance", "sun"),
    c("pickTheme", "Color Theme…", "", "Appearance", "sun"),
    c("settings", "Open Settings", "⌘,", "Preferences", "settings"),
    c("shortcuts", "Keyboard Shortcuts", "⌘/", "Preferences", "keyboard"),
    c("welcome", "Show Welcome", "", "Help", "info"),
    c("newWorkspace", "New Workspace…", "⌃⌘N", "Workspace", "layers"),
    c("switchWorkspace", "Switch Workspace…", "⌃⌘R", "Workspace", "layers"),
    c("renameWorkspace", "Rename Workspace…", "", "Workspace", "edit"),
    c("refreshExplorer", "Refresh Explorer", "", "Workspace", "refresh"),
    c("collapseFolders", "Collapse Folders in Explorer", "", "Workspace", "collapse"),
    c("showAllFiles", "Toggle Showing All Files", "", "Workspace", "command"),
];

pub fn get(id: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.id == id)
}

/// Subsequence match with bonuses for word starts and runs. Returns (score, matched char indices).
pub fn fuzzy(q: &str, s: &str) -> Option<(f64, Vec<usize>)> {
    if q.is_empty() {
        return Some((0.0, vec![]));
    }
    let qc: Vec<char> = q.to_lowercase().chars().collect();
    let sc: Vec<char> = s.chars().collect();
    let sl: Vec<char> = s.to_lowercase().chars().collect();
    if sl.len() != sc.len() {
        return None;
    }
    let (mut score, mut j, mut prev) = (0.0, 0usize, -2isize);
    let mut idx = vec![];
    for i in 0..sl.len() {
        if j >= qc.len() {
            break;
        }
        if sl[i] == qc[j] {
            let mut bonus = 1.0;
            if i as isize == prev + 1 {
                bonus += 4.0;
            }
            if i == 0 || " /_.-".contains(sc[i - 1]) || (sc[i].is_uppercase() && sc[i - 1].is_lowercase()) {
                bonus += 6.0;
            }
            score += bonus;
            idx.push(i);
            prev = i as isize;
            j += 1;
        }
    }
    if j < qc.len() {
        return None;
    }
    let ql: String = qc.iter().collect();
    let slow: String = sl.iter().collect();
    if let Some(sub) = slow.find(&ql) {
        score += 20.0 + if sub == 0 { 15.0 } else { 0.0 };
    }
    Some((score - sc.len() as f64 * 0.05, idx))
}

pub fn mark_idx(s: &str, idx: &[usize]) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if idx.contains(&i) {
            out.push_str("<b>");
            out.push_str(&esc(&ch.to_string()));
            out.push_str("</b>");
        } else {
            out.push_str(&esc(&ch.to_string()));
        }
    }
    out
}
