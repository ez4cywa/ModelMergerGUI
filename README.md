<div align="center">

# Cast Model Merger GUI

**拖入 CAST 部件，分组合并，在独立 3D 窗口中检查结果。**

面向 Windows 的 Rust 原生模型合并、预览与弹匣装填工具。

[![Release](https://img.shields.io/github/v/release/ez4cywa/ModelMergerGUI)](https://github.com/ez4cywa/ModelMergerGUI/releases/latest)
[![Windows x64](https://img.shields.io/badge/Windows-x64-0078D4)](https://github.com/ez4cywa/ModelMergerGUI/releases/latest)
[![Rust](https://img.shields.io/badge/Rust-native-dea584)](rust/Cargo.toml)
[![MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Rust CI](https://github.com/ez4cywa/ModelMergerGUI/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/ez4cywa/ModelMergerGUI/actions/workflows/rust-ci.yml)

简体中文 · [English](README.en.md)

[**下载免安装版**](https://github.com/ez4cywa/ModelMergerGUI/releases/latest/download/CastModelMerger-win-x64.zip) · [使用说明](docs/user-guide.zh-CN.md) · [更新记录](https://github.com/ez4cywa/ModelMergerGUI/releases) · [反馈问题](https://github.com/ez4cywa/ModelMergerGUI/issues/new/choose)

</div>

![Cast 模型合并器中文主界面](docs/images/rust-native/main-window-zh.png)

*截图来自 Rust 原生界面；部分截图早于最新版本，按钮与布局以当前发布包为准。*

## 可以做什么

| 场景 | 功能 |
| --- | --- |
| 把分散的部件合成模型 | 每组 2–15 个 CAST 部件，自动识别或手动指定根模型 |
| 分批处理多个模型 | 每批独立成组，最多同时执行 2 组合并，其余排队；支持取消 |
| 只查看模型、不合并 | 专用预览区支持一次拖入多个 CAST，分别打开独立窗口 |
| 检查形状、尺寸与拼接结果 | GPU 深度渲染、旋转、缩放、地面网格和模型信息 |
| 给弹匣补齐子弹模型 | 按子弹骨骼装填，可为备用弹匣复制位置与朝向布局 |
| 使用不同语言或主题 | 中、英、法、俄、西五语即时切换；醒目的亮暗按钮并记住选择 |

模型处理在本机完成。预览不修改源文件；合并输出先写入临时文件并重新读取验证。更新检查默认关闭，可在“帮助 → 关于”中手动检查或启用启动时检查。

## 下载与运行

1. 下载 [**CastModelMerger-win-x64.zip**](https://github.com/ez4cywa/ModelMergerGUI/releases/latest/download/CastModelMerger-win-x64.zip)。
2. 完整解压 ZIP，再运行 `CastModelMerger.exe`，不要直接在压缩包内启动。
3. 准备自己的 `.cast` 文件，按下方流程预览或合并。

支持 Windows 10/11 x64，预览需要支持 Direct3D 12 的显卡驱动。**无需安装 .NET 或 Rust**；发布包只有 Windows x64 原生免安装版，不是安装向导。

同一 [Release](https://github.com/ez4cywa/ModelMergerGUI/releases/latest) 提供 SHA-256 校验文件。在下载目录运行 `Get-FileHash .\CastModelMerger-win-x64.zip -Algorithm SHA256`，可与校验文件中的摘要比较。

## 快速上手

- **预览**：把一个或多个 CAST 拖到工作区顶部“模型预览区” → 在独立窗口中旋转、缩放或查看模型信息。
- **合并**：把 2–15 个部件拖入组 → 确认根模型和输出路径 → 开始合并 → 预览结果。
- **弹匣装填**：打开“弹匣装填” → 选择武器与单个 `tag_ammo` 骨骼的子弹模型 → 勾选目标 → 另存新文件。

### 拖到哪里，会发生什么

| 拖放区域 | 结果 |
| --- | --- |
| 顶部模型预览区 | 批量打开独立预览，不加入合并组 |
| 已有模型组或部件槽 | 向该组追加部件，最多 15 个 |
| 其他区域 | 每批单独成组，优先使用空闲空组，否则新建；无需先合并上一批 |

拖放目标区域会高亮提示。更多操作、快捷键及装填规则见[完整使用说明](docs/user-guide.zh-CN.md)。

<details>
<summary>查看预览与弹匣装填截图</summary>

| 浅色预览 | 深色预览 |
| --- | --- |
| ![浅色模型预览](docs/images/rust-native/model-preview-zh.png) | ![深色模型预览](docs/images/rust-native/model-preview-dark-zh.png) |

![弹匣装填（v2.2.1 界面）](docs/images/rust-native/ammo-fill-zh.png)

*弹匣截图为 v2.2.1；当前版本已合并重复控件，使用行内“复制来源”选择。*

</details>

## 支持范围与限制

- 当前输入与输出为 `.cast`，不是通用格式转换器；不提供游戏资源提取功能。
- 根识别、骨骼连接与重定位沿用上游逻辑，不保证任意来源的部件都能自动正确拼接。
- 预览使用中性灰几何显示；大型模型最多显示 250,000 个三角面，抽样时有提示，实际合并仍使用完整数据。
- 弹匣装填依赖受支持的骨骼命名和单位缩放刚体骨骼，最多 512 个槽位、500 万个新增顶点；必须另存新文件。
- 当前仅发布 Windows x64 版本，全部功能由 Rust 原生实现。不发布 macOS 或 Linux 版本。

## 常见问题与日志

**拖入文件却没有预览？** 只有顶部预览区用于拖入预览；其他区域用于导入部件。也可使用“文件 → 打开模型预览…”（`Ctrl+O`）。

**窗口打不开或预览崩溃？** 先确认已完整解压并更新显卡驱动，再提供复现步骤和诊断日志。日志通常位于：

```text
%LocalAppData%\CastModelMerger\logs\CastModelMerger.log
```

日志包含版本、显卡、模型路径和预览生命周期；Rust panic 记录堆栈，Windows 原生异常尽可能记录异常码与地址。并发日志与临时目录回退说明见[使用指南](docs/user-guide.zh-CN.md)。日志仅保存在本机，分享前可删除不希望公开的路径信息。

**设置保存在哪里？** `%LocalAppData%\CastModelMerger\settings.json`。亮暗选择自动保存，语言等设置可通过“设置 → 保存设置”保存；不会保存已选模型路径。

## 从源码运行与验证

开发需要 Windows x64、Rust 1.96 或兼容的更新稳定工具链，以及对应的 Windows 链接工具。以下命令从仓库根目录执行：

```powershell
git clone https://github.com/ez4cywa/ModelMergerGUI.git
cd ModelMergerGUI
# 阅读 MiSans 许可，接受后下载构建所需字体
.\scripts\Install-MiSans.ps1 -AcceptLicense
cargo run --manifest-path rust/Cargo.toml -p model-merger-gui --bin CastModelMerger
```

```powershell
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace
# 生成 Windows x64 ZIP 和 SHA-256 校验文件
.\scripts\Publish-RustNative.ps1
```

[Windows CI](https://github.com/ez4cywa/ModelMergerGUI/actions/workflows/rust-ci.yml) 检查格式、Clippy、测试、Release 构建与图标资源；[解码器模糊测试](https://github.com/ez4cywa/ModelMergerGUI/actions/workflows/fuzz.yml) 定期运行。字体授权、构建细节见[使用指南](docs/user-guide.zh-CN.md#构建和测试)，不以固定测试数量代替当前 CI 结果。

## 项目结构与文档

```text
rust/crates/cast-codec              CAST 编解码与资源预算
rust/crates/model-merger-engine     合并、装填、拼接、输出验证与预览抽样
rust/crates/model-merger-app-core   工作区、设置、多语言与任务调度
rust/crates/model-merger-gui        egui/wgpu 桌面界面
rust/fuzz                          解码器模糊测试
scripts/                           字体准备、Windows 构建与发布验证
docs/                              使用指南、设计记录与更新说明
tests/fixtures/rust-migration      Rust 合并测试的黄金语料
```

[中文指南](docs/user-guide.zh-CN.md) · [English guide](docs/user-guide.en.md) · [完整 Rust 迁移记录](docs/full-rust-migration.md) · [弹匣骨骼研究](docs/ammo-bone-research.md) · [版本说明](docs/release-notes)

## 参与与许可

欢迎提交 [Issue](https://github.com/ez4cywa/ModelMergerGUI/issues/new/choose) 或 Pull Request，贡献说明见 [CONTRIBUTING.md](CONTRIBUTING.md)。反馈时请提供版本、复现步骤、预期与实际结果，以及相关日志；模型样本优先使用可公开的最小复现文件。

原始 ModelMerger 由 Philip / Scobalula 开发，Cast 支持由 [echo000](https://github.com/echo000/ModelMerger) 添加。本项目保留原作者署名，源码采用 [MIT License](LICENSE)。嵌入的 MiSans 字体遵循独立许可，详见[第三方声明](THIRD-PARTY-NOTICES.md)。
