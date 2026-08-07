# AppDataHub

AI 软件多账号切换管理器，基于 Vue 3 + Tauri 2 + Rust 构建。

通过 JSON 插件化流程引擎，管理 AI 软件（如 Trae CN）的多账号配置，一键切换。

## 架构

- **前端**: Vue 3 (Composition API + `<script setup lang="ts">`) + TypeScript + Vite + Pinia + Vue Router + Element Plus
- **后端**: Rust + Tauri 2，JSON 插件化流程引擎
- **插件**: 每个软件一个 JSON 文件，定义数据目录、机器码、切换/清除流程

## 开发

```bash
winget install Rustlang.Rustup
pnpm install
pnpm tauri dev      # 开发
pnpm tauri build    # 生产构建
```

## 技术栈

Vue 3 / TypeScript / Tauri 2 / Rust / serde / MIT
