use js_sys::{global, Array, ArrayBuffer, Function, JsString, Object, Reflect, JSON};
use regex::Regex;
use std::{collections::VecDeque, env, error::Error, fs, path::PathBuf};
use tools::{
    get_js_function, get_nested_property, get_value_from_json, get_value_from_obj, obj_not_exist,
    set_property, to_jss,
};
use wasm_bindgen::prelude::*;
use web_sys::console::{log_1, log_2, log_3, log_4, time, time_end};
mod tools;

#[wasm_bindgen(module = "fs")]
extern "C" {
    #[wasm_bindgen]
    pub fn readFileSync(path: &str, decode: &str) -> JsValue;
    #[wasm_bindgen]
    pub fn writeFileSync(path: &str, content: &str);
    #[wasm_bindgen]
    pub fn readdirSync(path: &str) -> Array;
    #[wasm_bindgen]
    pub fn statSync(path: &str) -> Object;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    pub fn require(name: &str) -> JsValue;
}

fn read_pages_json(path: &str) -> Result<JsValue, JsValue> {
    Ok(JSON::parse(&remove_comments(
        readFileSync(path, "utf-8")
            .as_string()
            .ok_or(JsValue::from_str("read pages json fail"))?,
    ))?)
}

fn get_abs_path(path: &str) -> Result<PathBuf, std::io::Error> {
    Ok(env::current_dir()?.join(path))
}

fn write_json_into_file(path: &str, content: JsValue) -> Result<(), Box<dyn Error>> {
    let content = JSON::stringify(&content.into())
        .expect("stringify json fail")
        .as_string()
        .ok_or("stringify json fail")?;
    writeFileSync(path, &content);

    Ok(())
}

fn remove_comments(pages: String) -> String {
    let multi_line_comment_re = Regex::new(r"/\*.*?\*/").expect("get multi_line regexp fail");
    let single_line_comment_re = Regex::new(r"(?m)^\s*//.*$").expect("get single_line regexp fail");

    let pages_no_multi_line = multi_line_comment_re.replace_all(&pages, "");
    let pages_no_comments = single_line_comment_re.replace_all(&pages_no_multi_line, "");

    pages_no_comments.into()
}


#[wasm_bindgen]
pub fn rewrite_dist_app_json(dist_path: &str, app_json_path: &str) -> Result<Array, JsValue> {
    log_1(&to_jss("\n-------------开始重写dist/pages.json-------------"));
    time();
    let async_packages = match read_pages_json(app_json_path) {
        Ok(json) => {
            let sub_packages: Array = get_value_from_json(&json, "subPackages")?.into();
            sub_packages.filter(&mut |item, _, _| {
                let pages: Array = get_value_from_json(&item, "pages")
                    .unwrap_or_else(|_e| Array::new().into())
                    .into();

                pages.is_undefined()
                    || pages.is_null()
                    || if pages.is_array() {
                        pages.length() == 0
                    } else {
                        false
                    }
            })
        }
        Err(e) => return Err(e),
    };

    let dist_app_json_path = format!("{dist_path}/app.json");
    let dist_app_json = require(&dist_app_json_path);

    let dist_sub_packages: Array = Reflect::get(&dist_app_json, &to_jss("subPackages"))?.into();
    let async_pkg_roots = Array::new();
    for pkg in async_packages {
        async_pkg_roots.push(&Reflect::get(&pkg, &to_jss("root"))?);
        dist_sub_packages.push(&pkg.into());
    }

    match write_json_into_file(&dist_app_json_path, dist_app_json) {
        Ok(_) => {
            log_1(&to_jss("\n-------------重写完毕，耗时："));
            time_end();

            Ok(async_pkg_roots)
        }
        Err(e) => Err(JsValue::from(e.to_string())),
    }
}

fn write_str_into_file(list: Vec<(&str, &str)>) -> std::io::Result<()> {
    for (path, content) in list.into_iter() {
        writeFileSync(&path, content)
    }
    Ok(())
}

#[wasm_bindgen]
pub fn inject_empty_wrapper(path: &str) -> Result<(), JsValue> {
    let js = r#"Component({})"#;
    let wxml = r#"<view style="display:none;" class="_div"></view>"#;
    let json = r#"{ "usingComponents": {}, "component": true }"#;
    match write_str_into_file(vec![
        (&format!("{path}/FuEmptyWrapper.js"), js),
        (&format!("{path}/FuEmptyWrapper.wxml"), wxml),
        (&format!("{path}/FuEmptyWrapper.json"), json),
    ]) {
        Ok(_) => Ok(()),
        Err(e) => Err(JsValue::from(e.to_string())),
    }
}

