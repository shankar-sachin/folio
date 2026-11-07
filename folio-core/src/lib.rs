//! Folio's Markdown engine: GFM block and inline parsing, safe HTML rendering,
//! code highlighting, the source-editor syntax layer and find marking. No dependencies.

pub mod block;
pub mod code;
pub mod html;
pub mod icons;
pub mod inline;
pub mod source;

pub use block::{render, slug, toggle_task, word_count, Heading, Options, Rendered, Slugger};
pub use html::esc;
pub use icons::icon;
pub use source::{find_layer, highlight_source, mark_html};
