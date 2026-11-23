//! Lace's icon provider: named sets of SVG icons, looked up by name.
//!
//! An app registers its icon sets once ([`register`]); widgets then ask for
//! an icon by its stem (`"undo"`) or, to pick one set, by `"set/stem"`.
//! A bare stem is looked up in the sets in the order they were registered,
//! so an app that registers its curated set before a shared pool gets the
//! curated drawing for a stem both hold (as Weave's Qt menus prefer
//! `menu_icons` over `widget_icons`). Lace's own dock icons are always
//! there, as the set [`DOCK`], looked up last.
//!
//! gpui draws an SVG as a mask in the element's text colour, so an icon
//! takes the colour of whatever it sits in, the way PySide Lace tints its
//! icons from the theme. [`element`] builds one.
//!
//! ```ignore
//! lace_gpui::icons::register(
//!     IconSet::from_static("app", &[("undo", include_bytes!("undo.svg"))]),
//!     cx,
//! );
//! let svg = lace_gpui::icons::lookup("undo", cx);
//! ```

use std::path::Path;
use std::sync::{Arc, OnceLock};

use gpui_kit::gpui::{px, svg, App, Global, Hsla, IntoElement, SharedString, Styled as _};

use crate::dock_icons::DockIcon;

/// An icon's SVG source.
pub type Svg = Arc<[u8]>;

/// The name of Lace's own dock icon set.
pub const DOCK: &str = "lace-dock";

/// A named set of icons, each an SVG under its stem.
#[derive(Clone, Debug, Default)]
pub struct IconSet {
    name: SharedString,
    icons: Vec<(SharedString, Svg)>,
}

impl IconSet {
    /// An empty set called `name`.
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self { name: name.into(), icons: Vec::new() }
    }

    /// A set over icons built into the binary (`include_bytes!`).
    pub fn from_static(name: impl Into<SharedString>, icons: &[(&'static str, &'static [u8])]) -> Self {
        let mut set = Self::new(name);
        for (stem, svg) in icons {
            set.insert(*stem, Arc::from(*svg));
        }
        set
    }

    /// A set over the `*.svg` files in `dir`, each under its file stem.
    /// Files that cannot be read are left out.
    pub fn from_dir(name: impl Into<SharedString>, dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let mut set = Self::new(name);
        let mut paths: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("svg")))
            .collect();
        paths.sort();
        for path in paths {
            let (Some(stem), Ok(bytes)) = (path.file_stem().and_then(|s| s.to_str()), std::fs::read(&path)) else {
                continue;
            };
            set.insert(stem.to_string(), Arc::from(bytes));
        }
        Ok(set)
    }

    /// Add `svg` under `stem`, replacing an icon of that stem.
    pub fn insert(&mut self, stem: impl Into<SharedString>, svg: Svg) {
        let stem = stem.into();
        match self.icons.binary_search_by(|(s, _)| s.as_ref().cmp(stem.as_ref())) {
            Ok(i) => self.icons[i].1 = svg,
            Err(i) => self.icons.insert(i, (stem, svg)),
        }
    }

    /// `self` with `svg` under `stem`.
    pub fn with(mut self, stem: impl Into<SharedString>, svg: Svg) -> Self {
        self.insert(stem, svg);
        self
    }

    /// The set's name.
    pub fn name(&self) -> &SharedString {
        &self.name
    }

    /// The icon under `stem`.
    pub fn get(&self, stem: &str) -> Option<&Svg> {
        self.icons.binary_search_by(|(s, _)| s.as_ref().cmp(stem)).ok().map(|i| &self.icons[i].1)
    }

    /// Its stems, sorted.
    pub fn stems(&self) -> impl Iterator<Item = &SharedString> {
        self.icons.iter().map(|(s, _)| s)
    }

    /// How many icons it holds.
    pub fn len(&self) -> usize {
        self.icons.len()
    }

    /// Whether it holds none.
    pub fn is_empty(&self) -> bool {
        self.icons.is_empty()
    }
}

/// Lace's dock icons as a set.
fn dock_set() -> &'static IconSet {
    static SET: OnceLock<IconSet> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = IconSet::new(DOCK);
        for icon in DockIcon::ALL {
            set.insert(icon.stem(), Arc::from(icon.svg()));
        }
        set
    })
}

/// The registered icon sets (a gpui global).
#[derive(Clone, Debug, Default)]
pub struct Icons {
    sets: Vec<IconSet>,
}

