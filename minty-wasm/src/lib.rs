#![no_std]
extern crate alloc;
extern crate mintyml;
extern crate wasm_bindgen;

use alloc::{format, string::String};
use mintyml::{ConvertError, OutputConfig};
use wasm_bindgen::prelude::*;

fn to_js_error(e: ConvertError) -> JsError {
    JsError::new(&format!("{e:?}"))
}

#[wasm_bindgen]
pub fn convert(src: &str, xml: bool, indent: i32) -> Result<String, JsError> {
    let mut config = OutputConfig::new().xml(xml);
    if indent >= 0 {
        config.indent = Some(
            core::iter::repeat(' ')
                .take(indent as usize)
                .collect::<String>()
                .into(),
        )
    }
    mintyml::convert(src, config).map_err(to_js_error)
}
