# Anya 浏览器选区桥接

可选的 Chrome / Edge 扩展，用于取得网页选中文字和来源 URL。不安装扩展也可以使用 Anya 的原生选区采集。

## Windows 安装

使用已编译的桥接 ZIP 时，解压到固定目录，运行其中的 `install.ps1`，再执行下面第 3–4 步即可，压缩包已包含桥接程序。桌面应用需要使用支持选区桥接的新构建。

1. 用当前源码重新编译桌面应用，再编译小型原生桥接程序：

   ```powershell
   cargo build --manifest-path src-tauri/Cargo.toml --bin anya-browser-bridge
   ```

2. 为当前 Windows 用户注册桥接程序：

   ```powershell
   powershell -ExecutionPolicy Bypass -File browser-extension/install.ps1
   ```

3. 打开 `chrome://extensions` 或 `edge://extensions`，启用开发者模式，点击“加载已解压的扩展程序”，选择本 `browser-extension` 目录。刷新已经打开的网页。
4. 重启 Anya，在网页中选中文字，再按设置的全局快捷键。输入区域会出现行内引用标签。

发布版桥接程序可用 `-HostExecutable C:\path\anya-browser-bridge.exe` 指定。注册后不要移动该程序。扩展内置固定公钥，因此扩展 ID 保持稳定。

## 行为与边界

- 只向本地应用传递选中文字，不连接远程服务、不采集浏览历史；不读取密码输入框的选区。
- 选区设有长度上限，避免整篇长网页卡住输入区域。
- 扩展访问范围为 HTTP(S) 网页，安装时浏览器会显示权限。浏览器内部页面和内置 PDF 阅读器走原生采集回退。
- 选区快照必须匹配当前浏览器窗口、进程和标题。取消选择或切换标签页会清空缓存，快照在 60 秒后失效。
- Native Messaging 核对注册的扩展来源；本地桥接使用随机端口和令牌，拒绝超长消息，不在日志中记录选中文字。
- 支持普通网页、编辑框和允许内容脚本访问的 iframe。受限 iframe 或画布编辑器可能需要原生复制回退。
- 原生采集先于弹窗聚焦，并在工作线程运行。优先 UI Automation / MSAA，回退到 WM_COPY / Ctrl+Insert；任何应用中都不自动发送 Ctrl+C，避免中断内置终端。采集失败时可手动复制并粘贴。

在浏览器中移除扩展后，可注销原生桥接：

```powershell
powershell -ExecutionPolicy Bypass -File browser-extension/install.ps1 -Uninstall
```

[English](README.md)

维护者编译桥接程序后，可运行 `powershell -ExecutionPolicy Bypass -File browser-extension/package.ps1` 生成可选 ZIP。
