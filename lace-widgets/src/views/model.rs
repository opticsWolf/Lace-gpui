//! The item model behind every item view: a tree of rows with cells.
//!
//! One store serves `QTreeWidget`, `QTableWidget` and `QListWidget` alike:
//! a table is a tree whose rows have no children, a list a table with one
//! column. Rows are addressed by a stable [`NodeId`] that survives sorting,
//! inserts and removals of other rows.
//!
//! Builders (`add_row`, `set`, ...) change the store quietly; wrap them in
//! `model.update(cx, |m, cx| { ...; cx.notify() })` and every view on the
//! model repaints. Changes a *user* makes through a view also emit a
//! [`ModelEvent`], so an app can tell its own writes from the user's.
//!
//! A *lazy* table ([`ItemModel::lazy_table`]) knows only its size: a view
//! asks for the rows it is about to draw ([`ModelEvent::FetchRows`]) and the
//! owner answers with [`ItemModel::fill_rows`], as Qt's `fetchMore`. Its
//! rows are read-only and its cache is bounded, so it serves a table of
//! millions of rows without holding them.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use gpui_kit::gpui::{EventEmitter, SharedString};

use crate::controls::CheckState;

/// A row's stable id.
pub type NodeId = usize;

/// One cell's data (Qt's `EditRole`).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Value {
    /// Nothing.
    #[default]
    Empty,
    /// Text.
    Text(SharedString),
    /// An integer.
    Int(i64),
    /// A float.
    Float(f64),
    /// A flag.
    Bool(bool),
}

impl Value {
    /// The text a plain cell shows; floats with `decimals` places (None: as is).
    pub fn display(&self, decimals: Option<usize>) -> SharedString {
        match self {
            Value::Empty => SharedString::default(),
            Value::Text(t) => t.clone(),
            Value::Int(i) => i.to_string().into(),
            Value::Float(f) => match decimals {
                Some(d) => format!("{f:.d$}").into(),
                None => f.to_string().into(),
            },
            Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        }
    }

    /// As a number, if it is one (or text that parses as one).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            Value::Bool(b) => Some(*b as u8 as f64),
            Value::Text(t) => t.trim().parse().ok(),
            Value::Empty => None,
        }
    }

    /// As a flag.
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Text(t) => !t.is_empty(),
            Value::Empty => false,
        }
    }

    /// Whether this is a number (right-aligned by default).
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Float(_))
    }

    /// Parse `text` into a value of this value's kind (text stays text).
    pub fn parse_like(&self, text: &str) -> Value {
        let t = text.trim();
        match self {
            Value::Int(_) => t.parse().map(Value::Int).unwrap_or_else(|_| self.clone()),
            Value::Float(_) => t.parse().map(Value::Float).unwrap_or_else(|_| self.clone()),
            Value::Bool(_) => Value::Bool(matches!(t, "1" | "true" | "yes" | "on")),
            _ => Value::Text(text.to_string().into()),
        }
    }

    /// Sort order: numbers numerically, text case-insensitively, empty first.
    pub fn compare(&self, other: &Value) -> Ordering {
        match (self, other) {
            (Value::Empty, Value::Empty) => Ordering::Equal,
            (Value::Empty, _) => Ordering::Less,
            (_, Value::Empty) => Ordering::Greater,
            (Value::Text(a), Value::Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
                _ => a.display(None).to_lowercase().cmp(&b.display(None).to_lowercase()),
            },
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Text(s.to_string().into())
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s.into())
    }
}
impl From<SharedString> for Value {
    fn from(s: SharedString) -> Self {
        Value::Text(s)
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}
impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v as i64)
    }
}
impl From<usize> for Value {
    fn from(v: usize) -> Self {
        Value::Int(v as i64)
    }
}
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

/// A row.
#[derive(Clone, Debug, Default)]
pub struct Node {
    /// Cells, by column.
    pub cells: Vec<Value>,
    /// Parent row; None for top-level rows.
    pub parent: Option<NodeId>,
    /// Child rows, in display order.
    pub children: Vec<NodeId>,
    /// Check state, for user-checkable rows (`ItemIsUserCheckable`).
    pub check: Option<CheckState>,
    /// Shown expanded.
    pub expanded: bool,
    /// Enabled (`ItemIsEnabled`).
    pub enabled: bool,
    /// Cells may be edited (`ItemIsEditable`).
    pub editable: bool,
}

