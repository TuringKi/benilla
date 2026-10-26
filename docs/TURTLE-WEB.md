# Turtle 中文客户端的 Web 构建与局域网运行

此版本已使用 Turtle 1.18.1 中文资源完成实际浏览器登录、进入游戏和界面验证。
游戏资源由本机资源服务读取，不放入 Git 仓库。

## 构建

需要 Rust、Linux 原生构建依赖 `pkg-config libasound2-dev libudev-dev`，以及
`web-setup.sh` 安装的 WASM 工具链。已验证 Rust 1.98.1、wasm-bindgen 0.2.122、
WASI SDK 27、Binaryen 130。

```bash
bash scripts/web-setup.sh
CARGO_BUILD_JOBS=10 bash scripts/web-build.sh
cargo build --locked --release -p wenilla-host
```

网页和 WASM 输出到 `web/dist/`。WASM 与生成的 JavaScript 必须使用同一次构建的文件。

## 运行

手动运行时先设置 `GAME_HOST` 为游戏服主机；推荐使用下节的参数化部署脚本。

```bash
target/release/wenilla-host \
  --www web/dist \
  --data /path/to/client/Data \
  --upstream "$GAME_HOST" \
  --auth-port 3725 \
  --world-port 8091
```

本次验证的混合服务端要求登录认证使用 **1.18.1 / build 7272**，世界认证使用
**build 5875**。`web/index.html` 已配置 `client_build: '7272'`、
`world_client_build: '5875'` 和 `turtle_compat: '1'`。连接其他服务端时按其要求调整；
不配置 build 覆盖时，协议库仍默认使用 5875。

浏览器连接地址由部署脚本输出，`host` 参数是登录服地址。世界端口由服务端
角色列表中的地址决定，需与部署参数 `--world-port` 一致。

## 参数化部署：局域网与 VPN HTTPS

需要 Linux、Python 3、nginx、OpenSSL。首次构建还需按 `scripts/web-setup.sh`
安装 Rust/WASM 工具链。所有网络地址均由运行参数传入，仓库中不保存部署地址。

先设置本机变量（将尖括号占位内容替换成自己的值）：

```bash
WEB_IP='<网页服务所在的本机地址>'
VPN_IP='<可选的本机 VPN 地址>'
GAME_HOST='<登录服和世界服所在的主机>'
WOW_DATA='<客户端 Data 目录>'
ADDONS_DIR='<可选的独立 AddOns 目录>'
```

首次构建并启动：

```bash
python3 scripts/deploy-web.py up --build \
  --listen-ip "$WEB_IP" \
  --game-host "$GAME_HOST" \
  --data "$WOW_DATA" \
  --addons "$ADDONS_DIR" \
  --auth-port 3725 --world-port 8091
```

如需同时提供 VPN 入口，再添加 `--listen-ip "$VPN_IP"`。只填写本机已配置的接口
地址，不使用通配监听；未安装插件时省略 `--addons`。脚本会打印游戏入口和 CA 证书
下载地址，访问端需信任该 CA 才能正常使用 HTTPS/WebGPU。

默认端口：内部后端 8090、HTTP 8093、HTTPS 8443。可用 `--backend-port`、
`--http-port`、`--https-port` 修改。内部后端只监听本机回环接口。

已有构建产物时，省略 `--build`；产物默认取自 `web/dist`。可通过 `--www` 指定
其他构建目录，或通过 `--host-bin` 指定已有的 `wenilla-host` 可执行文件。
`--build` 与 `--host-bin` 不同时使用；自定义 Cargo 产物目录用 `CARGO_TARGET_DIR`。

```bash
python3 scripts/deploy-web.py up \
  --listen-ip "$WEB_IP" --listen-ip "$VPN_IP" \
  --game-host "$GAME_HOST" --data "$WOW_DATA" --addons "$ADDONS_DIR" \
  --auth-port 3725 --world-port 8091 \
  --host-bin '<已有 wenilla-host 可执行文件>'
```

重复执行 `up` 会发布新静态文件并重载 nginx；后端命令未变化时保留现有后端进程。
更新程序后，访问端刷新页面。已有诊断接收器时加 `--diagnostics-port` 指定其端口。旧测试入口 `render-test/` 始终共用主入口文件，避免漏更新。

```bash
python3 scripts/deploy-web.py status
python3 scripts/deploy-web.py stop
# 按本机保存的参数重新启动（不重新构建）：
bash .web-deploy/start.local.sh
```

