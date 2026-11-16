//! Item views: tree, table and list over one [`ItemModel`] (Qt's model /
//! view, with `QTreeWidget`-style convenience built in).

pub mod model;
pub mod view;

pub use model::{ItemModel, ModelEvent, Node, NodeId, Value};
pub use view::{Align, CellCtx, Column, Delegate, ItemView, Mode, Resize, SelectionMode, ViewEvent};