/// A change made by the user through a view.
#[derive(Clone, Debug, PartialEq)]
pub enum ModelEvent {
    /// A cell was edited.
    Edited {
        /// Row.
        node: NodeId,
        /// Column.
        column: usize,
    },
    /// A row's check state changed (also emitted for rows a tri-state
    /// parent changed).
    Checked {
        /// Row.
        node: NodeId,
        /// New state.
        state: CheckState,
    },
    /// Rows were sorted.
    Sorted {
        /// Column.
        column: usize,
        /// Ascending.
        ascending: bool,
    },
    /// A view is about to draw rows of a lazy table it does not hold:
    /// answer with [`ItemModel::fill_rows`] from `first` (fewer rows are
    /// fine; the view asks again for the rest).
    FetchRows {
        /// First row.
        first: usize,
        /// Rows wanted.
        count: usize,
    },
}

/// Rows a lazy table asks for at once, by default.
pub const FETCH_CHUNK: usize = 256;
/// Rows a lazy table keeps, by default.
pub const CACHE_LIMIT: usize = 20_000;

/// A lazy table's size and the rows it holds now.
#[derive(Clone, Debug)]
struct Lazy {
    rows: usize,
    cache: BTreeMap<usize, Vec<Value>>,
    /// Chunks asked for and not yet filled.
    pending: BTreeSet<usize>,
    chunk: usize,
    limit: usize,
}

/// The item store.
#[derive(Clone, Debug, Default)]
pub struct ItemModel {
    headers: Vec<SharedString>,
    nodes: Vec<Option<Node>>,
    roots: Vec<NodeId>,
    /// Parents derive their check state from their children (`ItemIsAutoTristate`).
    pub auto_tristate: bool,
    revision: u64,
    lazy: Option<Lazy>,
}

impl EventEmitter<ModelEvent> for ItemModel {}

impl ItemModel {
    /// An empty model with column `headers`.
    pub fn new(headers: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self { headers: headers.into_iter().map(Into::into).collect(), auto_tristate: true, ..Default::default() }
    }

    /// A flat table of `rows`.
    pub fn table(
        headers: impl IntoIterator<Item = impl Into<SharedString>>,
        rows: impl IntoIterator<Item = Vec<Value>>,
    ) -> Self {
        let mut m = Self::new(headers);
        for r in rows {
            m.add_row(None, r);
        }
        m
    }

    /// A one-column list of `items`.
    pub fn list(items: impl IntoIterator<Item = impl Into<Value>>) -> Self {
        let mut m = Self::new([""]);
        for i in items {
            m.add_row(None, vec![i.into()]);
        }
        m
    }

    /// A read-only table of `rows` rows whose cells are fetched as a view
    /// shows them: the view emits [`ModelEvent::FetchRows`] and the owner
    /// answers with [`fill_rows`](Self::fill_rows). Rows are addressed by
    /// their index (a row's [`NodeId`] is its index) and never sort.
    pub fn lazy_table(headers: impl IntoIterator<Item = impl Into<SharedString>>, rows: usize) -> Self {
        let mut m = Self::new(headers);
        m.lazy = Some(Lazy {
            rows,
            cache: BTreeMap::new(),
            pending: BTreeSet::new(),
            chunk: FETCH_CHUNK,
            limit: CACHE_LIMIT,
        });
        m
    }

    /// Ask for `rows` rows at once (at least 1).
    pub fn fetch_chunk(mut self, rows: usize) -> Self {
        if let Some(l) = &mut self.lazy {
            l.chunk = rows.max(1);
        }
        self
    }

    /// Keep at most `rows` rows (at least one chunk); the rows farthest
    /// from the last fill go first.
    pub fn cache_limit(mut self, rows: usize) -> Self {
        if let Some(l) = &mut self.lazy {
            l.limit = rows.max(l.chunk);
        }
        self
    }

    /// A table fetched as it is shown.
    pub fn is_lazy(&self) -> bool {
        self.lazy.is_some()
    }

    /// New data for a lazy table: `rows` rows, nothing held, nothing asked.
    pub fn reset_rows(&mut self, rows: usize) {
        if let Some(l) = &mut self.lazy {
            l.rows = rows;
            l.cache.clear();
            l.pending.clear();
            self.touch();
        }
    }

