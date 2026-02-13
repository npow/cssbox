//! `cssbox-core` — A standalone CSS layout engine.
//!
//! This crate implements CSS layout algorithms: block, inline, float,
//! positioning, flexbox, grid, and table. It takes a tree of styled nodes
//! as input and produces computed positions and sizes as output.
//!
//! # Usage
//!
//! ```rust
//! use cssbox_core::tree::BoxTreeBuilder;
//! use cssbox_core::style::ComputedStyle;
//! use cssbox_core::geometry::Size;
//! use cssbox_core::layout::{compute_layout, FixedWidthTextMeasure};
//!
//! let mut builder = BoxTreeBuilder::new();
//! let root = builder.root(ComputedStyle::block());
//! // ... add children ...
//! let tree = builder.build();
//!
//! let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));
//! let root_rect = result.bounding_rect(tree.root());
//! ```

#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]

pub mod block;
pub mod box_model;
pub mod flex;
pub mod float;
pub mod fragment;
pub mod geometry;
pub mod grid;
pub mod inline;
pub mod layout;
pub mod position;
pub mod style;
pub mod table;
pub mod tree;
pub mod values;
