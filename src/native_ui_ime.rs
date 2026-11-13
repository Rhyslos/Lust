// ----- typing in any language: input methods, and letters from the computer's own fonts -----
//
// Chinese, Japanese, Korean and many other languages are typed through an input method (IME):
// keys make a few letters that are still being put together (the pre-edit text), and choosing
// from a little list turns them into the finished text. While an input, text box, number box,
// date picker's box or colour picker's hex box has focus, the window asks the computer for its
// input method and tells it where the caret is, so the list shows beside it. The pre-edit text
// is drawn at the caret, underlined, without being written to the field; the finished text is
// typed in like any other.
//
// Lust's own font (Inter) has no Chinese, Japanese or Korean letters, so letters it doesn't have
// come from the computer's own fonts, read the first time a letter needs one. Tests leave them
// out (unless LUST_SYSTEM_FONTS is set), so they measure the same on every computer.

/// The computer's own fonts that missing letters come from, in the order they're tried, with
/// which font in the file to read (a .ttc file holds several).
fn fallback_files() -> Vec<(String, u32)> {
    let mut out: Vec<(String, u32)> = Vec::new();
    if cfg!(windows) {
        let root = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        // Yu Gothic UI, Microsoft YaHei UI, Malgun Gothic and Segoe UI Symbol.
        for (file, index) in [("YuGothR.ttc", 1), ("YuGothM.ttc", 1), ("msyh.ttc", 1), ("malgun.ttf", 0), ("seguisym.ttf", 0)] {
            out.push((format!("{root}\\Fonts\\{file}"), index));
        }
    } else if cfg!(target_os = "macos") {
        for file in ["/System/Library/Fonts/Hiragino Sans GB.ttc", "/System/Library/Fonts/AppleSDGothicNeo.ttc", "/System/Library/Fonts/Apple Symbols.ttf"] {
            out.push((file.into(), 0));
        }
    } else {
        // Where Linux systems usually keep Noto CJK, and a few others with the same letters.
        for file in [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/google-noto-sans-cjk-fonts/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
            "/usr/share/fonts/noto-cjk/NotoSansCJKjp-Regular.otf",
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ] {
            out.push((file.into(), 0));
        }
    }
    out
}

/// Whether letters may come from the computer's own fonts: always in a window, and in tests only
/// when LUST_SYSTEM_FONTS is set, so tests give the same everywhere.
fn system_fonts_allowed() -> bool {
    std::env::var("LUST_VIEW_TEST").is_err() || std::env::var("LUST_SYSTEM_FONTS").is_ok()
}

/// A system font that has this letter: one read already, or the next in the list that has it,
/// read now. None leaves the letter to be drawn as a box.
fn fallback_for(f: &mut Faces, ch: char) -> Option<usize> {
    use ab_glyph::Font;
    if let Some(&face) = f.fallbacks.iter().find(|&&face| f.fonts[face].glyph_id(ch).0 != 0) {
        return Some(face);
    }
    if !system_fonts_allowed() {
        return None;
    }
    let files = fallback_files();
    while f.fallback_next < files.len() {
        let (path, index) = &files[f.fallback_next];
        f.fallback_next += 1;
        let Ok(bytes) = std::fs::read(path) else { continue };
        let font = ab_glyph::FontVec::try_from_vec_and_index(bytes.clone(), *index).or_else(|_| ab_glyph::FontVec::try_from_vec_and_index(bytes, 0));
        let Ok(font) = font else { continue };
        f.fonts.push(ab_glyph::FontArc::new(font));
        let face = f.fonts.len() - 1;
        f.fallbacks.push(face);
        if f.fonts[face].glyph_id(ch).0 != 0 {
            return Some(face);
        }
    }
    None
}

/// The text an input method is putting together, and its own caret in it (where its chosen part
/// starts and ends, in bytes), or None when it isn't composing.
type Preedit = Option<(String, Option<(usize, usize)>)>;