fn inject_placeholder(path: &str, reg: &Regex, json: JsValue) -> Result<(), JsValue> {
    let json = if json.is_undefined() { require(path) } else { json };
    // log_1(&json);
    let using_components: Object = get_value_from_json(&json, "usingComponents")?.into();

    if obj_not_exist(&using_components) {
        return Ok(());
    }
    let mut component_placeholder = Reflect::get(&json, &to_jss("componentPlaceholder"))?.into();
    for component_name in Reflect::own_keys(&using_components)? {
        let component_path_str = Reflect::get(&using_components, &component_name)?
            .as_string()
            .ok_or(JsValue::from_str("get compoent path str fail"))?;
        if reg.is_match(&component_path_str) {
            if obj_not_exist(&component_placeholder) {
                component_placeholder = Object::new();
                Reflect::set(
                    &json,
                    &to_jss("componentPlaceholder"),
                    &component_placeholder,
                )?;
            }
            if !&component_placeholder.has_own_property(&component_name) {
                // 引用状态还在的
                Reflect::set(
                    &component_placeholder,
                    &component_name,
                    &to_jss("fu-empty-wrapper").into(),
                )?;
            }
        }
    }
    if obj_not_exist(&component_placeholder) {
        return Ok(());
    }

    if !using_components.has_own_property(&to_jss("fu-empty-wrapper").into()) {
        set_property(
            &using_components,
            "fu-empty-wrapper",
            &to_jss("/FuEmptyWrapper"),
        )?;
    }

    match write_json_into_file(&path, json) {
        Ok(_) => Ok(()),
        Err(e) => Err(JsValue::from(e.to_string())),
    }
}

fn check_if_is_directory(path: &str) -> Result<bool, JsValue> {
    let stat = statSync(path);
    let is_directory = get_js_function("isDirectory", &stat)?;

    Ok(is_directory.call0(&stat)?.is_truthy())
}

fn traverse(
    base_path: &str,
    async_pkg_roots: Vec<String>,
    ignore_keywords: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let async_root_regexp = Regex::new(&async_pkg_roots.join("|"))?;
    let ignore_regexp = Regex::new(&ignore_keywords.join("|"))?;
    // 这里还是采用遍历 + 广度优先的方案，因为ignore_path的指针递归每层都在复制，避免内存膨胀
    // 时间复杂度都是O(n)，每个节点只过一遍
    
    let mut queue = VecDeque::new();

    queue.push_back(base_path.to_string());

    while !queue.is_empty() {
        let len = queue.len();

        for _i in 0..len {
            let path = &queue.pop_front().expect("get path from queue fail");
            if ignore_regexp.is_match(path) {
                log_1(&to_jss(&format!("-------------跳过：{}", path)));
                continue;
            }
            for file_name in readdirSync(path) {
                let name: String = file_name.as_string().unwrap();
                let file_path = format!("{}/{}", path, name);

                if name.ends_with(".json") {
                    match inject_placeholder(&file_path, &async_root_regexp, JsValue::undefined()) {
                        Err(e) => log_1(&e),
                        _ => {}
                    }
                } else {
                    match check_if_is_directory(&file_path) {
                        Ok(state) if state => queue.push_back(file_path),
                        Err(e) => log_1(&e),
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

/// 算了，写两个，一个直接处理全部，另一个搭配webpack plugin
/// 因为是业务方，所以这里就只针对`app.json`和`fuViewPackage`这俩即可，后续如果有再补充到ignore_paths即可。
/// 算了，直接改成关键字，省事
/// 关于async_pkg和ignore_path俩想了很久，可以考虑缓存到全局，但是这不是一个程序，只是单独的函数
/// 如果缓存到全局可能会有内存没有释放，并且每次执行都会创建一个，所以还是单独传入吧。。。
#[wasm_bindgen]
pub fn traverse_all_components_json(
    path: &str,
    async_pkg_roots: Vec<String>,
    ignore_keywords: Vec<String>,
) -> Result<(), JsValue> {
    log_1(&to_jss("-------------开始遍历组件json文件-------------"));
    time();

    match traverse(path, async_pkg_roots, ignore_keywords) {
        Err(e) => return Err(JsValue::from(e.to_string())),
        _ => {}
    };
    log_1(&to_jss("-------------遍历结束，耗时："));
    time_end();
    Ok(())
}

/// 处理组件的json文件
/// 这里还是准备只处理change的json，全处理太浪费资源了
#[wasm_bindgen]
pub fn traverse_some_components_json(
    dist_path: &str,
    files: Vec<Array>,
    async_pkg_roots: Vec<String>,
    ignore_keywords: Vec<String>,
) -> Result<(), JsValue> {
    log_1(&to_jss("-------------开始遍历组件json文件-------------"));
    time();
    let async_root_regexp = Regex::new(&async_pkg_roots.join("|"))
        .expect("get regexp fail when traverse some components json");
    let ignore_regexp = Regex::new(&ignore_keywords.join("|"))
        .expect("get regexp fail when traverse some components json");

    for el in files {
        let file_path = el.get(0).as_string().unwrap();
        let json = el.get(1);
        if ignore_regexp.is_match(&file_path) {
            log_1(&to_jss(&format!("-------------跳过：{}", file_path)));
            continue;
        }

        inject_placeholder(&format!("{dist_path}/{file_path}"), &async_root_regexp, json)?;
    }

    log_1(&to_jss("-------------遍历结束，耗时："));
    time_end();
    Ok(())
}
