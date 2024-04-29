#![no_std]
extern crate alloc;
extern crate mintyml;
extern crate wasm_bindgen;

use alloc::{format, string::String};
use js_sys::{JsString, Reflect};
use mintyml::{ConvertError, MetadataConfig, OutputConfig};
use wasm_bindgen::prelude::*;

fn to_js_error(e: ConvertError) -> JsValue {
    match e {
        ConvertError::Syntax { syntax_errors, src } => {
            let error = js_sys::Object::new();
            let message_key = JsString::from("message");
            let actual_key = JsString::from("actual");
            let start_key = JsString::from("start");
            let end_key = JsString::from("end");
            let expected_key = JsString::from("expected");

            let errors = syntax_errors
                .into_iter()
                .map(|e| {
                    let obj = js_sys::Object::new();
                    let _ = Reflect::set(
                        &obj,
                        &actual_key,
                        &src.get(e.range.start.position..e.range.end.position)
                            .unwrap_or("<end-of-file>")
                            .into(),
                    );
                    let _ = Reflect::set(&obj, &start_key, &e.range.start.position.into());
                    let _ = Reflect::set(&obj, &end_key, &e.range.end.position.into());

                    match e.kind {
                        mintyml::SyntaxErrorKind::InvalidEscape { .. } => {
                            let _ =
                                Reflect::set(&obj, &message_key, &"Invalid escape sequence".into());
                        }
                        mintyml::SyntaxErrorKind::ParseFailed { expected, .. } => {
                            let _ = Reflect::set(&obj, &message_key, &"Parsing failed".into());
                            let _ = Reflect::set(
                                &obj,
                                &expected_key,
                                &expected
                                    .into_iter()
                                    .map(|ex| JsString::from(format!("{ex}")))
                                    .collect::<js_sys::Array>(),
                            );
                        }
                        _ => {
                            let _ = Reflect::set(&obj, &message_key, &"Unknown error".into());
                        }
                    }
                    obj
                })
                .collect::<js_sys::Array>();
            let _ = Reflect::set(&error, &"syntaxErrors".into(), &errors);
            error.into()
        }
        _ => js_sys::Error::new("Unknown error").into(),
    }
}

#[wasm_bindgen]
pub fn convert(
    src: &str,
    xml: bool,
    indent: i32,
    complete_page: bool,
    special_tags: JsValue,
    metadata: JsValue,
) -> Result<String, JsValue> {
    let mut config = OutputConfig::new().xml(xml).complete_page(complete_page);

    if special_tags.is_object() {
        let field = |key: &str| {
            Reflect::get(&special_tags, &key.into()).map(|j| j.as_string().map(Into::into))
        };

        config.special_tags.emphasis = field("emphasis")?;
        config.special_tags.strong = field("strong")?;
        config.special_tags.underline = field("underline")?;
        config.special_tags.strike = field("strike")?;
        config.special_tags.quote = field("quote")?;
        config.special_tags.code = field("code")?;
        config.special_tags.code_block_container = field("codeBlockContainer")?;
    }

    if metadata.is_truthy() {
        let mut metadata_config = MetadataConfig::new();
        if metadata.is_object() {
            metadata_config.elements = Reflect::get(&metadata, &"elements".into())?.is_truthy();
        }
        config.metadata = metadata_config.into();
    }

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
