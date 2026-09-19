# 参与贡献 / Contributing

欢迎提交可复现的问题、文档修正和范围明确的 Pull Request。

## 反馈问题

请通过 [Issues](https://github.com/ez4cywa/ModelMergerGUI/issues) 提供：

- 软件版本与 Windows 版本，预览问题另附显卡型号。
- 最小复现步骤、预期结果与实际结果。
- 相关日志；位置见[使用指南](docs/user-guide.zh-CN.md)。
- 如需模型样本，提供可以公开分享的最小 CAST 文件，避免上传无关资源。

## 提交修改

从当前 `main` 创建工作分支，保持每个 PR 只解决一个明确问题。说明修改目的、影响和验证结果。文档变更检查链接即可；修改代码时运行：

```powershell
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace
```

首次构建前按 [README](README.md#从源码运行与验证) 准备字体。核心逻辑变更应补充相关回归测试；涉及界面时附截图，检查五种语言与亮暗主题。不要提交下载的字体、构建产物、个人日志或无关模型文件。

## English

For bug reports, include the app and Windows versions, reproduction steps, expected and actual results, and relevant logs. For preview issues, include the GPU model. Share only a minimal CAST sample you can publish.

Branch from current `main`, keep each pull request focused, and explain its purpose, impact and verification. Check links for documentation changes; run the commands above for code changes. Follow the [README](README.en.md#run-from-source-and-verify) for font setup. Add regression coverage for core logic changes and screenshots for UI changes, checking all five languages and both themes. Do not commit downloaded fonts, build output, personal logs or unrelated model assets.
