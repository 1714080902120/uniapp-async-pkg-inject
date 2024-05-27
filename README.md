# uniapp => mp-weixin 分包异步化方案
基于：
1. [微信小程序分包异步化](https://developers.weixin.qq.com/miniprogram/dev/framework/subpackages.html)
2. https://ask.dcloud.net.cn/article/id-39622 大佬的方案


## 解决的问题
解决当前uniapp不支持分包异步化的问题`（底层原理是转换成小程序代码时pages.json会过滤pages为空的分包）`

比如pages.json中：
```js
    // ...
    "subPackages": [
        {
            "root": "asyncComp",
            "name": "asyncComp",
            "pages": []
        }
        // ...
    ]
    // ...
```
最终是会被过滤掉的

## 原理
原理很简单，最终uniapp的代码都会转换成符合微信小程序要求的代码，所以我们可以在uniapp => miniprogram完成之后，我们直接操作`pages.json`，把我们被过滤掉的给补充回去。

**得益于组件并不会被编译进`vendor`，所以我们可以直接把一堆组件所在的文件夹直接定义成一个包！**

当然，只是补充`pages.json`还是不够的，根据微信官方的要求，如果分包异步化的分包还没加载，那么组件就不能被使用，这个时候如果不用`componentPlaceholder`的话会直接报错，所以我们还需要给所有的组件的`.json`修改一波。

比如组件`xxx.vue`引用了`xx`组件，我们把这个`xx`迁移至需要分包异步化：
```js
import xx from '@/asyncComp/xx/xx.vue';
```

转换成微信小程序的代码后，会生成我们熟悉的`xxx.json`:
```js
{
  "navigationStyle": "custom",
  "enablePullDownRefresh": true,
  "usingComponents": {
    // ...
    "xx": "/asyncComp/xx/xx",
    // ...
  },
  // ...
}
```
我们还需要给这个`xx`补充一个未加载前的占位元素，一般可以是`view`：
```js
  "componentPlaceholder": {
    "xx": "view"
  }
```

那么到这原理就基本解释完了。

## 注意点
1. 存在时序问题的，因为分包可能还未加载，如果此时你对包中的某个组件使用`ref`访问，那么这个时候你拿到的只是一个空`view`，因为此时组件还没加载只是个占位符。
2. 体验感问题，体验感还是很重要的，有必要的话还是放回主包。
3. 奇形怪状的组件（比较特殊的，具体看你个人的分析，比如全局注册（只能放主包））还是放回主包稳妥点。
4. *我就是要放主包！*

## 怎么分包
差点忘了
1. 在`pages.json`中添加你要分包的文件夹：
```js
    // ...
    "subPackages": [
        {
            "root": "asyncComp",
            "name": "asyncComp",
            "pages": []
        }
        // ...
    ]
    // ...
```
2. 把你准备要分出去的组件放到你要分的包里面
3. 修改引用这个组件的路径，让它指向这个分包里的组件（为什么要这么做？因为要确保它被参与打包）
4. 完了！
----------------------
## 如何使用
1. npm 引入 https://www.npmjs.com/package/uniapp-async-pkg-inject
```bash
npm install uniapp-async-pkg-inject -D
```
2. 自定义一个`webpack`插件：
```js
const { rewrite_dist_app_json, inject_empty_wrapper, traverse_all_components_json, traverse_some_components_json } = require('uniapp-async-pkg-inject/index');

class AutoInjectFuviewPackageDev {
    constructor() {
        this.isInject = false;
    }
    apply(compiler) {
        const base_path = process.cwd();
        const mode = process.env.NODE_ENV === 'production' ? 'build' : 'dev';
        const distPath = path.join(base_path, `/dist/${mode}/mp-weixin`);
        const appJsonPath = path.join(base_path, `/src/pages.json`);
        // 需要忽略的路径，注意执行时会把这些装换成一个全局匹配的正则，所以你需要确保路径不会被误伤
        const ignoreKeywords = ["app.json", "ext.json", "static", "node-modules", "uni_modules", "common"];
        // 呃，这个属实是语言不同的无奈，用于存储需要异步化分包的名字
        let asyncPkgRoots = [];
        // 二次编译时拿到差量
        const needed = []
        // 是否需要重写app.json
        let containAppJson = false;

        compiler.hooks.assetEmitted.tap("collect change data", (fileName, content) => {
            if (this.isInject) {
                if (fileName.endsWith(".json")) {
                    // 如果有改动过`pages.json`，这里就需要重写一次`pages.json`
                    // 理论上不需要重新处理整个包的json，因为如果改pages.json，那一定会触发对应目标页面的json改变
                    // 不然就是uniapp的bug了，所以这里直接重写app.json并且拿差量的处理即可
                    if (fileName.includes('pages.json')) {
                        containAppJson = true
                    }
                    try {
                        needed.push([fileName, JSON.parse(content.toString())]);
                    } catch (error) {
                        console.error('someting went wrong when parse content')
                    }
                }
            }
        })


        compiler.hooks.done.tap("inject async pkg after emit assets", (compilation, callback) => {
            if (!this.isInject) {
                this.isInject = true;

                // 重写`app.json`
                asyncPkgRoots = rewrite_dist_app_json(distPath, appJsonPath);
                // 注入占位组件（这一步也可以不要，不过你要自行调整逻辑，让占位变成你想要的）
                inject_empty_wrapper(distPath);
                // 遍历`dist/dev/mp-weixin`下非ignore的所有组件json
                traverse_all_components_json(distPath, asyncPkgRoots, ignoreKeywords);
            }

            // 除了第一次之外，剩下的直接处理差量的即可
            if (needed.length > 0) {
                // console.log(22, needed)

                if (containAppJson) {
                    asyncPkgRoots = rewrite_dist_app_json(distPath, appJsonPath);
                }
                // 只处理差量的文件
                traverse_some_components_json(distPath, needed, asyncPkgRoots, ignoreKeywords);
                // needed = []
                needed.splice(0, needed.length);
            } else {
                console.log("------------本次改动不包含组件引用改动------------")
            }
            containAppJson = false;

            callback && callback()
        })

    }
}
```
### dev
1. 我们在[`hooks.done`](https://webpack.js.org/api/compiler-hooks/#done)阶段执行我们的方法
2. 我们在[`hooks.assetEmitted`](https://webpack.js.org/api/compiler-hooks/#assetemitted)注册了回调，获取此次重新编译的`assets`或者叫`chunk`，然后给`done`时使用。
这么做之后 只需要重写app.json文件一次（后续改动涉及`app.json`还是需要重写），并且除了第一次，后面都是差量调整，耗时算下来会少很多。

### production
生产环境就简单多了，直接全部遍历即可

3. 在注册`webpack`插件的地方引入，比如`vue.config.js`：
```js
plugins.push(
    process.env.NODE_ENV === 'production' ?
    new AutoInjectFuviewPackageProd() :
    new AutoInjectFuviewPackageDev()
);
```