/// A field's text with the pre-edit text in place of its selection: the text, where the pre-edit
/// starts and ends in it, and where the caret goes.
fn with_preedit(text: &str, caret: &Caret, pre: &str, cursor: Option<(usize, usize)>) -> (String, usize, usize, usize) {
    let (a, b) = caret.range();
    let (a, b) = (a.min(text.len()), b.min(text.len()));
    let shown = format!("{}{pre}{}", &text[..a], &text[b..]);
    let at = match cursor {
        Some((_, e)) if e <= pre.len() && pre.is_char_boundary(e) => a + e,
        _ => a + pre.len(),
    };
    (shown, a, a + pre.len(), at)
}

/// The line under the pre-edit text, from x0 to x1 at the bottom of a line, thicker under the part
/// the input method is working on.
fn preedit_line(c: &mut Canvas, x0: f32, x1: f32, bottom: f32, colour: u32) {
    if x1 > x0 {
        c.rect(x0, bottom - 2.0, x1 - x0, 1.0, 0.0, colour, 1.0);
    }
}

/// An input (or number box, or date picker's box) while an input method composes in it: the
/// field's text with the pre-edit text at the caret, underlined, and the caret where the input
/// method puts it.
fn paint_input_preedit(c: &mut Canvas, n: &Laid, st: &PaintState, pre: &str, cursor: Option<(usize, usize)>) {
    let l = &n.look;
    let (shown, a, b, at) = with_preedit(value_text(n), &st.caret, pre, cursor);
    let (ix, top, iw) = input_inner(n);
    let (_, lh) = line_metrics(l.size, l.face());
    // Slid so the caret stays in view.
    let cx = text_width(&shown[..at], l.size, l.face());
    let mut scroll = st.scroll_x;
    if cx - scroll > iw - 2.0 {
        scroll = cx - iw + 2.0;
    }
    if cx < scroll {
        scroll = cx;
    }
    let mut m = n.clone();
    m.value = Some(SV::Text(shown.clone()));
    let mut st2 = st.clone();
    st2.preedit = None;
    st2.caret = Caret::at(at);
    st2.scroll_x = scroll.max(0.0);
    paint_input(c, &m, true, &st2);
    let was = c.clip_to(ix - 1.0, n.y, iw + 2.0, n.h);
    let x0 = ix + text_width(&shown[..a], l.size, l.face()) - st2.scroll_x;
    let x1 = ix + text_width(&shown[..b], l.size, l.face()) - st2.scroll_x;
    preedit_line(c, x0, x1, top + lh, l.color);
    if let Some((s, e)) = cursor.filter(|(s, e)| s < e && *e <= pre.len() && pre.is_char_boundary(*s) && pre.is_char_boundary(*e)) {
        let xs = ix + text_width(&shown[..a + s], l.size, l.face()) - st2.scroll_x;
        let xe = ix + text_width(&shown[..a + e], l.size, l.face()) - st2.scroll_x;
        c.rect(xs, top + lh - 3.0, xe - xs, 2.0, 0.0, l.color, 1.0);
    }
    c.clip = was;
}

/// A text box while an input method composes in it: as an input, with the lines broken again
/// around the pre-edit text.
fn paint_textarea_preedit(c: &mut Canvas, n: &Laid, st: &PaintState, pre: &str, cursor: Option<(usize, usize)>) {
    let l = &n.look;
    let (shown, a, b, at) = with_preedit(value_text(n), &st.caret, pre, cursor);
    let mut m = n.clone();
    let iw = n.w - l.pad[1] - l.pad[3];
    m.spans = wrap_spans(&shown, l.size, l.face(), (iw - 6.0).max(1.0));
    m.value = Some(SV::Text(shown.clone()));
    let mut st2 = st.clone();
    st2.preedit = None;
    st2.caret = Caret::at(at);
    paint_textarea(c, &m, true, &st2);
    let (_, lh) = line_metrics(l.size, l.face());
    let was = c.clip_to(n.x, n.y, n.w, n.h);
    // Underlined on each line the pre-edit text is on.
    for &(s, e) in &m.spans {
        let (from, to) = (a.max(s), b.min(e));
        if from >= to {
            continue;
        }
        let (x0, y) = textarea_point(&m, from);
        let (x1, _) = textarea_point(&m, to);
        preedit_line(c, x0, x1, y + lh, l.color);
    }
    c.clip = was;
}

