//! Perceptual colour maths for theme derivation.
//!
//! Rust port of Lace's `lace/color_science.py` (Apache-2.0, (c) opticsWolf),
//! which is itself built on Björn Ottosson's OKLab. One place that knows how
//! to lighten, darken, mix and check colours, in a space where equal steps
//! look equal. HLS lightness is not that space: +0.1 L on a yellow is a much
//! larger visible change than on a blue. OKLCH (OKLab in polar form) is close
//! enough, and cheap.
//!
//! Colours are `[u8; 4]` (r, g, b, a) — the format the theme builder already
//! uses — so nothing here depends on any GUI crate. Contrast follows WCAG 2.x.
//! All math runs in `f64` end to end, matching Python's double precision.

/// OKLCH lightness below which a colour counts as dark.
pub const DARK_L: f64 = 0.6;

/// An sRGB colour with alpha, `[r, g, b, a]`, 0..255.
pub type Rgba = [u8; 4];

pub const WHITE: Rgba = [255, 255, 255, 255];
pub const BLACK: Rgba = [0, 0, 0, 255];

// ---------------------------------------------------------------------------
// sRGB <-> linear <-> OKLab <-> OKLCH
// ---------------------------------------------------------------------------

#[inline]
fn to_linear(c: u8) -> f64 {
    let c = c as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
fn from_linear(c: f64) -> f64 {
    // Returns 0..255 (like Lace's _from_linear).
    let c = if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    c * 255.0
}

fn linear_to_oklab(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
    let l_ = cbrt(l);
    let m_ = cbrt(m);
    let s_ = cbrt(s);
    (
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    )
}

fn oklab_to_linear(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let l_ = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m_ = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s_ = (l - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    (
        4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_,
        -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_,
        -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_,
    )
}

#[inline]
fn cbrt(v: f64) -> f64 {
    // copysign(|v|^(1/3), v): the real cube root, matching Lace's
    // `copysign(abs(v) ** (1/3), v)`.
    v.signum() * v.abs().powf(1.0 / 3.0)
}

/// sRGB -> OKLab. Returns `(L, a, b)`; alpha is dropped.
pub fn to_oklab(rgba: Rgba) -> (f64, f64, f64) {
    linear_to_oklab(to_linear(rgba[0]), to_linear(rgba[1]), to_linear(rgba[2]))
}

/// sRGB -> OKLCH. Returns `(L, C, h)`: L 0..1, chroma, hue in degrees 0..360.
pub fn to_oklch(rgba: Rgba) -> (f64, f64, f64) {
    let (l, a, b) = to_oklab(rgba);
    let c = a.hypot(b);
    let h = b.atan2(a).to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };
    (l, c, h)
}

#[inline]
fn in_gamut(lin: (f64, f64, f64)) -> bool {
    // eps 1e-6, matching Lace.
    lin.0 >= -1e-6
        && lin.0 <= 1.0 + 1e-6
        && lin.1 >= -1e-6
        && lin.1 <= 1.0 + 1e-6
        && lin.2 >= -1e-6
        && lin.2 <= 1.0 + 1e-6
}

#[inline]
fn pack(lin: (f64, f64, f64), alpha: u8) -> Rgba {
    let ch = |c: f64| -> u8 {
        let v = from_linear(c.clamp(0.0, 1.0));
        (v.round() as i64).clamp(0, 255) as u8
    };
    [ch(lin.0), ch(lin.1), ch(lin.2), alpha]
}

/// OKLCH to sRGB, gamut-mapped by reducing chroma (L and hue kept).
/// Alpha comes back 255.
pub fn from_oklch(l: f64, c: f64, h: f64) -> Rgba {
    from_oklch_a(l, c, h, 255)
}

/// OKLCH to sRGB with alpha, gamut-mapped by reducing chroma (L and hue kept).
pub fn from_oklch_a(l: f64, c: f64, h: f64, a: u8) -> Rgba {
    let l = l.clamp(0.0, 1.0);
    let c = c.max(0.0);
    let hr = h.to_radians();
    let (ca, sa) = (hr.cos(), hr.sin());

    let lin = oklab_to_linear(l, c * ca, c * sa);
    if in_gamut(lin) {
        return pack(lin, a);
    }
    // Binary search the largest chroma that fits, 24 iterations (Lace).
    let (mut lo, mut hi) = (0.0_f64, c);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if in_gamut(oklab_to_linear(l, mid * ca, mid * sa)) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    pack(oklab_to_linear(l, lo * ca, lo * sa), a)
}

// ---------------------------------------------------------------------------
// WCAG
// ---------------------------------------------------------------------------

/// `fg` over an opaque `bg` (plain sRGB alpha blending, as gpui composites).
pub fn composite(fg: Rgba, bg: Rgba) -> Rgba {
    let t = fg[3] as f64 / 255.0;
    let mix = |f: u8, b: u8| -> u8 {
        (f as f64 * t + b as f64 * (1.0 - t)).round() as u8
    };
    [
        mix(fg[0], bg[0]),
        mix(fg[1], bg[1]),
        mix(fg[2], bg[2]),
        255,
    ]
}

/// WCAG relative luminance.
pub fn relative_luminance(rgba: Rgba) -> f64 {
    0.2126 * to_linear(rgba[0]) + 0.7152 * to_linear(rgba[1]) + 0.0722 * to_linear(rgba[2])
}

/// WCAG contrast of `fg` drawn on `bg`; a translucent `fg` is composited first.
pub fn contrast_ratio(fg: Rgba, bg: Rgba) -> f64 {
    let fg = if fg[3] < 255 { composite(fg, bg) } else { fg };
    let (l1, l2) = (relative_luminance(fg), relative_luminance(bg));
    let (hi, lo) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

/// True when the colour's OKLCH lightness is below [`DARK_L`].
pub fn is_dark(rgba: Rgba) -> bool {
    to_oklch(rgba).0 < DARK_L
}

/// Perceptual distance: Euclidean in OKLab (alpha ignored). ~0.02 is just
/// noticeable.
pub fn delta_e(a: Rgba, b: Rgba) -> f64 {
    let (la, aa, ba) = to_oklab(a);
    let (lb, ab, bb) = to_oklab(b);
    ((la - lb).powi(2) + (aa - ab).powi(2) + (ba - bb).powi(2)).sqrt()
}

/// `target`, or as far from `orig` toward it as `max_de` allows (OKLab).
pub fn toward(orig: Rgba, target: Rgba, max_de: f64) -> Rgba {
    let d = delta_e(orig, target);
    if d <= max_de {
        return target;
    }
    // Linear in OKLab, but 8-bit rounding can land past the cap (near black a
    // whole step is ~0.04), so search on the rounded result.
    let mut lo = 0.0_f64;
    let mut hi = max_de / d;
    if delta_e(orig, mix(orig, target, hi)) <= max_de {
        return mix(orig, target, hi);
    }
    for _ in 0..20 {
        let mid = (lo + hi) / 2.0;
        if delta_e(orig, mix(orig, target, mid)) <= max_de {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    mix(orig, target, lo)
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// Shift perceptual lightness by `dL`, keeping hue and chroma.
pub fn step(rgba: Rgba, dl: f64) -> Rgba {
    let (l, c, h) = to_oklch(rgba);
    from_oklch_a(l + dl, c, h, rgba[3])
}

/// [`step`] "toward contrast": `abs(dl)` in the direction away from the colour
/// itself — lighter for a dark colour, darker for a light one.
pub fn step_contrast(rgba: Rgba, dl: f64) -> Rgba {
    let (l, c, h) = to_oklch(rgba);
    let dl = if l < DARK_L { dl.abs() } else { -dl.abs() };
    from_oklch_a(l + dl, c, h, rgba[3])
}

/// `a` blended toward `b` by `t` (0 = a, 1 = b), in OKLab.
pub fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let (la, aa, ba) = to_oklab(a);
    let (lb, ab, bb) = to_oklab(b);
    let lin = oklab_to_linear(
        la + (lb - la) * t,
        aa + (ab - aa) * t,
        ba + (bb - ba) * t,
    );
    let alpha = (a[3] as f64 + (b[3] as f64 - a[3] as f64) * t).round() as u8;
    pack(lin, alpha)
}

/// `fg` with the smallest lightness shift that reaches `ratio` on `bg`.
///
/// Hue is kept (chroma only shrinks where the gamut forces it) and so is
/// alpha. `fg` comes back unchanged when it already passes. When no lightness
/// reaches the ratio, the best reachable colour is returned.
pub fn ensure_contrast(fg: Rgba, bg: Rgba, ratio: f64) -> Rgba {
    if contrast_ratio(fg, bg) >= ratio {
        return fg;
    }
    let (l, c, h) = to_oklch(fg);

    let mut best: Option<Rgba> = None;
    let mut best_shift = f64::INFINITY;
    for &end in &[1.0_f64, 0.0] {
        if contrast_ratio(from_oklch_a(end, c, h, fg[3]), bg) < ratio {
            continue;
        }
        // Contrast grows monotonically moving away from bg toward this end.
        // The predicate tests the rounded 8-bit colour, so `hi` always
        // passes — no nudging after the search.
        let (mut lo, mut hi) = (l, end);
        for _ in 0..30 {
            let mid = (lo + hi) / 2.0;
            if contrast_ratio(from_oklch_a(mid, c, h, fg[3]), bg) >= ratio {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        if (hi - l).abs() < best_shift {
            best = Some(from_oklch_a(hi, c, h, fg[3]));
            best_shift = (hi - l).abs();
        }
    }
    if let Some(b) = best {
        return b;
    }
    let ends = [
        from_oklch_a(1.0, c, h, fg[3]),
        from_oklch_a(0.0, c, h, fg[3]),
    ];
    if contrast_ratio(ends[0], bg) >= contrast_ratio(ends[1], bg) {
        ends[0]
    } else {
        ends[1]
    }
}

/// Readable text colour for `bg`.
///
/// Picks whichever of `prefer` (typically the theme's light and dark text)
/// contrasts most, then lifts it to `ratio` if it falls short.
pub fn on_color(bg: Rgba, prefer: [Rgba; 2], ratio: f64) -> Rgba {
    let (a, b) = (prefer[0], prefer[1]);
    let best = if contrast_ratio(a, bg) >= contrast_ratio(b, bg) { a } else { b };
    ensure_contrast(best, bg, ratio)
}

/// Alias of [`composite`] — blend `top` over `bottom` in sRGB.
pub fn blend_rgba(top: Rgba, bottom: Rgba) -> Rgba {
    composite(top, bottom)
}

// ---------------------------------------------------------------------------
// Hex parsing / formatting
// ---------------------------------------------------------------------------

/// Parse `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` (with or without `#`, and
/// `0x` accepted). Short forms expand each digit (`f` -> `ff`). Panics-free:
/// falls back to opaque black on garbage.
pub fn hex(s: &str) -> Rgba {
    let s = s.trim();
    let s = s
        .strip_prefix('#')
        .or_else(|| s.strip_prefix("0x"))
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    let v: Vec<u8> = (0..s.len() / 2)
        .filter_map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
        .collect();
    match v.len() {
        3 => [v[0], v[1], v[2], 255],
        4 => [v[0], v[1], v[2], v[3]],
        _ => {
            // Short forms: each character is half a byte, doubled.
            let n: Vec<u32> = s.chars().filter_map(|c| c.to_digit(16)).collect();
            match n.len() {
                3 => [
                    (n[0] * 17) as u8,
                    (n[1] * 17) as u8,
                    (n[2] * 17) as u8,
                    255,
                ],
                4 => [
                    (n[0] * 17) as u8,
                    (n[1] * 17) as u8,
                    (n[2] * 17) as u8,
                    (n[3] * 17) as u8,
                ],
                _ => [0, 0, 0, 255],
            }
        }
    }
}

/// Format as `#rrggbb` (alpha 255) or `#rrggbbaa` (otherwise).
pub fn hex_str(c: Rgba) -> String {
    if c[3] == 255 {
        format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
    }
}

/// Build an [`Rgba`] from components.
pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba {
    [r, g, b, a]
}

// ---------------------------------------------------------------------------
// Tests (14)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn oklab_round_trip() {
        for c in [[40u8, 42, 54, 255], [189, 147, 249, 255], [0, 43, 54, 255], [253, 246, 227, 255]] {
            let (l, a, b) = to_oklab(c);
            let back = pack(oklab_to_linear(l, a, b), 255);
            assert!(delta_e(c, back) < 0.01, "{:?} -> {:?}", c, back);
        }
    }

    #[test]
    fn white_black_lightness_extremes() {
        let (lw, _, _) = to_oklch(WHITE);
        let (lb, _, _) = to_oklch(BLACK);
        assert!(close(lw, 1.0, 1e-6), "white L {}", lw);
        assert!(close(lb, 0.0, 1e-6), "black L {}", lb);
    }

    #[test]
    fn hue_pure_primaries() {
        let (_, _, hr) = to_oklch([255, 0, 0, 255]);
        let (_, _, hg) = to_oklch([0, 255, 0, 255]);
        let (_, _, hb) = to_oklch([0, 0, 255, 255]);
        assert!(close(hr, 29.23, 0.5), "red hue {}", hr);
        assert!(close(hg, 142.5, 0.5), "green hue {}", hg);
        assert!(close(hb, 264.05, 0.5), "blue hue {}", hb);
    }

    #[test]
    fn gamut_mapping_keeps_lightness_and_hue() {
        // An out-of-gamut neon: chroma must shrink, L and hue must hold.
        let (l0, _, h0) = to_oklch([120, 120, 255, 255]);
        let mapped = from_oklch(l0, 0.4, h0);
        let (l1, c1, h1) = to_oklch(mapped);
        assert!(c1 < 0.4, "chroma should shrink, got {}", c1);
        assert!(close(l1, l0, 1e-3), "L {} vs {}", l1, l0);
        assert!(close(h1, h0, 1e-3), "h {} vs {}", h1, h0);
    }

    #[test]
    fn contrast_ratio_reference_values() {
        assert!(close(contrast_ratio(BLACK, WHITE), 21.0, 0.01));
        assert!(close(contrast_ratio(WHITE, BLACK), 21.0, 0.01));
        assert!(close(contrast_ratio(BLACK, BLACK), 1.0, 0.01));
    }

    #[test]
    fn contrast_ratio_composites_translucent_fg() {
        // 50% white over black is #808080; ratio vs black ~5.3.
        let grey = contrast_ratio([255, 255, 255, 128], BLACK);
        assert!(close(grey, contrast_ratio([128, 128, 128, 255], BLACK), 0.3));
    }

    #[test]
    fn ensure_contrast_reaches_target() {
        let fg = [131, 148, 150, 255]; // solarized base05
        let bg = [0, 43, 54, 255];
        let out = ensure_contrast(fg, bg, 7.0);
        assert!(contrast_ratio(out, bg) >= 7.0);
        // Already passing: untouched.
        assert_eq!(ensure_contrast(WHITE, BLACK, 4.5), WHITE);
    }

    #[test]
    fn ensure_contrast_keeps_alpha() {
        let fg = [131, 148, 150, 128];
        let bg = [0, 43, 54, 255];
        let out = ensure_contrast(fg, bg, 7.0);
        assert_eq!(out[3], 128);
    }

    #[test]
    fn on_color_prefers_higher_contrast() {
        let dark_text = on_color([20, 20, 20, 255], [WHITE, BLACK], 4.5);
        assert_eq!(dark_text, WHITE);
        let light_text = on_color([230, 230, 230, 255], [WHITE, BLACK], 4.5);
        assert_eq!(light_text, BLACK);
    }

    #[test]
    fn step_moves_lightness_only() {
        let c = [40, 42, 54, 255];
        let (l0, c0, h0) = to_oklch(c);
        let up = step(c, 0.05);
        let (l1, c1, h1) = to_oklch(up);
        // 8-bit rounding shifts L by up to ~half a count (~0.004 near black),
        // and wobbles the small chroma of a near-neutral colour.
        assert!(close(l1 - l0, 0.05, 0.01), "dL {}", l1 - l0);
        assert!(close(c1, c0, 5e-3) && close(h1, h0, 5.0));
    }

    #[test]
    fn step_contrast_direction() {
        let dark = step_contrast([10, 10, 10, 255], 0.05);
        let light = step_contrast([240, 240, 240, 255], 0.05);
        assert!(to_oklch(dark).0 > 0.05, "dark steps lighter");
        assert!(to_oklch(light).0 < 0.95, "light steps darker");
    }

    #[test]
    fn mix_endpoints_and_midpoint() {
        assert_eq!(mix([10u8, 20, 30, 255], [200, 210, 220, 255], 0.0), [10, 20, 30, 255]);
        assert_eq!(mix([10u8, 20, 30, 255], [200, 210, 220, 255], 1.0), [200, 210, 220, 255]);
        // OKLab is perceptual: the halfway point of black/white sits well
        // below sRGB grey 128 (L 0.5 -> linear 0.125 -> ~#636363).
        let mid = mix([0, 0, 0, 255], [255, 255, 255, 255], 0.5);
        assert!((mid[0] as i32 - 99).abs() <= 2, "oklab mid grey {:?}", mid);
        // ...and stays neutral.
        assert_eq!(mid[0], mid[1]);
        assert_eq!(mid[1], mid[2]);
    }

    #[test]
    fn delta_e_zero_for_equal() {
        assert!(close(delta_e([1u8, 2, 3, 255], [1, 2, 3, 255]), 0.0, 1e-12));
        // OKLab L spans 0..1, so black->white is a distance of ~1 (f64
        // rounding puts white's L a hair under 1.0).
        assert!(delta_e([0, 0, 0, 255], [255, 255, 255, 255]) > 0.999);
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(hex("#282a36"), [40, 42, 54, 255]);
        assert_eq!(hex("282a36"), [40, 42, 54, 255]);
        assert_eq!(hex("#fff"), [255, 255, 255, 255]);
        assert_eq!(hex("#1e1e2e80"), [30, 30, 46, 128]);
        assert_eq!(hex_str([40, 42, 54, 255]), "#282a36");
        assert_eq!(hex_str([30, 30, 46, 128]), "#1e1e2e80");
        // Garbage falls back to opaque black, never panics.
        assert_eq!(hex("zz"), [0, 0, 0, 255]);
    }
}
