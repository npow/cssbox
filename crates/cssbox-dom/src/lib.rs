//! `cssbox-dom` — HTML/CSS parsing and style resolution for cssbox.
//!
//! This crate provides the bridge between HTML/CSS input and the `cssbox-core`
//! layout engine. It parses HTML documents, resolves CSS styles through the
//! cascade, and builds the box tree that the layout engine requires.
//!
//! # Usage
//!
//! ```rust
//! use cssbox_dom::computed::html_to_box_tree;
//! use cssbox_core::geometry::Size;
//! use cssbox_core::layout::{compute_layout, FixedWidthTextMeasure};
//!
//! let html = r#"<div style="width: 200px; height: 100px"></div>"#;
//! let tree = html_to_box_tree(html);
//! let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));
//! ```

pub mod cascade;
pub mod computed;
pub mod css;
pub mod dom;
pub mod html;