impl<'a, 'b> Ui<'a, 'b> {
    /// Whether what has focus takes typing, so the computer's input method can be used.
    fn takes_typing(&self) -> bool {
        self.picker_takes_text() || self.focused().is_some_and(|n| editable(&n.kind) && n.action.is_some())
    }

    /// Whether an input method is putting text together, so keys go to it rather than to Lust.
    fn composing(&self) -> bool {
        self.st.preedit.is_some()
    }

    /// The input method's pre-edit text changed (empty when it stops composing).
    fn ime_preedit(&mut self, text: String, cursor: Option<(usize, usize)>) {
        if text.is_empty() || !self.takes_typing() {
            self.st.preedit = None;
            return;
        }
        let cursor = cursor.filter(|&(s, e)| s <= e && e <= text.len() && text.is_char_boundary(s) && text.is_char_boundary(e));
        self.st.preedit = Some((text, cursor));
        self.wake();
    }

    /// The input method finished: its text is typed in, like keys, and the bound field changes.
    fn ime_commit(&mut self, text: &str) -> Result<(), String> {
        self.st.preedit = None;
        if text.is_empty() || !self.takes_typing() {
            return Ok(());
        }
        self.typed(text)
    }

    /// Where the caret is in the window (x, y, width, height), for the input method's list of
    /// choices to show beside it.
    fn caret_area(&self) -> Option<[f32; 4]> {
        if let Some(p) = &self.picker {
            if p.colour && p.part == Part::Hex {
                let (_, b) = self.picker_node()?;
                let (r, _) = col_rects(b.look.alpha, swatches_of(&b.look).len());
                let face = b.look.face();
                let (_, lh) = line_metrics(14.0, face);
                let hx = [b.x + r[3][0], b.y + r[3][1], r[3][2], r[3][3]];
                let x = hx[0] + 10.0 + text_width(&p.hex[..p.caret.caret.min(p.hex.len())], 14.0, face);
                return Some([x, hx[1] + ((hx[3] - lh) / 2.0).round(), 1.0, lh]);
            }
        }
        let n = self.focused()?;
        if !editable(&n.kind) {
            return None;
        }
        let l = &n.look;
        let (_, lh) = line_metrics(l.size, l.face());
        let t = value_text(n);
        let caret = self.st.caret.range().0.min(t.len());
        if n.kind == "textarea" {
            let (x, y) = textarea_point(n, caret);
            return Some([x, y, 1.0, lh]);
        }
        let (ix, top, _) = input_inner(n);
        Some([ix + text_width(&t[..caret], l.size, l.face()) - self.st.scroll_x, top, 1.0, lh])
    }
}

#[cfg(test)]
mod ime_tests {
    use super::*;

    #[test]
    fn preedit_goes_in_place_of_the_selection() {
        let mut c = Caret::at(3);
        assert_eq!(with_preedit("abcdef", &c, "にほん", None), ("abcにほんdef".to_string(), 3, 12, 12));
        c.anchor = 1;
        assert_eq!(with_preedit("abcdef", &c, "x", Some((0, 0))), ("axdef".to_string(), 1, 2, 1));
        // A caret past the end, or an input method's caret inside a letter, is kept safe.
        let c = Caret::at(99);
        assert_eq!(with_preedit("ab", &c, "日本", Some((1, 1))), ("ab日本".to_string(), 2, 8, 8));
        assert_eq!(with_preedit("", &Caret::at(0), "", None), (String::new(), 0, 0, 0));
    }

    #[test]
    fn missing_letters_come_from_system_fonts() {
        // Only where a CJK font is installed, and not while view tests turn them off.
        if !system_fonts_allowed() || !fallback_files().iter().any(|(p, _)| std::path::Path::new(p).exists()) {
            return;
        }
        let w = text_width("日本", 20.0, 0);
        let face = FACES.with(|f| letter(&mut f.borrow_mut(), 0, '日').0);
        assert_ne!(face, 0, "日 comes from a system font");
        assert!(w > 30.0 && w < 45.0, "two CJK letters are about two ems wide, not boxes: {w}");
        // Latin letters still come from Inter.
        assert_eq!(FACES.with(|f| letter(&mut f.borrow_mut(), 0, 'a').0), 0);
    }
}
