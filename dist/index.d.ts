/* tslint:disable */
/* eslint-disable */
/**
* @param {string} dist_path
* @param {string} app_json_path
* @returns {Array<any>}
*/
export function rewrite_dist_app_json(dist_path: string, app_json_path: string): Array<any>;
/**
* @param {string} path
*/
export function inject_empty_wrapper(path: string): void;
/**
* 算了，写两个，一个直接处理全部，另一个搭配webpack plugin
* 因为是业务方，所以这里就只针对`app.json`和`fuViewPackage`这俩即可，后续如果有再补充到ignore_paths即可。
* 算了，直接改成关键字，省事
* 关于async_pkg和ignore_path俩想了很久，可以考虑缓存到全局，但是这不是一个程序，只是单独的函数
* 如果缓存到全局可能会有内存没有释放，并且每次执行都会创建一个，所以还是单独传入吧。。。
* @param {string} path
* @param {(string)[]} async_pkg_roots
* @param {(string)[]} ignore_keywords
*/
export function traverse_all_components_json(path: string, async_pkg_roots: (string)[], ignore_keywords: (string)[]): void;
/**
* 处理组件的json文件
* 这里还是准备只处理change的json，全处理太浪费资源了
* @param {string} dist_path
* @param {(Array<any>)[]} files
* @param {(string)[]} async_pkg_roots
* @param {(string)[]} ignore_keywords
*/
export function traverse_some_components_json(dist_path: string, files: (Array<any>)[], async_pkg_roots: (string)[], ignore_keywords: (string)[]): void;