    /// Rows of a lazy table from `first` (past the end: dropped).
    pub fn fill_rows(&mut self, first: usize, rows: Vec<Vec<Value>>) {
        let Some(l) = &mut self.lazy else { return };
        let end = first.saturating_add(rows.len()).min(l.rows);
        for (r, cells) in (first..end).zip(rows) {
            l.cache.insert(r, cells);
        }
        if end > first {
            for chunk in first / l.chunk..=(end - 1) / l.chunk {
                l.pending.remove(&chunk);
            }
        }
        if l.cache.len() > l.limit {
            let mid = (first + end) / 2;
            let mut keys: Vec<usize> = l.cache.keys().copied().collect();
            keys.sort_by_key(|&r| std::cmp::Reverse(r.abs_diff(mid)));
            let extra = l.cache.len() - l.limit;
            for r in keys.into_iter().take(extra) {
                l.cache.remove(&r);
            }
        }
        self.touch();
    }

    /// The chunks of `rows` a lazy table neither holds nor has asked for,
    /// as row ranges (adjacent chunks joined); they count as asked for now.
    pub fn request(&mut self, rows: Range<usize>) -> Vec<Range<usize>> {
        let Some(l) = &mut self.lazy else { return Vec::new() };
        let end = rows.end.min(l.rows);
        if rows.start >= end {
            return Vec::new();
        }
        let mut out: Vec<Range<usize>> = Vec::new();
        for chunk in rows.start / l.chunk..=(end - 1) / l.chunk {
            let span = chunk * l.chunk..((chunk + 1) * l.chunk).min(l.rows);
            if l.pending.contains(&chunk) || span.clone().all(|r| l.cache.contains_key(&r)) {
                continue;
            }
            l.pending.insert(chunk);
            match out.last_mut() {
                Some(last) if last.end == span.start => last.end = span.end,
                _ => out.push(span),
            }
        }
        out
    }

    /// Forget what was asked for and not filled (a fetch that failed), so
    /// a view asks again.
    pub fn forget_pending(&mut self) {
        if let Some(l) = &mut self.lazy {
            l.pending.clear();
        }
    }

    /// Whether a lazy table holds row `row` now.
    pub fn is_loaded(&self, row: usize) -> bool {
        self.lazy.as_ref().is_some_and(|l| l.cache.contains_key(&row))
    }

    /// Whether row `id` exists.
    pub fn contains(&self, id: NodeId) -> bool {
        match &self.lazy {
            Some(l) => id < l.rows,
            None => self.node(id).is_some(),
        }
    }

    /// A row's cells (a lazy row not held yet: none).
    pub fn cells(&self, id: NodeId) -> Vec<Value> {
        match &self.lazy {
            Some(l) => l.cache.get(&id).cloned().unwrap_or_default(),
            None => self.node(id).map(|n| n.cells.clone()).unwrap_or_default(),
        }
    }

    /// Bumped on every change; views cache by it.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    /// Column headers.
    pub fn headers(&self) -> &[SharedString] {
        &self.headers
    }

    /// Column count.
    pub fn column_count(&self) -> usize {
        self.headers.len().max(1)
    }

    /// Replace the headers.
    pub fn set_headers(&mut self, headers: impl IntoIterator<Item = impl Into<SharedString>>) {
        self.headers = headers.into_iter().map(Into::into).collect();
        self.touch();
    }

    /// Append a row under `parent` (None: top level); returns its id.
    pub fn add_row(&mut self, parent: Option<NodeId>, cells: Vec<Value>) -> NodeId {
        self.insert_row(parent, usize::MAX, cells)
    }

