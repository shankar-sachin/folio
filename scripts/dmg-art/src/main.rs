//! Renders the Finder background for Folio's DMG window.
//!
//!   dmg-art <out-dir> <version>   -> <out-dir>/background.png (720x440) and background@2x.png
//!
//! Layout (points, top-left origin) must match the AppleScript in scripts/build.sh:
//! window content 720x440, Folio icon centred at (190, 232), Applications at (530, 232), icon size 128.
#![allow(non_upper_case_globals)]

use std::ffi::{c_void, CString};
use std::ptr::null;

type CGFloat = f64;
type Ref = *const c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint { x: CGFloat, y: CGFloat }

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize { w: CGFloat, h: CGFloat }

#[repr(C)]
struct CFDictionaryKeyCallBacks([usize; 6]);
#[repr(C)]
struct CFDictionaryValueCallBacks([usize; 5]);

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFTypeDictionaryKeyCallBacks: CFDictionaryKeyCallBacks;
    static kCFTypeDictionaryValueCallBacks: CFDictionaryValueCallBacks;
    fn CFStringCreateWithCString(alloc: Ref, s: *const i8, enc: u32) -> Ref;
    fn CFDictionaryCreate(alloc: Ref, keys: *const Ref, values: *const Ref, n: isize, kcb: *const CFDictionaryKeyCallBacks, vcb: *const CFDictionaryValueCallBacks) -> Ref;
    fn CFAttributedStringCreate(alloc: Ref, s: Ref, attrs: Ref) -> Ref;
    fn CFURLCreateFromFileSystemRepresentation(alloc: Ref, buf: *const u8, len: isize, dir: bool) -> Ref;
    fn CFNumberCreate(alloc: Ref, ty: isize, val: *const c_void) -> Ref;
    fn CFStringGetCString(s: Ref, buf: *mut i8, len: isize, enc: u32) -> bool;
    fn CFRelease(r: Ref);
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorSpaceCreateWithName(name: Ref) -> Ref;
    static kCGColorSpaceSRGB: Ref;
    fn CGBitmapContextCreate(data: *mut c_void, w: usize, h: usize, bpc: usize, bpr: usize, cs: Ref, info: u32) -> Ref;
    fn CGBitmapContextCreateImage(c: Ref) -> Ref;
    fn CGContextScaleCTM(c: Ref, sx: CGFloat, sy: CGFloat);
    fn CGContextSaveGState(c: Ref);
    fn CGContextRestoreGState(c: Ref);
    fn CGGradientCreateWithColorComponents(cs: Ref, comps: *const CGFloat, locs: *const CGFloat, n: usize) -> Ref;
    fn CGContextDrawLinearGradient(c: Ref, g: Ref, a: CGPoint, b: CGPoint, opts: u32);
    fn CGContextDrawRadialGradient(c: Ref, g: Ref, c0: CGPoint, r0: CGFloat, c1: CGPoint, r1: CGFloat, opts: u32);
    fn CGContextBeginPath(c: Ref);
    fn CGContextMoveToPoint(c: Ref, x: CGFloat, y: CGFloat);
    fn CGContextAddLineToPoint(c: Ref, x: CGFloat, y: CGFloat);
    fn CGContextAddCurveToPoint(c: Ref, c1x: CGFloat, c1y: CGFloat, c2x: CGFloat, c2y: CGFloat, x: CGFloat, y: CGFloat);
    fn CGContextSetLineWidth(c: Ref, w: CGFloat);
    fn CGContextSetLineCap(c: Ref, cap: i32);
    fn CGContextSetLineJoin(c: Ref, join: i32);
    fn CGContextSetLineDash(c: Ref, phase: CGFloat, lens: *const CGFloat, n: usize);
    fn CGContextSetRGBStrokeColor(c: Ref, r: CGFloat, g: CGFloat, b: CGFloat, a: CGFloat);
    fn CGContextSetRGBFillColor(c: Ref, r: CGFloat, g: CGFloat, b: CGFloat, a: CGFloat);
    fn CGContextStrokePath(c: Ref);
    fn CGContextFillPath(c: Ref);
    fn CGContextReplacePathWithStrokedPath(c: Ref);
    fn CGContextClip(c: Ref);
    fn CGContextSetShadowWithColor(c: Ref, off: CGSize, blur: CGFloat, color: Ref);
    fn CGColorCreateSRGB(r: CGFloat, g: CGFloat, b: CGFloat, a: CGFloat) -> Ref;
    fn CGContextSetTextPosition(c: Ref, x: CGFloat, y: CGFloat);
    fn CGContextSetShouldSmoothFonts(c: Ref, b: bool);
}

