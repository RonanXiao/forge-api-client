# Forge

轻量、本地优先的高性能 API 客户端。对标 Postman / Bruno，基于 **Tauri 2 + Rust + Svelte 5**。

规格：[`api-client-spec.md`](./api-client-spec.md)

当前版本：`v0.0.1-beta`（GitHub Releases）

## 许可证 / License

**个人 / 非商业使用免费**；**商业使用需付费授权**。

| 用途 | 协议 | 说明 |
|------|------|------|
| 个人、学习、爱好、非商业组织等 | [PolyForm Noncommercial 1.0.0](./LICENSE) | 可使用、修改；分发需遵守该协议 |
| 公司业务、商业产品、有偿服务等 | 单独 **Commercial License** | 见 [COMMERCIAL.md](./COMMERCIAL.md) |

商用授权联系：

- 邮箱：[wrdfeng@gmail.com](mailto:wrdfeng@gmail.com)
- 私信：联系作者（GitHub / 社交账号私信均可）

> 本仓库为 **源码可见（source-available）** 双授权模式，**不是** MIT/Apache 等允许免费商用的宽松开源协议。完整条款以 `LICENSE` 与商用合同为准。

## 功能概览

- HTTP：全部常用方法、Query/Headers、Body（none / form-data / x-www-form-urlencoded / raw / binary，对齐 Postman）
- 环境变量 `{{var}}`，多环境切换
- 集合树：文件夹/请求、重命名、删除、拖拽排序；JSON/YAML 本地存储
- 双脚本引擎：Rhai（默认）+ JavaScript（Boa）；pre/post、断言、fs、tools
- Auth：Bearer / Basic / API Key；Cookie jar；代理 system/none/manual
- 导入 cURL / Postman v2.1；URL 栏粘贴 cURL 自动解析
- 代码生成 cURL / fetch / Python
- Response：Pretty JSON/XML 折叠高亮、**Verbose**（curl `-v` 风格调试）
- CodeMirror 6、深浅色主题、全局搜索、快捷键、历史记录

## 版本号规范

对外发布（Git tag / GitHub Release）统一使用：

```text
vMAJOR.MINOR.PATCH-beta
vMAJOR.MINOR.PATCH-release
```

| 部分 | 说明 |
|------|------|
| `MAJOR.MINOR.PATCH` | 三级数字版本（SemVer 数字段） |
| `-beta` | 预览 / 测试版，可包含未完全稳定的功能 |
| `-release` | 正式版，建议用于日常使用与对外分发 |

示例：

- `v0.0.1-beta` — 首个公开测试版  
- `v0.0.1-release` — 同数字段的正式版  
- `v0.1.0-beta` — 下一功能周期的测试版  

### 仓库内版本字段

发版前同步（去掉前缀 `v`，保留 `-beta` / `-release` 后缀）：

| 文件 | 字段 |
|------|------|
| `package.json` | `"version"` |
| `src-tauri/Cargo.toml` | `version` |
| `src-tauri/tauri.conf.json` | `"version"` |

### 发版步骤（GitHub）

```bash
# 1. 改上述三处 version，更新本 README「当前版本」
# 2. 提交并 push 到 main
# 3. 打安装包
pnpm install
pnpm tauri:build

# 4. 打 tag 并推送（示例：beta）
git tag -a v0.0.1-beta -m "v0.0.1-beta"
git push origin v0.0.1-beta

# 5. 创建 Release 并挂上 DMG
gh release create v0.0.1-beta \
  --title "v0.0.1-beta" \
  --notes "Forge v0.0.1-beta" \
  --prerelease \
  src-tauri/target/release/bundle/dmg/Forge_*.dmg
```

- **beta**：`gh release create … --prerelease`  
- **release**：去掉 `--prerelease`，tag 用 `vX.Y.Z-release`

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

macOS 上 `pnpm tauri:build` 会：

1. 只打 **`.app`**（不让 Tauri 生成/自动打开带 `.VolumeIcon.icns` 的 DMG）  
2. 用 `scripts/fix-macos-dmg.sh` 从 `.app` 打干净 DMG（`Forge.app` + `Applications`）  
3. **打开干净的 DMG**（不会再弹旧版安装窗）

### 产物路径（macOS）

| 产物 | 路径 |
|------|------|
| **`.app` 应用** | `src-tauri/target/release/bundle/macos/Forge.app` |
| **`.dmg` 安装包** | `src-tauri/target/release/bundle/dmg/` |
| **可执行文件** | `src-tauri/target/release/forge` |

本地使用：双击 `Forge.app` 即可。  
分发给他人：发送 `.dmg`（或整个 `.app`），或从 [GitHub Releases](https://github.com/RonanXiao/forge-api-client/releases) 下载。

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

1. **调试版**：`src-tauri/target/debug/forge` 也能跑，日常开发更推荐 `pnpm tauri:dev`；对外请用 **release 构建** 的安装包（与版本后缀 `-release` 不是同一概念）。
2. **macOS 分发**：未签名时，别人可能被 Gatekeeper 拦截，可在「系统设置 → 隐私与安全性」里选择「仍要打开」，或使用 Apple 开发者证书做 **codesign + notarize**（正式对外分发建议签名）。
3. **体积**：release 构建比 debug 更小更快；依赖（reqwest、Rhai、Boa 等）会让体积大于纯 CLI，但仍远小于 Electron / Postman。

## 项目结构

```
src/                 # Svelte 前端
src-tauri/src/
  http.rs            # reqwest 客户端 + auth/cookies/proxy + verbose
  env_interp.rs      # {{variable}}
  scripts/           # Rhai + Boa JS
  import.rs          # cURL / Postman
  codegen.rs
  storage.rs         # collections / envs / cookies / history
  models.rs
```