    /// Insert a row under `parent` at `index` (clamped).
    pub fn insert_row(&mut self, parent: Option<NodeId>, index: usize, cells: Vec<Value>) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Some(Node { cells, parent, enabled: true, editable: true, ..Default::default() }));
        let list = match parent {
            Some(p) => &mut self.node_mut(p).expect("no such parent").children,
            None => &mut self.roots,
        };
        let at = index.min(list.len());
        list.insert(at, id);
        self.touch();
        id
    }

    /// Remove `id` and its subtree.
    pub fn remove(&mut self, id: NodeId) {
        let Some(node) = self.nodes.get_mut(id).and_then(Option::take) else { return };
        match node.parent {
            Some(p) => {
                if let Some(pn) = self.node_mut(p) {
                    pn.children.retain(|&c| c != id);
                }
            }
            None => self.roots.retain(|&c| c != id),
        }
        for c in node.children {
            self.remove(c);
        }
        if let Some(p) = node.parent {
            self.refresh_tristate(p);
        }
        self.touch();
    }

    /// Remove every row.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.roots.clear();
        self.reset_rows(0);
        self.touch();
    }

    /// A row.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id).and_then(Option::as_ref)
    }

    fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id).and_then(Option::as_mut)
    }

    /// Top-level rows.
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    /// Children of `parent` (None: the top level).
    pub fn children(&self, parent: Option<NodeId>) -> &[NodeId] {
        match parent {
            Some(p) => self.node(p).map(|n| n.children.as_slice()).unwrap_or(&[]),
            None => &self.roots,
        }
    }

    /// A cell (Empty when missing).
    pub fn value(&self, id: NodeId, column: usize) -> Value {
        if let Some(l) = &self.lazy {
            return l.cache.get(&id).and_then(|r| r.get(column).cloned()).unwrap_or_default();
        }
        self.node(id).and_then(|n| n.cells.get(column).cloned()).unwrap_or_default()
    }

    /// Set a cell.
    pub fn set(&mut self, id: NodeId, column: usize, value: impl Into<Value>) {
        if let Some(n) = self.node_mut(id) {
            if n.cells.len() <= column {
                n.cells.resize(column + 1, Value::Empty);
            }
            n.cells[column] = value.into();
            self.touch();
        }
    }

    /// Make a row user-checkable with `state`, or not checkable (None).
    pub fn set_checkable(&mut self, id: NodeId, state: Option<CheckState>) {
        if let Some(n) = self.node_mut(id) {
            n.check = state;
            self.touch();
        }
    }

    /// Enable or disable a row.
    pub fn set_enabled(&mut self, id: NodeId, enabled: bool) {
        if let Some(n) = self.node_mut(id) {
            n.enabled = enabled;
            self.touch();
        }
    }

    /// Allow or forbid editing a row's cells.
    pub fn set_editable(&mut self, id: NodeId, editable: bool) {
        if let Some(n) = self.node_mut(id) {
            n.editable = editable;
            self.touch();
        }
    }

    /// Expand or collapse a row.
    pub fn set_expanded(&mut self, id: NodeId, expanded: bool) {
        if let Some(n) = self.node_mut(id) {
            n.expanded = expanded;
            self.touch();
        }
    }

    /// Expand every row.
    pub fn expand_all(&mut self) {
        for n in self.nodes.iter_mut().flatten() {
            n.expanded = true;
        }
        self.touch();
    }

    /// Set a row's check state. Under [`auto_tristate`](Self::auto_tristate)
    /// it spreads to the checkable rows below and the parents above follow.
    /// Returns every row whose state changed.
    pub fn set_check(&mut self, id: NodeId, state: CheckState) -> Vec<(NodeId, CheckState)> {
        let mut changed = Vec::new();
        self.set_check_down(id, state, &mut changed);
        if self.auto_tristate {
            let mut up = self.node(id).and_then(|n| n.parent);
            while let Some(p) = up {
                let before = self.node(p).and_then(|n| n.check);
                self.refresh_tristate(p);
                let after = self.node(p).and_then(|n| n.check);
                if before != after {
                    changed.push((p, after.unwrap_or_default()));
                }
                up = self.node(p).and_then(|n| n.parent);
            }
        }
        self.touch();
        changed
    }

    fn set_check_down(&mut self, id: NodeId, state: CheckState, changed: &mut Vec<(NodeId, CheckState)>) {
        let Some(n) = self.node_mut(id) else { return };
        if n.check.is_some() && n.check != Some(state) {
            n.check = Some(state);
            changed.push((id, state));
        }
        if self.auto_tristate && state != CheckState::Partial {
            let kids = self.node(id).map(|n| n.children.clone()).unwrap_or_default();
            for c in kids {
                self.set_check_down(c, state, changed);
            }
        }
    }

    /// Recompute a tri-state parent from its checkable children.
    fn refresh_tristate(&mut self, id: NodeId) {
        if !self.auto_tristate {
            return;
        }
        let Some(n) = self.node(id) else { return };
        if n.check.is_none() {
            return;
        }
        let states: Vec<CheckState> = n.children.iter().filter_map(|&c| self.node(c).and_then(|k| k.check)).collect();
        if states.is_empty() {
            return;
        }
        let state = if states.iter().all(|&s| s == CheckState::Checked) {
            CheckState::Checked
        } else if states.iter().all(|&s| s == CheckState::Unchecked) {
            CheckState::Unchecked
        } else {
            CheckState::Partial
        };
        if let Some(n) = self.node_mut(id) {
            n.check = Some(state);
        }
    }

    /// Recompute every tri-state parent from its children (after building).
    pub fn sync_tristate(&mut self) {
        fn walk(m: &mut ItemModel, id: NodeId) {
            let kids = m.node(id).map(|n| n.children.clone()).unwrap_or_default();
            for c in kids {
                walk(m, c);
            }
            m.refresh_tristate(id);
        }
        for r in self.roots.clone() {
            walk(self, r);
        }
        self.touch();
    }

    /// Sort every level by `column`, stably (a lazy table: never).
    pub fn sort(&mut self, column: usize, ascending: bool) {
        if self.lazy.is_some() {
            return;
        }
        let key = |m: &ItemModel, a: &NodeId, b: &NodeId| {
            let o = m.value(*a, column).compare(&m.value(*b, column));
            if ascending { o } else { o.reverse() }
        };
        let mut roots = std::mem::take(&mut self.roots);
        roots.sort_by(|a, b| key(self, a, b));
        self.roots = roots;
        for i in 0..self.nodes.len() {
            let Some(mut kids) = self.node_mut(i).map(|n| std::mem::take(&mut n.children)) else { continue };
            kids.sort_by(|a, b| key(self, a, b));
            if let Some(n) = self.node_mut(i) {
                n.children = kids;
            }
        }
        self.touch();
    }

    /// The rows a view shows, in order, with their depth: children of
    /// expanded rows only (all rows when `flat`).
    pub fn visible(&self, flat: bool) -> Vec<(NodeId, usize)> {
        if let Some(l) = &self.lazy {
            return (0..l.rows).map(|r| (r, 0)).collect();
        }
        let mut out = Vec::with_capacity(self.nodes.len());
        let mut stack: Vec<(NodeId, usize)> = self.roots.iter().rev().map(|&r| (r, 0)).collect();
        while let Some((id, depth)) = stack.pop() {
            let Some(n) = self.node(id) else { continue };
            out.push((id, depth));
            if flat || n.expanded {
                stack.extend(n.children.iter().rev().map(|&c| (c, depth + 1)));
            }
        }
        out
    }

    /// Every row's cells, top-level rows in order (a table's value; a lazy
    /// table: the rows it holds now).
    pub fn rows(&self) -> Vec<Vec<Value>> {
        if let Some(l) = &self.lazy {
            return l.cache.values().cloned().collect();
        }
        self.roots.iter().filter_map(|&r| self.node(r).map(|n| n.cells.clone())).collect()
    }

    /// Row count, all levels.
    pub fn len(&self) -> usize {
        match &self.lazy {
            Some(l) => l.rows,
            None => self.nodes.iter().flatten().count(),
        }
    }

    /// No rows.
    pub fn is_empty(&self) -> bool {
        match &self.lazy {
            Some(l) => l.rows == 0,
            None => self.roots.is_empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> (ItemModel, NodeId, [NodeId; 3]) {
        let mut m = ItemModel::new(["Path", "Size"]);
        let src = m.add_row(None, vec!["src".into(), 6.into()]);
        let kids = [
            m.add_row(Some(src), vec!["b.rs".into(), 2.into()]),
            m.add_row(Some(src), vec!["a.rs".into(), 3.into()]),
            m.add_row(Some(src), vec!["c.rs".into(), 1.into()]),
        ];
        m.set_checkable(src, Some(CheckState::Unchecked));
        for k in kids {
            m.set_checkable(k, Some(CheckState::Unchecked));
        }
        (m, src, kids)
    }

    #[test]
    fn checking_a_child_makes_the_parent_partial_then_checked() {
        let (mut m, src, kids) = tree();
        m.set_check(kids[0], CheckState::Checked);
        assert_eq!(m.node(src).unwrap().check, Some(CheckState::Partial));
        m.set_check(kids[1], CheckState::Checked);
        let changed = m.set_check(kids[2], CheckState::Checked);
        assert_eq!(m.node(src).unwrap().check, Some(CheckState::Checked));
        assert!(changed.contains(&(src, CheckState::Checked)));
    }

    #[test]
    fn checking_a_parent_checks_its_children() {
        let (mut m, src, kids) = tree();
        let changed = m.set_check(src, CheckState::Checked);
        assert_eq!(changed.len(), 4);
        assert!(kids.iter().all(|&k| m.node(k).unwrap().check == Some(CheckState::Checked)));
    }

    #[test]
    fn visible_rows_follow_expansion() {
        let (mut m, src, _) = tree();
        assert_eq!(m.visible(false).len(), 1);
        m.set_expanded(src, true);
        assert_eq!(m.visible(false), vec![(0, 0), (1, 1), (2, 1), (3, 1)]);
        assert_eq!(m.visible(true).len(), 4);
    }

    #[test]
    fn sort_orders_children_and_keeps_ids() {
        let (mut m, src, kids) = tree();
        m.sort(1, true);
        assert_eq!(m.children(Some(src)), &[kids[2], kids[0], kids[1]]);
        m.sort(0, false);
        assert_eq!(m.children(Some(src)), &[kids[2], kids[0], kids[1]]);
        m.sort(0, true);
        assert_eq!(m.children(Some(src)), &[kids[1], kids[0], kids[2]]);
    }

    #[test]
    fn remove_drops_the_subtree_and_refreshes_the_parent() {
        let (mut m, src, kids) = tree();
        m.set_check(kids[0], CheckState::Checked);
        m.remove(kids[1]);
        m.remove(kids[2]);
        assert_eq!(m.children(Some(src)), &[kids[0]]);
        assert_eq!(m.node(src).unwrap().check, Some(CheckState::Checked));
        m.remove(src);
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn a_lazy_table_asks_for_the_chunks_it_lacks_once() {
        let mut m = ItemModel::lazy_table(["a", "b"], 1000).fetch_chunk(100).cache_limit(250);
        assert!(m.is_lazy() && m.len() == 1000 && m.contains(999) && !m.contains(1000));
        assert_eq!(m.request(150..260), vec![100..300], "whole chunks, joined");
        assert!(m.request(150..260).is_empty(), "asked for already");
        assert_eq!(m.request(950..2000), vec![900..1000], "clipped to the table");
        let rows: Vec<Vec<Value>> = (100..300).map(|r| vec![Value::Int(r as i64), "x".into()]).collect();
        m.fill_rows(100, rows);
        assert_eq!(m.value(250, 0), Value::Int(250));
        assert_eq!(m.cells(250), vec![Value::Int(250), Value::from("x")]);
        assert_eq!(m.value(50, 0), Value::Empty, "not fetched: empty");
        assert!(m.request(100..300).is_empty(), "held");
        m.forget_pending();
        assert_eq!(m.request(950..1000), vec![900..1000], "a failed fetch is asked again");
        m.fill_rows(900, (900..1000).map(|r| vec![Value::Int(r as i64)]).collect());
        assert_eq!(m.rows().len(), 250, "the cache keeps its limit");
        assert!(m.is_loaded(999) && m.is_loaded(150) && !m.is_loaded(149), "the rows farthest from the fill went");
        assert_eq!(m.request(100..200), vec![100..200], "a chunk partly dropped is asked again");
        let rev = m.revision();
        m.sort(0, false);
        assert_eq!(m.revision(), rev, "a lazy table never sorts");
        m.reset_rows(5);
        assert_eq!((m.len(), m.rows().len()), (5, 0));
        assert_eq!(m.request(0..10), vec![0..5]);
        assert_eq!(m.visible(true), (0..5).map(|r| (r, 0)).collect::<Vec<_>>());
    }

    #[test]
    fn values_parse_and_compare_by_kind() {
        assert_eq!(Value::Int(1).parse_like(" 42 "), Value::Int(42));
        assert_eq!(Value::Int(1).parse_like("x"), Value::Int(1));
        assert_eq!(Value::Float(0.0).parse_like("2.5"), Value::Float(2.5));
        assert_eq!(Value::Int(10).compare(&Value::Float(9.5)), Ordering::Greater);
        assert_eq!(Value::from("b").compare(&Value::from("A")), Ordering::Greater);
        assert_eq!(Value::Float(1.0).display(Some(2)), SharedString::from("1.00"));
    }
}