#[link(name = "CoreText", kind = "framework")]
extern "C" {
    static kCTFontAttributeName: Ref;
    static kCTForegroundColorAttributeName: Ref;
    static kCTKernAttributeName: Ref;
    fn CTFontCreateUIFontForLanguage(ty: u32, size: CGFloat, lang: Ref) -> Ref;
    fn CTFontDescriptorCreateWithAttributes(attrs: Ref) -> Ref;
    fn CTFontCreateWithFontDescriptor(d: Ref, size: CGFloat, m: *const c_void) -> Ref;
    fn CTFontCopyPostScriptName(f: Ref) -> Ref;
    fn CTLineCreateWithAttributedString(s: Ref) -> Ref;
    fn CTLineGetTypographicBounds(l: Ref, asc: *mut CGFloat, desc: *mut CGFloat, lead: *mut CGFloat) -> f64;
    fn CTLineDraw(l: Ref, c: Ref);
}

#[link(name = "ImageIO", kind = "framework")]
extern "C" {
    static kCGImagePropertyDPIWidth: Ref;
    static kCGImagePropertyDPIHeight: Ref;
    fn CGImageDestinationCreateWithURL(url: Ref, ty: Ref, count: usize, opts: Ref) -> Ref;
    fn CGImageDestinationAddImage(d: Ref, img: Ref, props: Ref);
    fn CGImageDestinationFinalize(d: Ref) -> bool;
}

const W: CGFloat = 720.0;
const H: CGFloat = 440.0;
const LEFT: (CGFloat, CGFloat) = (190.0, 232.0);
const RIGHT: (CGFloat, CGFloat) = (530.0, 232.0);

fn cfstr(s: &str) -> Ref {
    let c = CString::new(s).unwrap();
    unsafe { CFStringCreateWithCString(null(), c.as_ptr(), 0x0800_0100) }
}

/// Points in top-left coordinates -> CG's bottom-left coordinates.
fn p(x: CGFloat, y: CGFloat) -> CGPoint { CGPoint { x, y: H - y } }

struct Canvas { ctx: Ref, cs: Ref }

impl Canvas {
    fn new(scale: CGFloat) -> Canvas {
        unsafe {
            let cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
            // kCGImageAlphaPremultipliedLast
            let ctx = CGBitmapContextCreate(std::ptr::null_mut(), (W * scale) as usize, (H * scale) as usize, 8, 0, cs, 1);
            assert!(!ctx.is_null(), "bitmap context");
            CGContextScaleCTM(ctx, scale, scale);
            Canvas { ctx, cs }
        }
    }

    fn gradient(&self, stops: &[(CGFloat, [CGFloat; 4])]) -> Ref {
        let comps: Vec<CGFloat> = stops.iter().flat_map(|s| s.1).collect();
        let locs: Vec<CGFloat> = stops.iter().map(|s| s.0).collect();
        unsafe { CGGradientCreateWithColorComponents(self.cs, comps.as_ptr(), locs.as_ptr(), stops.len()) }
    }

    fn linear(&self, a: CGPoint, b: CGPoint, stops: &[(CGFloat, [CGFloat; 4])]) {
        let g = self.gradient(stops);
        unsafe { CGContextDrawLinearGradient(self.ctx, g, a, b, 3); CFRelease(g) }
    }

    /// Elliptical glow: radial gradient drawn in a scaled space.
    fn glow(&self, cx: CGFloat, cy: CGFloat, rx: CGFloat, ry: CGFloat, rgb: [CGFloat; 3], alpha: CGFloat) {
        let [r, g, b] = rgb;
        let grad = self.gradient(&[
            (0.0, [r, g, b, alpha]),
            (0.45, [r, g, b, alpha * 0.45]),
            (1.0, [r, g, b, 0.0]),
        ]);
        unsafe {
            CGContextSaveGState(self.ctx);
            let c = p(cx, cy);
            CGContextScaleCTM(self.ctx, 1.0, ry / rx);
            let c = CGPoint { x: c.x, y: c.y * rx / ry };
            CGContextDrawRadialGradient(self.ctx, grad, c, 0.0, c, rx, 0);
            CGContextRestoreGState(self.ctx);
            CFRelease(grad);
        }
    }

