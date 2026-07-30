# Forge

轻量、本地优先的高性能 API 客户端。对标 Postman / Bruno，基于 **Tauri 2 + Rust + Svelte 5**。

规格：[`api-client-spec.md`](./api-client-spec.md)

## 功能概览

- HTTP：全部常用方法、Query/Headers、JSON/Form/Multipart/Raw/Binary body
- 环境变量 `{{var}}`，多环境切换
- 集合树：文件夹/请求、重命名、删除、拖拽排序；JSON/YAML 本地存储
- 双脚本引擎：Rhai（默认）+ JavaScript（Boa）；pre/post、断言、fs、tools
- Auth：Bearer / Basic / API Key；Cookie jar；代理 system/none/manual
- 导入 cURL / Postman v2.1；代码生成 cURL / fetch / Python
- CodeMirror 6、深浅色主题、全局搜索、快捷键、历史记录

## 开发

```bash
pnpm install
pnpm tauri:dev
```

```bash
# 测试
cd src-tauri && cargo test
pnpm check
pnpm build
```

数据目录：`~/Library/Application Support/Forge/`（macOS）

## 发布 / 打二进制包

这是 **Tauri 桌面应用**，可以编译成系统原生程序（不依赖 Node 即可运行）。

```bash
pnpm install
pnpm tauri:build
```

会先构建前端，再编译 Rust release，并打出安装包；macOS 上还会跑 `scripts/fix-macos-dmg.sh`：  
**丢弃 Tauri 自带的 DMG**，用 `.app` 重新打一个干净盘（只有 `Forge.app` + `Applications`，不含 `.VolumeIcon.icns`）。

### 产物路径（macOS）

| 产物 | 路径 |
|------|------|
| **`.app` 应用** | `src-tauri/target/release/bundle/macos/Forge.app` |
| **`.dmg` 安装包** | `src-tauri/target/release/bundle/dmg/` |
| **可执行文件** | `src-tauri/target/release/forge` |

本地使用：双击 `Forge.app` 即可。  
分发给他人：发送 `.dmg`（或整个 `.app`）。

DMG 里正常应只有：

1. **Forge.app** — 应用本体  
2. **Applications** — 应用程序文件夹快捷方式（拖进去完成安装）  

若安装窗口里仍看到 `.VolumeIcon.icns`，多半是旧 DMG 还挂着：在 Finder 侧栏把 **Forge** 推出，再打开  
`src-tauri/target/release/bundle/dmg/Forge_*.dmg`。

### 其他平台

在对应系统上执行同一条 `pnpm tauri:build`：

- **Windows**：`.msi` / `.exe`（NSIS 等）
- **Linux**：`.deb` / `.AppImage` 等

产物一般在 `src-tauri/target/release/bundle/` 下。

### 注意

1. **调试版**：`src-tauri/target/debug/forge` 也能跑，日常开发更推荐 `pnpm tauri:dev`；对外请用 `release` 包。
2. **macOS 分发**：未签名时，别人可能被 Gatekeeper 拦截，可在「系统设置 → 隐私与安全性」里选择「仍要打开」，或使用 Apple 开发者证书做 **codesign + notarize**（正式对外分发建议签名）。
3. **体积**：release 比 debug 更小更快；依赖（reqwest、Rhai、Boa 等）会让体积大于纯 CLI，但仍远小于 Electron / Postman。

## 项目结构

```
src/                 # Svelte 前端
src-tauri/src/
  http.rs            # reqwest 客户端 + auth/cookies/proxy
  env_interp.rs      # {{variable}}
  scripts/           # Rhai + Boa JS
  import.rs          # cURL / Postman
  codegen.rs
  storage.rs         # collections / envs / cookies / history
  models.rs
```
