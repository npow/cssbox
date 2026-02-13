//! `layout-test-harness` — WPT test runner for the CSS layout engine.
//!
//! This crate provides tools for running Web Platform Tests against the
//! layout engine, supporting both reftest comparison and testharness.js
//! assertion extraction.

pub mod reftest;
pub mod runner;
pub mod testharness;
pub mod wpt_parser;
