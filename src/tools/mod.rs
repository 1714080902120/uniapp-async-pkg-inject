use js_sys::JsString;

pub fn to_jss (s: &str) -> JsString {
  JsString::from(s)
}

use js_sys::{Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

pub fn get_nested_property(
    node: &JsValue,
    keys: &Vec<&str>,
    depth: usize,
) -> Result<JsValue, JsValue> {
    if depth >= keys.len() {
        return Ok(node.clone());
    }
    match node.is_object() {
        true => {
            let next = Reflect::get(&node, &JsValue::from(keys[depth]))?;
            get_nested_property(&next, keys, depth + 1)
        }
        _ => Err(JsValue::from(format!(
            "current depth is not object, key: {}",
            keys[0.min(depth - 1)],
        ))),
    }
}

pub fn set_property(target: &Object, key: &str, value: &JsValue) -> Result<bool, JsValue> {
    Reflect::set(target, &JsString::from(key), value)
}

pub fn new_obj(keys_vals: &Vec<(&str, JsValue)>) -> Result<Object, JsValue> {
    let obj = Object::new();

    for (key, value) in keys_vals {
        set_property(&obj, key, value)?;
    }

    Ok(obj)
}

pub fn call_js_get(this: &Object, arg: &str) -> Result<JsValue, JsValue> {
    get_js_function("get", this)?.call1(this, &JsString::from(arg))
}

pub fn get_js_function(name: &str, target: &Object) -> Result<Function, JsValue> {
    Reflect::get(&target, &JsValue::from(name))?.dyn_into()
}

pub fn obj_not_exist (obj: &Object) -> bool {
  obj.is_undefined() || obj.is_null()
}

pub fn get_value_from_json (obj: &JsValue, key: &str) -> Result<JsValue, JsValue> {
  Reflect::get(&obj, &to_jss(key))
}

pub fn get_value_from_obj (obj: &Object, key: &str) -> Result<JsValue, JsValue> {
  Reflect::get(&obj, &to_jss(key))
}

