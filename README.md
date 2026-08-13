# Cast Model Merger GUI

一个基于 [echo000/ModelMerger](https://github.com/echo000/ModelMerger) 的 Windows 图形界面工具。可同时管理多个模型组，每组把 2 至 16 个 Cast 模型部件合并为一个 `.cast` 文件。

## 功能

- 可创建多个互相独立的合并组，每组均可展开或折叠。
- 每组提供 4 × 4 可视化槽位，清楚显示当前已选数量。
- 点击空槽位逐个选择部件，也支持将多个 `.cast` 文件拖入对应组。
- 各组会记住最近添加部件的文件夹，后续选择会从同一路径打开。
- 支持删除、替换部件，并可为各组手动指定根模型。
- 可单独启动、取消某一组，也可一键合并所有已就绪的组。
- 最多同时执行 2 组合并，其他组自动排队，避免大型模型同时占用过多内存。
- 默认沿用上游的根模型识别、骨骼连接和模型重定位逻辑。
- 可选择输出文件夹和输出文件名，覆盖已有文件前会确认。
- 后台合并、阶段进度、运行日志及取消操作，界面不会因处理大模型而冻结。
- 输出先写入临时文件并重新读取验证，成功后才生成最终文件。
- 可保存输出目录、根模型模式及窗口位置；不会保存已选择的模型路径。

设置文件保存在：

```text
%LocalAppData%\CastModelMerger\settings.json
```

## 使用

1. 使用“新建模型组”添加任务；不需要查看的组可以折叠。
2. 在目标组中点击“添加下一个”或任一空槽位，每次选择一个 `.cast` 部件。
3. 重复添加，直到该组选择 2 至 16 个部件；也可以直接拖入多个文件。
4. 保持“自动识别”根模型，或切换到“手动指定”并在部件槽点击“设为根”。
5. 选择该组的输出文件夹；文件名可留空，此时使用根模型名称。
6. 点击组内“开始合并”，或点击底部“合并所有已就绪组”。成功后可以直接打开输出文件所在位置。

## 构建和测试

需要 .NET 8 SDK 或能够构建 `net8.0` 项目的更新版 SDK：

```powershell
dotnet build .\src\ModelMerger\ModelMerger.sln -c Release
dotnet test .\tests\ModelMerger.Core.Tests\ModelMerger.Core.Tests.csproj -c Release
```

生成无需安装 .NET Runtime 的 `win-x64` 单文件版本：

```powershell
dotnet publish .\src\ModelMerger.Gui\ModelMerger.Gui.csproj `
  -c Release `
  -r win-x64 `
  --self-contained true `
  -o .\artifacts\publish\win-x64
```

## 工程结构

```text
src/ModelMerger.Core   合并、验证、设置存储
src/ModelMerger.Gui    WPF 图形界面
src/ModelMerger        上游控制台版本（保留作回归参考）
tests/                 核心接口与真实 Cast 合并测试
```

## 致谢与许可

原始 ModelMerger 由 Philip / Scobalula 开发，Cast 支持由 echo000 添加。本项目保留原作者署名并继续采用 [MIT License](LICENSE)。
