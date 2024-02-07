#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate rs_typed_parser;
extern crate either;

pub(crate) mod ast;
pub(crate) mod ir;
pub(crate) mod utils;
pub(crate) mod escape;