//! Lace 0.9.7 classic-Basics parity.
//!
//! These are the shared-token values Lace actually derives for the eight
//! classic Basics. They are deliberately exact: the themes are ports, so a
//! drift here is a fidelity bug rather than a palette refresh.

use lace_gpui::engine::build;
use lace_gpui::themes::by_name;

type Rgba = [u8; 4];

/// `(name, border, action-disabled glyph, highlight, selected text,
/// tooltip text, body text)`, from Lace's `build_theme` for the classic
/// `THEME_SPECS` entries.
const CLASSIC: [(&str, Rgba, Rgba, Rgba, Rgba, Rgba, Rgba); 8] = [
    (
        "midnight",
        [34, 39, 52, 85],
        [73, 77, 85, 255],
        [55, 95, 226, 255],
        [255, 255, 255, 255],
        [206, 211, 224, 255],
        [206, 211, 224, 255],
    ),
    (
        "dark",
        [46, 48, 49, 85],
        [92, 94, 95, 255],
        [0, 120, 212, 255],
        [254, 255, 255, 255],
        [202, 204, 207, 255],
        [202, 204, 207, 255],
    ),
    (
        "mocha",
        [71, 61, 56, 85],
        [113, 106, 102, 255],
        [200, 110, 60, 255],
        [36, 30, 27, 255],
        [235, 225, 210, 255],
        [235, 225, 210, 255],
    ),
    (
        "slate",
        [121, 126, 136, 85],
        [92, 97, 106, 255],
        [22, 70, 150, 255],
        [178, 185, 195, 255],
        [13, 17, 25, 255],
        [13, 17, 25, 255],
    ),
    (
        "caramel",
        [154, 150, 139, 85],
        [135, 132, 128, 255],
        [170, 89, 0, 255],
        [249, 241, 232, 255],
        [39, 34, 28, 255],
        [39, 34, 28, 255],
    ),
    (
        "neutral",
        [158, 159, 163, 85],
        [128, 129, 132, 255],
        [0, 104, 192, 255],
        [232, 237, 247, 255],
        [30, 33, 40, 255],
        [30, 33, 40, 255],
    ),
    (
        "cream",
        [213, 206, 194, 85],
        [155, 151, 142, 255],
        [176, 92, 0, 255],
        [255, 247, 236, 255],
        [48, 40, 32, 255],
        [48, 40, 32, 255],
    ),
    (
        "light",
        [212, 212, 215, 85],
        [162, 161, 164, 255],
        [0, 102, 192, 255],
        [238, 238, 241, 255],
        [45, 47, 52, 255],
        [45, 47, 52, 255],
    ),
];

#[test]
fn classic_themes_match_lace_shared_tokens() {
    for (name, border, disabled, highlight, selected_text, tooltip_text, body_text) in CLASSIC {
        let t = build(by_name(name).unwrap());

        // Lace's classic `_neutral_border` list, shared by every edge.
        assert_eq!(t.border_color, border, "{name}: core border");
        assert_eq!(t.border_color[3], 85, "{name}: classic idle alpha");
        assert_eq!(t.title_border_color, border, "{name}: title border");
        assert_eq!(t.sidepanel_border, border, "{name}: side-panel border");
        assert_eq!(t.tab_border_normal, border, "{name}: tab border");
        assert_eq!(
            t.sidebar_tab_border_normal, border,
            "{name}: sidebar tab border"
        );

        // Lace's one `_btn_disabled` list, shared by all action buttons.
        assert_eq!(
            t.title_button_disabled, disabled,
            "{name}: title disabled glyph"
        );
        assert_eq!(
            t.sidepanel_button_disabled, disabled,
            "{name}: side-panel disabled glyph"
        );
        assert_eq!(
            t.tab_close_btn_disabled, disabled,
            "{name}: tab-close disabled glyph"
        );

        // `PANEL.highlight` shares the accent list; `_accent_dim` follows
        // the accent's RGB but keeps its own 25% alpha.
        assert_eq!(t.accent_color, highlight, "{name}: accent");
        assert_eq!(t.highlight, highlight, "{name}: selection fill");
        assert_eq!(
            &t.overlay_color[..3],
            &highlight[..3],
            "{name}: overlay RGB"
        );
        assert_eq!(t.overlay_color[3], 64, "{name}: overlay alpha");
        assert_eq!(
            t.focus_border_color, highlight,
            "{name}: focus border follows accent"
        );
        assert_eq!(t.tab_border_active, highlight, "{name}: tab active border");
        assert_eq!(t.tab_indicator, highlight, "{name}: tab indicator");
        assert_eq!(
            t.sidebar_tab_border_active, highlight,
            "{name}: sidebar active border"
        );
        assert_eq!(t.sidebar_indicator, highlight, "{name}: sidebar indicator");
        assert_eq!(t.sidebar_badge_bg, highlight, "{name}: badge fill");
        assert_eq!(t.splitter_handle_hover, highlight, "{name}: splitter hover");

        assert_eq!(t.highlighted_text, selected_text, "{name}: selected text");
        assert_eq!(t.text, body_text, "{name}: body text");
        assert_eq!(t.tooltip_text, tooltip_text, "{name}: tooltip text");
    }
}
