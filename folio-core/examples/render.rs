fn main() {
    let p = std::env::args().nth(1).unwrap();
    let t = std::fs::read_to_string(&p).unwrap();
    let r = folio_core::render(&t, &folio_core::Options { base_dir: Some("/tmp"), remote_images: false });
    println!("{}", r.html);
    eprintln!("blocks {:?}\nheadings {:?}\nwords {} tasks {:?}", r.blocks, r.headings.iter().map(|h| (&h.text, h.line, &h.id)).collect::<Vec<_>>(), r.words, r.tasks);
}