impl Global for Icons {}

impl Icons {
    /// Add `set` after those registered before it, or put it in the place
    /// of a set of the same name.
    pub fn register(&mut self, set: IconSet) {
        match self.sets.iter_mut().find(|s| s.name == set.name) {
            Some(old) => *old = set,
            None => self.sets.push(set),
        }
    }

    /// The set called `name` (Lace's dock set included).
    pub fn set(&self, name: &str) -> Option<&IconSet> {
        self.sets.iter().find(|s| s.name == name).or_else(|| (name == DOCK).then(dock_set))
    }

    /// The icon called `name`: `"set/stem"` from that set, or a bare stem
    /// from the first set that holds it, Lace's dock set last.
    pub fn lookup(&self, name: &str) -> Option<Svg> {
        if let Some((set, stem)) = name.split_once('/') {
            return self.set(set)?.get(stem).cloned();
        }
        self.sets.iter().chain(std::iter::once(dock_set())).find_map(|s| s.get(name)).cloned()
    }

    /// The registered sets' names, in lookup order (the dock set last).
    pub fn names(&self) -> Vec<SharedString> {
        let mut out: Vec<_> = self.sets.iter().map(|s| s.name.clone()).collect();
        if !out.iter().any(|n| n == DOCK) {
            out.push(DOCK.into());
        }
        out
    }
}

/// Register `set` with the app's icon provider (see [`Icons::register`]).
pub fn register(set: IconSet, cx: &mut App) {
    cx.default_global::<Icons>().register(set);
}

/// The icon called `name` (see [`Icons::lookup`]); Lace's dock icons even
/// before anything is registered.
pub fn lookup(name: &str, cx: &App) -> Option<Svg> {
    match cx.try_global::<Icons>() {
        Some(icons) => icons.lookup(name),
        None => Icons::default().lookup(name),
    }
}

/// An `size` x `size` element drawing `svg` in `color`.
pub fn element(svg_data: &Svg, size: f32, color: Hsla) -> impl IntoElement {
    svg().data(svg_data).flex_none().size(px(size)).text_color(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &[u8] = b"<svg id='a'/>";
    const B: &[u8] = b"<svg id='b'/>";

    #[test]
    fn a_bare_stem_comes_from_the_first_set_that_holds_it() {
        let mut icons = Icons::default();
        icons.register(IconSet::from_static("menu", &[("undo", A)]));
        icons.register(IconSet::from_static("widget", &[("undo", B), ("cut", B)]));
        assert_eq!(&*icons.lookup("undo").unwrap(), A);
        assert_eq!(&*icons.lookup("cut").unwrap(), B);
        assert_eq!(&*icons.lookup("widget/undo").unwrap(), B, "a qualified name picks its set");
        assert!(icons.lookup("menu/cut").is_none());
        assert!(icons.lookup("nothing").is_none());
    }

    #[test]
    fn the_dock_icons_are_a_set_looked_up_last() {
        let mut icons = Icons::default();
        assert!(icons.lookup("pin").is_some(), "there before anything is registered");
        assert!(icons.lookup("lace-dock/close_tab").is_some());
        icons.register(IconSet::from_static("app", &[("pin", A)]));
        assert_eq!(&*icons.lookup("pin").unwrap(), A, "an app's set wins over the dock's");
        assert_eq!(icons.names(), vec![SharedString::from("app"), SharedString::from(DOCK)]);
        assert_eq!(dock_set().len(), DockIcon::ALL.len());
    }

    #[test]
    fn registering_a_name_again_replaces_that_set_in_place() {
        let mut icons = Icons::default();
        icons.register(IconSet::from_static("one", &[("x", A)]));
        icons.register(IconSet::from_static("two", &[("x", B)]));
        icons.register(IconSet::from_static("one", &[("y", A)]));
        assert_eq!(&*icons.lookup("x").unwrap(), B, "`one` no longer holds x");
        assert_eq!(icons.names()[0], "one", "and keeps its place");
    }

    #[test]
    fn a_set_reads_a_folder_of_svgs() {
        let dir = std::env::temp_dir().join(format!("lace-icons-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("undo.svg"), A).unwrap();
        std::fs::write(dir.join("notes.txt"), B).unwrap();
        let set = IconSet::from_dir("dir", &dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(set.stems().map(|s| s.to_string()).collect::<Vec<_>>(), ["undo"]);
        assert_eq!(&**set.get("undo").unwrap(), A);
    }
}