运行配置、证书、私钥、日志、PID 和发布文件默认存于被 Git 忽略的 `.web-deploy/`，
目录仅允许当前用户访问。`--state-dir` 可以指定其他目录；仓库内的自定义目录必须
已被 Git 忽略。后续 `up/status/stop` 使用同一目录。`stop` 只停止该目录管理的进程。
发布保留先前版本，私钥和游戏资源不复制到 Git。

已有 CA 时，首次 `up` 加 `--ca-cert '<原 CA 证书>' --ca-key '<原 CA 私钥>'`；
以后会复用状态目录中的 CA。增加监听地址时重新执行带完整地址列表的 `up`，
服务器证书会覆盖所有传入地址，不要求客户端重新安装同一个 CA。

此部署供可信局域网或私有 VPN 使用，数据接口没有用户认证，不应用于公开互联网。

## 界面修复与诊断

MPQ 的 listfile 可能漏掉实际可按文件名读取的成员。网页端现在将目录作为存在提示，
对未列出的资源执行一次真实存在性检查并缓存；读取文件不再被目录缺项阻断。
这修复了中文字体和基础 Lua 脚本被误判缺失、继而中断技能图标和窗口更新的问题。

Turtle 模式还为 Lua 提供 `_G`，并保留该客户端自带的设置菜单。
启动异常会显示在页面上；右上角“界面诊断”可以查看选定控件状态和界面错误。

需要保存浏览器诊断到本机时，运行：

```bash
python3 scripts/ui-report-server.py
```

并在 HTTPS server 内增加：

```nginx
location = /ui-report {
    client_max_body_size 64k;
    proxy_pass http://localhost:8092;
}
```

接收器默认监听 loopback 的 8092 端口，输出 `scripts/ui-browser-reports.jsonl`。
页面只记录界面/渲染错误和选定控件属性。关闭诊断接收器不影响游戏运行。

已完成 WASM 构建、V8/gzip 校验、497 项协议测试、13 项 host 测试、12 项网页测试，
以及浏览器中实际目录和刻意删减目录的资源查找回归验证。资源探针源码为
`crates/benilla-formats/examples/web_asset_probe.rs`；最终游戏运行由实际浏览器复测确认。

## 与目标仓库同步

此分支基于 `TuringKi/benilla` 的 `84d7bd7c` 整合 Web 支持。Web 分支原始来源是
`Arnesen/wenilla` 的 `83dde612`，通过一个汇总提交 rebase 到目标 `main`，保留目标仓库的更新。
整合时适配了浏览器本地时区、登录报文时区偏移、分帧界面加载和新增网络命令。
浏览器时区适配已对照 Python `zoneinfo` 验证 15 个用例，覆盖 UTC、上海、加德满都、
纽约、悉尼的日历字段、夏令时、时间往返和认证时区偏移。

## Turtle_ShopUI 模型预览

Turtle 模式支持 `PlayerModel`/`DressUpModel:SetUnit(数字 displayId)`，通过已有的
生物显示模型渲染流程加载模型、皮肤和动画；`RefreshUnit()` 会保留该显示编号。
例如商城的 20418（幽灵虎）、20506、18778、11444 已验证对应模型与贴图可读。
此扩展仅在显式开启 Turtle 模式后启用，普通单位 API 的名称校验保持原有规则。
当前版本也支持 `SetPortraitToTexture(textureObject, texturePath)` 的纹理对象参数。

## 扩展人物皮肤与选角公会背心

Turtle 模式通过 `CharSections::load_turtle` 将带扩展标记的身体皮肤，追加到各自种族、
性别的标准肤色之后；标准肤色和原有 NPC 查询键仍然保留。例如亡灵女性的肤色 6
使用 `ScourgeBloodWidowSkin00_00.blp`，其面部行已采用颜色 6，不能按原始身体行的
颜色 0 覆盖普通肤色。身体合成使用完整扩展外观，再叠加实际装备。

网页版保留角色列表中的公会编号，在已经认证的世界连接上异步查询公会记录。
查询不阻塞角色列表或进入世界；选角模型在徽章记录到达后重新合成背心。
游戏内和选角使用同一份公会记录。

界面诊断还会回传角色与相机坐标、地面碰撞距离、建筑加载/可见状态和身体图集绑定，
用于区分坐标、资源和绘制问题；不采集账号口令或认证密钥。
