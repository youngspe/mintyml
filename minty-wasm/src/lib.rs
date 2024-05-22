#![no_std]
extern crate alloc;
extern crate mintyml;
extern crate wasm_bindgen;

use alloc::{format, string::String};
use js_sys::{JsString, Reflect};
use mintyml::{error::LocationRange, ConvertError, MetadataConfig, OutputConfig};
use wasm_bindgen::prelude::*;

fn to_js_error(e: ConvertError) -> JsValue {
    struct Context {
        message_key: JsString,
        actual_key: JsString,
        start_key: JsString,
        end_key: JsString,
        expected_key: JsString,
    }

    let cx = Context {
        message_key: JsString::from("message"),
        actual_key: JsString::from("actual"),
        start_key: JsString::from("start"),
        end_key: JsString::from("end"),
        expected_key: JsString::from("expected"),
    };

    let error = js_sys::Object::new();

    fn process_error(
        obj: &js_sys::Object,
        range: LocationRange,
        message: impl core::fmt::Display,
        cx: &Context,
    ) {
        let _ = Reflect::set(obj, &cx.start_key, &range.start.position.into());
        let _ = Reflect::set(obj, &cx.end_key, &range.end.position.into());
        let _ = Reflect::set(obj, &cx.message_key, &format!("{message}").into());
    }

    match e {
        ConvertError::Syntax { syntax_errors, src } => {
            let errors = syntax_errors
                .into_iter()
                .map(|e| {
                    let obj = js_sys::Object::new();
                    let range = e.range;
                    process_error(
                        &obj,
                        range,
                        e.display_with_src(&src, &Default::default()),
                        &cx,
                    );

                    match e.kind {
                        mintyml::SyntaxErrorKind::ParseFailed { expected, .. } => {
                            if range != LocationRange::INVALID {
                                let _ = Reflect::set(
                                    &obj,
                                    &cx.actual_key,
                                    &if range.start.position >= src.len() {
                                        "<end-of-file>".into()
                                    } else {
                                        range.slice(&src).into()
                                    },
                                );
                            }
                            let _ = Reflect::set(
                                &obj,
                                &cx.expected_key,
                                &expected
                                    .into_iter()
                                    .map(|ex| JsString::from(format!("{ex}")))
                                    .collect::<js_sys::Array>(),
                            );
                        }
                        _ => (),
                    }
                    obj
                })
                .collect::<js_sys::Array>();
            let _ = Reflect::set(&error, &"syntaxErrors".into(), &errors);
            error.into()
        }

        ConvertError::Semantic {
            semantic_errors,
            src,
        } => {
            let errors = semantic_errors
                .into_iter()
                .map(|e| {
                    let obj = js_sys::Object::new();
                    let range = e.range;
                    process_error(
                        &obj,
                        range,
                        e.display_with_src(&src, &Default::default()),
                        &cx,
                    );
                    obj
                })
                .collect::<js_sys::Array>();
            let _ = Reflect::set(&error, &"syntaxErrors".into(), &errors);
            error.into()
        }
        e => js_sys::Error::new(&format!("{e}")).into(),
    }
}

fn get_special_tags(config: &mut OutputConfig, special_tags: JsValue) -> Result<(), JsValue> {
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
    Ok(())
}

pub fn convert_inner(
    src: &str,
    xml: Option<bool>,
    indent: i32,
    complete_page: Option<bool>,
    special_tags: JsValue,
    metadata: JsValue,
    fail_fast: Option<bool>,
) -> Result<String, (Option<String>, JsValue)> {
    let mut config = OutputConfig::new();

    config.xml = xml;
    config.complete_page = complete_page;
    config.fail_fast = fail_fast;

    get_special_tags(&mut config, special_tags).map_err(|e| (None, e))?;

    if metadata.is_truthy() {
        let mut metadata_config = MetadataConfig::new();
        if metadata.is_object() {
            metadata_config.elements = Reflect::get(&metadata, &"elements".into())
                .map_err(|e| (None, e))?
                .is_truthy();
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
    mintyml::convert_forgiving(src, config).map_err(|(out, e)| (out, to_js_error(e)))
}

#[wasm_bindgen]
pub fn convert(
    src: &str,
    xml: Option<bool>,
    indent: i32,
    complete_page: Option<bool>,
    special_tags: JsValue,
    metadata: JsValue,
    fail_fast: Option<bool>,
) -> Result<JsValue, JsValue> {
    let (success, out, err) = match convert_inner(
        src,
        xml,
        indent,
        complete_page,
        special_tags,
        metadata,
        fail_fast,
    ) {
        Ok(out) => (true, Some(out), None),
        Err((out, e)) => (false, out, Some(e)),
    };

    let mut obj = js_sys::Object::new();
    Reflect::set(&mut obj, &"success".into(), &success.into())?;
    Reflect::set(&mut obj, &"output".into(), &out.into())?;
    Reflect::set(&mut obj, &"error".into(), &err.into())?;
    Ok(obj.into())
}