    fn line(&self, text: &str, size: CGFloat, font: Font, rgba: [CGFloat; 4], kern: CGFloat) -> (Ref, CGFloat) {
        unsafe {
            let f = match font {
                Font::Bold => CTFontCreateUIFontForLanguage(3, size, null()),
                Font::Regular => CTFontCreateUIFontForLanguage(2, size, null()),
                Font::Serif(weight) => {
                    // Apple's New York, reached the supported way: the system font's serif design.
                    let (kd, vd, kw) = (cfstr("NSCTFontUIFontDesignTrait"), cfstr("NSCTFontUIFontDesignSerif"), cfstr("NSCTFontWeightTrait"));
                    let w = CFNumberCreate(null(), 13, &weight as *const CGFloat as *const c_void);
                    let traits = CFDictionaryCreate(null(), [kd, kw].as_ptr(), [vd, w].as_ptr(), 2, &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
                    let kt = cfstr("NSCTFontTraitsAttribute");
                    let attrs = CFDictionaryCreate(null(), [kt].as_ptr(), [traits].as_ptr(), 1, &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
                    let desc = CTFontDescriptorCreateWithAttributes(attrs);
                    let f = CTFontCreateWithFontDescriptor(desc, size, null());
                    if std::env::var_os("DMG_ART_DEBUG").is_some() {
                        let n = CTFontCopyPostScriptName(f);
                        let mut buf = [0i8; 128];
                        CFStringGetCString(n, buf.as_mut_ptr(), 128, 0x0800_0100);
                        eprintln!("serif font: {}", std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy());
                        CFRelease(n);
                    }
                    for r in [desc, attrs, kt, traits, w, kw, vd, kd] { CFRelease(r) }
                    f
                }
            };
            let color = CGColorCreateSRGB(rgba[0], rgba[1], rgba[2], rgba[3]);
            let k = CFNumberCreate(null(), 13 /* kCFNumberCGFloatType */, &kern as *const CGFloat as *const c_void);
            let keys = [kCTFontAttributeName, kCTForegroundColorAttributeName, kCTKernAttributeName];
            let vals = [f, color, k];
            let attrs = CFDictionaryCreate(null(), keys.as_ptr(), vals.as_ptr(), 3, &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
            let s = cfstr(text);
            let astr = CFAttributedStringCreate(null(), s, attrs);
            let line = CTLineCreateWithAttributedString(astr);
            let w = CTLineGetTypographicBounds(line, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            for r in [astr, s, attrs, k, color, f] { CFRelease(r) }
            (line, w)
        }
    }

    /// Draws `text` horizontally centred on `cx` with its baseline at `y` (top-left coordinates).
    fn text(&self, text: &str, cx: CGFloat, y: CGFloat, size: CGFloat, font: Font, rgba: [CGFloat; 4], kern: CGFloat) {
        let (line, w) = self.line(text, size, font, rgba, kern);
        unsafe {
            CGContextSetTextPosition(self.ctx, (cx - w / 2.0).round(), H - y);
            CTLineDraw(line, self.ctx);
            CFRelease(line);
        }
    }

    fn save_png(&self, path: &str, dpi: f64) {
        unsafe {
            let img = CGBitmapContextCreateImage(self.ctx);
            let url = CFURLCreateFromFileSystemRepresentation(null(), path.as_ptr(), path.len() as isize, false);
            let ty = cfstr("public.png");
            let dest = CGImageDestinationCreateWithURL(url, ty, 1, null());
            assert!(!dest.is_null(), "cannot write {path}");
            let n = CFNumberCreate(null(), 13, &dpi as *const f64 as *const c_void);
            let keys = [kCGImagePropertyDPIWidth, kCGImagePropertyDPIHeight];
            let vals = [n, n];
            let props = CFDictionaryCreate(null(), keys.as_ptr(), vals.as_ptr(), 2, &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
            CGImageDestinationAddImage(dest, img, props);
            assert!(CGImageDestinationFinalize(dest), "finalize {path}");
            for r in [props, n, dest, ty, url, img] { CFRelease(r) }
        }
    }
}

#[derive(Clone, Copy)]
enum Font { Bold, Regular, Serif(CGFloat) }

fn hex(h: u32, a: CGFloat) -> [CGFloat; 4] {
    [((h >> 16) & 255) as CGFloat / 255.0, ((h >> 8) & 255) as CGFloat / 255.0, (h & 255) as CGFloat / 255.0, a]
}

fn render(scale: CGFloat, version: &str) -> Canvas {
    let c = Canvas::new(scale);
    let ctx = c.ctx;
    unsafe { CGContextSetShouldSmoothFonts(ctx, false) };

    // Base: deep brand gradient, slightly lighter at the top.
    c.linear(p(0.0, 0.0), p(0.0, H), &[(0.0, hex(0x171922, 1.0)), (0.55, hex(0x101117, 1.0)), (1.0, hex(0x0b0c10, 1.0))]);

    // Atmosphere: a violet bloom between the icons, a cool blue one top right, pink low left.
    c.glow(360.0, 210.0, 330.0, 190.0, [0.545, 0.486, 1.0], 0.20);
    c.glow(640.0, 40.0, 260.0, 170.0, [0.424, 0.714, 1.0], 0.08);
    c.glow(70.0, 430.0, 240.0, 150.0, [0.941, 0.561, 0.753], 0.06);

    // Soft halos behind each icon so they sit on light, not on void.
    for (x, y) in [LEFT, RIGHT] {
        c.glow(x, y, 120.0, 120.0, [0.722, 0.682, 1.0], 0.10);
    }

    // Faint dotted grid, fading out towards the edges (a nod to paper / notebooks).
    unsafe {
        let mut y = 22.0;
        while y < H {
            let mut x = 22.0;
            while x < W {
                let dx = (x - 360.0) / 360.0;
                let dy = (y - 220.0) / 220.0;
                let fade = (1.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
                // Keep the icon and text areas clean.
                let near_icon = [LEFT, RIGHT].iter().any(|&(ix, iy)| (x - ix).hypot(y - iy) < 100.0);
                if fade > 0.0 && !near_icon && y > 130.0 {
                    CGContextSetRGBFillColor(ctx, 1.0, 1.0, 1.0, 0.035 * fade + 0.01);
                    CGContextBeginPath(ctx);
                    let q = p(x, y);
                    CGContextMoveToPoint(ctx, q.x - 0.75, q.y);
                    CGContextAddCurveToPoint(ctx, q.x - 0.75, q.y + 0.41, q.x - 0.41, q.y + 0.75, q.x, q.y + 0.75);
                    CGContextAddCurveToPoint(ctx, q.x + 0.41, q.y + 0.75, q.x + 0.75, q.y + 0.41, q.x + 0.75, q.y);
                    CGContextAddCurveToPoint(ctx, q.x + 0.75, q.y - 0.41, q.x + 0.41, q.y - 0.75, q.x, q.y - 0.75);
                    CGContextAddCurveToPoint(ctx, q.x - 0.41, q.y - 0.75, q.x - 0.75, q.y - 0.41, q.x - 0.75, q.y);
                    CGContextFillPath(ctx);
                }
                x += 24.0;
            }
            y += 24.0;
        }
    }

    // The drag arrow: a gentle arc from Folio to Applications, dashed, with a violet gradient.
    let (sx, sy) = (LEFT.0 + 92.0, LEFT.1 - 6.0);
    let (ex, ey) = (RIGHT.0 - 96.0, RIGHT.1 - 6.0);
    let arc = |ctx: Ref| unsafe {
        let a = p(sx, sy);
        let c1 = p(sx + 45.0, sy - 46.0);
        let c2 = p(ex - 45.0, ey - 46.0);
        let b = p(ex, ey);
        CGContextBeginPath(ctx);
        CGContextMoveToPoint(ctx, a.x, a.y);
        CGContextAddCurveToPoint(ctx, c1.x, c1.y, c2.x, c2.y, b.x, b.y);
    };
    unsafe {
        // Glow pass.
        CGContextSaveGState(ctx);
        let shadow = CGColorCreateSRGB(0.545, 0.486, 1.0, 0.55);
        CGContextSetShadowWithColor(ctx, CGSize { w: 0.0, h: 0.0 }, 10.0, shadow);
        CFRelease(shadow);
        arc(ctx);
        CGContextSetLineWidth(ctx, 3.0);
        CGContextSetLineCap(ctx, 1);
        let dash = [0.1, 9.0];
        CGContextSetLineDash(ctx, 0.0, dash.as_ptr(), 2);
        CGContextSetRGBStrokeColor(ctx, 0.62, 0.57, 1.0, 0.9);
        CGContextStrokePath(ctx);
        CGContextRestoreGState(ctx);

        // Gradient-coloured dots on top.
        CGContextSaveGState(ctx);
        arc(ctx);
        CGContextSetLineWidth(ctx, 3.4);
        CGContextSetLineCap(ctx, 1);
        CGContextSetLineDash(ctx, 0.0, dash.as_ptr(), 2);
        CGContextReplacePathWithStrokedPath(ctx);
        CGContextClip(ctx);
        c.linear(p(sx, 0.0), p(ex, 0.0), &[(0.0, hex(0x8b7cff, 0.55)), (1.0, hex(0xd6cfff, 1.0))]);
        CGContextRestoreGState(ctx);

        // Arrowhead (rounded chevron), aligned with the curve's end tangent.
        let tip = p(ex + 3.0, ey + 3.0);
        let (tx, ty) = (45.0_f64, 46.0_f64); // end tangent (c2 -> end), top-left coords
        let len = tx.hypot(ty);
        let (ux, uy) = (tx / len, -ty / len); // CG coords
        let back = 11.0;
        let wing = 8.5;
        let bx = tip.x - ux * back;
        let by = tip.y - uy * back;
        let (nx, ny) = (-uy, ux);
        CGContextSaveGState(ctx);
        let shadow = CGColorCreateSRGB(0.545, 0.486, 1.0, 0.6);
        CGContextSetShadowWithColor(ctx, CGSize { w: 0.0, h: 0.0 }, 8.0, shadow);
        CFRelease(shadow);
        CGContextBeginPath(ctx);
        CGContextMoveToPoint(ctx, bx + nx * wing, by + ny * wing);
        CGContextAddLineToPoint(ctx, tip.x, tip.y);
        CGContextAddLineToPoint(ctx, bx - nx * wing, by - ny * wing);
        CGContextSetLineWidth(ctx, 3.2);
        CGContextSetLineCap(ctx, 1);
        CGContextSetLineJoin(ctx, 1);
        CGContextSetRGBStrokeColor(ctx, 0.84, 0.81, 1.0, 1.0);
        CGContextStrokePath(ctx);
        CGContextRestoreGState(ctx);
    }

    // Headline + subtitle.
    c.text("Drag Folio into Applications", 360.0, 74.0, 25.0, Font::Serif(0.3), hex(0xf3f3f7, 1.0), -0.2);
    c.text("Then open it from Launchpad or Spotlight.", 360.0, 100.0, 13.0, Font::Regular, hex(0xa2a7b5, 1.0), 0.0);

    // Footer: version pill.
    let footer = format!("Folio {version}  ·  Written in Rust  ·  Local-first Markdown");
    c.text(&footer, 360.0, 412.0, 11.0, Font::Regular, hex(0x6e7383, 1.0), 0.3);

    // A hairline above the footer.
    unsafe {
        CGContextSaveGState(ctx);
        CGContextBeginPath(ctx);
        let a = p(230.0, 384.5);
        let b = p(490.0, 384.5);
        CGContextMoveToPoint(ctx, a.x, a.y);
        CGContextAddLineToPoint(ctx, b.x, b.y);
        CGContextSetLineWidth(ctx, 1.0);
        CGContextReplacePathWithStrokedPath(ctx);
        CGContextClip(ctx);
        c.linear(a, b, &[(0.0, [1.0, 1.0, 1.0, 0.0]), (0.5, [1.0, 1.0, 1.0, 0.09]), (1.0, [1.0, 1.0, 1.0, 0.0])]);
        CGContextRestoreGState(ctx);
    }
    let _ = Font::Bold;
    c
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("usage: dmg-art <out-dir> <version>");
    let version = args.next().unwrap_or_else(|| "2".into());
    std::fs::create_dir_all(&out).expect("create out dir");
    render(1.0, &version).save_png(&format!("{out}/background.png"), 72.0);
    render(2.0, &version).save_png(&format!("{out}/background@2x.png"), 144.0);
    println!("wrote {out}/background.png and background@2x.png");
}
