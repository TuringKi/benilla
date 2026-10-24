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

```bash
target/release/wenilla-host \
  --www web/dist \
  --data /path/to/client/Data \
  --upstream 192.168.1.79 \
  --auth-port 3725 \
  --world-port 8091
```

本次验证的混合服务端要求登录认证使用 **1.18.1 / build 7272**，世界认证使用
**build 5875**。`web/index.html` 已配置 `client_build: '7272'`、
`world_client_build: '5875'` 和 `turtle_compat: '1'`。连接其他服务端时按其要求调整；
不配置 build 覆盖时，协议库仍默认使用 5875。

浏览器连接地址的 `host` 参数是登录服地址，例如
`https://192.168.1.79:8443/?host=192.168.1.79%3A3725`。
世界端口由服务端角色列表中的地址决定，同时需要在 host 的允许端口中配置。

## 局域网 HTTPS

WebGPU 需要安全上下文。使用局域网 IP 访问时，在前面配置 HTTPS 反向代理，
并让访问电脑信任对应的 TLS 证书。后端默认仍监听 `127.0.0.1:8090`。
以下片段放在 nginx 的 `http` 内，替换证书路径和监听地址：

```nginx
map $http_upgrade $connection_upgrade {
    default upgrade;
    '' close;
}
server {
    listen 192.168.1.79:8443 ssl;
    server_name 192.168.1.79;
    ssl_certificate /path/to/lan-server.crt;
    ssl_certificate_key /path/to/lan-server.key;
    location / {
        proxy_pass http://127.0.0.1:8090;
        proxy_http_version 1.1;
        proxy_set_header Host $http_host;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
        proxy_buffering off;
    }
}
```

保留后端的 COOP/COEP 响应头。证书私钥、游戏资源、账号密码和运行日志仅保存在本机。

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
    proxy_pass http://127.0.0.1:8092;
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
