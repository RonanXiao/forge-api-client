# API Client — Tauri + Svelte 项目规格

> 一个轻量、本地优先、支持强脚本能力的 API 客户端  
> 对标 Bruno / Postman，但更轻、更快，脚本能力更灵活

---

## 一、项目目标

- 本地优先，无账号、无云同步
- 集合以纯文本文件存储（支持 Git）
- 高性能（Tauri + Rust）
- **强脚本能力**：支持 **Rhai** 和 **JavaScript** 两种脚本引擎，用户可选择
- 支持请求前后读写本地文件系统
- 界面现代、简洁（Svelte + Tailwind）

---

## 二、技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2 |
| 前端 | Svelte 5 + TypeScript + Tailwind CSS |
| 代码编辑器 | CodeMirror 6 |
| HTTP 客户端 | Rust `reqwest` + `tokio` |
| 脚本引擎 | **Rhai**（默认） + **JavaScript**（Boa） |
| 存储 | 本地 JSON / YAML 文件 |
| 状态管理 | Svelte 5 runes |

---

## 三、功能列表

### 1. 核心功能（MVP）

- [x] 发送 HTTP 请求（GET / POST / PUT / PATCH / DELETE / HEAD / OPTIONS）
- [x] 请求编辑
  - URL
  - Method
  - Query Params
  - Headers
  - Body（JSON / Form-urlencoded / Multipart / Raw / Binary）
- [x] 响应查看
  - 状态码、耗时、大小
  - Headers
  - Body（Pretty / Raw 切换）
  - 响应内容搜索
- [x] 环境变量系统
  - 支持 `{{variable}}` 语法
  - 多环境切换（dev / test / prod）
  - 环境变量增删改
- [x] 集合管理
  - 树形结构（文件夹 + 请求）
  - 新建 / 重命名 / 删除 / 拖拽排序
- [x] 本地文件存储
  - 集合以 JSON 或 YAML 格式保存在本地文件夹
  - 支持 Git 版本控制
- [x] 请求历史记录
- [x] 导入
  - cURL
  - Postman Collection v2.1
  - Bruno（可选，未实现）

### 2. 脚本系统（核心竞争力）

#### 2.1 脚本引擎（双引擎可选）

- [x] **支持两种脚本引擎**：
  - **Rhai**（推荐默认）
  - **JavaScript**（Boa pure Rust engine）
- [x] 脚本引擎可在全局设置与单请求级别覆盖

#### 2.2 脚本类型

- [x] **Pre-request 脚本**
- [x] **Post-response 脚本**
- [x] 脚本作用域：集合 → 文件夹 → 请求

#### 2.3 脚本能力

- [x] 文件系统访问（可配置权限）
- [x] 控制台输出
- [x] 变量操作
- [x] 常用工具函数（UUID、时间、Base64、MD5/SHA、JSON）
- [x] 简单断言（状态码 / 字段 / 响应时间）

#### 2.4 脚本安全

- [x] 可配置脚本权限（fs / network reserved / timeout）
- [x] 脚本错误友好提示

### 3. 进阶功能

- [x] 认证（Auth）：Bearer / Basic / API Key
- [x] Cookie 管理
- [x] 代理设置：system / none / manual HTTP/HTTPS/SOCKS
- [x] 请求配置：超时、重定向
- [x] 代码生成：cURL / fetch / Python requests
- [x] 快捷键支持（Send ⌘/Ctrl+Enter，Search ⌘K，Env ⌘L）
- [x] 深色 / 浅色主题
- [x] 全局搜索（请求名、URL）

### 4. 后期扩展（可选）

- [ ] GraphQL 支持
- [ ] WebSocket 基础支持
- [ ] 批量运行集合 / 文件夹
- [ ] 环境变量加密存储
- [ ] 插件系统
- [ ] 请求时间线（DNS / TCP / TTFB 等）
- [ ] 简单 Mock Server

---

**文档版本**：v1.2  
**最后更新**：阶段 1–4 非可选功能已实现并通过自动化测试
