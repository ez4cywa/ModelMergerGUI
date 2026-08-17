# Cast Model Merger GUI

一个基于 [echo000/ModelMerger](https://github.com/echo000/ModelMerger) 的 Windows 图形界面工具。可同时管理多个模型组，每组把 2 至 15 个 Cast 模型部件合并为一个 `.cast` 文件。

## 下载

从 v1.2.0 起，[Releases](https://github.com/ez4cywa/ModelMergerGUI/releases/latest) 只提供不包含运行环境的 Windows x64 轻量免安装版：

| 版本 | 下载文件 | 运行环境 |
| --- | --- | --- |
| 轻量免安装版 | `CastModelMerger-portable-win-x64.exe` | 不包含运行环境，需要预先安装 **.NET 8 Desktop Runtime x64** |

### 免安装版需要的环境

- [受 .NET 8 支持的 64 位 Windows](https://github.com/dotnet/core/blob/main/release-notes/8.0/supported-os.md)。普通个人电脑建议使用 Windows 11 x64；Windows 10 或 Windows Server 请以微软当前支持列表为准。
- [Microsoft .NET 8 Desktop Runtime（Windows x64）](https://dotnet.microsoft.com/download/dotnet/8.0)。请选择下载页面中的 **.NET Desktop Runtime → Windows → x64**；它已经包含基础 .NET Runtime，不需要再单独安装 SDK。
- 必须安装 x64 Desktop Runtime；仅安装 x86 版本、普通 `.NET Runtime` 或 `ASP.NET Core Runtime` 不能满足本程序的 WPF 桌面运行环境。

如果电脑缺少所需运行环境，双击程序时 .NET 启动器会显示缺少的框架、版本和架构，并提供下载入口。安装 .NET 8 Desktop Runtime x64 后，重新双击程序即可。也可以在命令提示符中运行 `dotnet --list-runtimes`，确认列表包含 `Microsoft.WindowsDesktop.App 8.`。

## 功能

- 可创建多个互相独立的合并组，每组均可展开或折叠。
- 每组提供 5 × 3 可视化槽位，清楚显示当前已选数量。
- 点击空槽位逐个选择部件，也支持将多个 `.cast` 文件拖入对应组。
- 各组会记住最近添加部件的文件夹，后续选择会从同一路径打开。
- 支持删除、替换部件，并可为各组手动指定根模型。
- 可从任一已选槽位打开交互式 3D 部件预览，也可在合并完成后预览最终拼接模型。
- 预览窗口支持鼠标拖动旋转、滚轮缩放、键盘操作和一键重置视角；大型模型会自动抽样显示，不修改源文件。
- 可单独启动、取消某一组，也可一键合并所有已就绪的组。
- 最多同时执行 2 组合并，其他组自动排队；排队或运行中的任务均可取消。
- 同一输出路径不会被两个模型组同时写入，冲突会在网格合并前停止。
- 默认沿用上游的根模型识别、骨骼连接和模型重定位逻辑。
- 可选择输出文件夹和输出文件名，覆盖已有文件前会确认。
- 后台合并、阶段进度、运行日志及取消操作，界面不会因处理大模型而冻结。
- 输出先写入临时文件并重新读取验证，成功后才生成最终文件。
- 中文、English、Français、Русский、Español 可在同一程序内即时切换，已有状态、日志和对话框会同步更新。
- 中文界面使用随程序嵌入的 MiSans，其他四种语言使用 Segoe UI。
- 可保存界面语言、输出目录、根模型模式及窗口位置；不会保存已选择的模型路径。

原有命令行程序现在与 GUI 共用同一个合并引擎。GUI 保持只接受 2–15 个 `.cast` 部件；命令行拖放入口继续兼容一个或多个 `.cast` / `.semodel` 输入，并统一输出经过验证的 `.cast` 文件。

设置文件保存在：

```text
%LocalAppData%\CastModelMerger\settings.json
```

## 使用

1. 使用“新建模型组”添加任务；不需要查看的组可以折叠。
2. 在目标组中点击“添加下一个”或任一空槽位，每次选择一个 `.cast` 部件。
3. 重复添加，直到该组选择 2 至 15 个部件；也可以直接拖入多个文件。
4. 点击已添加部件下方的“预览”，可在合并前检查单个部件；保持“自动识别”根模型，或切换到“手动指定”并在部件槽点击“设为根”。
5. 选择该组的输出文件夹；文件名可留空，此时使用根模型名称。
6. 点击组内“开始合并”，或点击底部“合并所有已就绪组”。成功后可预览合并模型，或直接打开输出文件所在位置。

界面右上角可随时选择中文、English、Français、Русский 或 Español；点击“保存设置”后，下次启动会沿用该语言。首次启动会跟随 Windows 的上述五种界面语言，其他系统语言默认显示中文。

中文界面嵌入并使用小米 MiSans 字体。MiSans 不属于本项目的 MIT 授权范围，使用和分发遵循小米的 MiSans 字体许可；详情见 [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) 和官方许可页面。

## 如何预览模型

### 预览单个部件

1. 在模型组中点击“添加下一个”或任一空槽位，选择一个 `.cast` 文件。
2. 文件添加成功后，原来的空槽位会显示文件名和“预览”按钮。
3. 点击该槽位中的“预览”，即可打开独立的 3D 预览窗口。
4. 可以继续预览其他部件；每个预览窗口相互独立，允许同时打开多个窗口进行对比。

### 预览合并后的模型

1. 为模型组添加 2 至 15 个有效部件，并确认自动生成的输出文件夹；如有需要，也可以另选目录。
2. 点击“开始合并”，等待本组状态显示合并成功。
3. 在右侧“本组状态”区域点击“预览合并模型”。该按钮只有在合并成功且输出文件仍然存在时才会启用。
4. 如果移动或删除了输出文件，请重新合并后再预览。

### 预览窗口操作

| 操作 | 鼠标或键盘 |
| --- | --- |
| 自由旋转 | 按住鼠标左键拖动 |
| 分步旋转 | 点击“向左旋转”或“向右旋转”，也可使用方向键 |
| 放大或缩小 | 滚动鼠标滚轮、点击“放大/缩小”，或按 `+` / `-` |
| 恢复初始视角 | 点击“重置视角”或按 `R` |
| 关闭预览 | 点击“关闭”或按 `Esc` |

预览只读取模型，不会修改部件、合并计划或输出文件。为保证大型模型操作流畅，预览画面最多显示 75,000 个三角面；发生抽样时窗口会显示黄色提示，但实际合并仍使用完整模型数据。

## 构建和测试

需要 .NET 8 SDK 或能够构建 `net8.0` 项目的更新版 SDK：

MiSans 的许可允许把字体嵌入应用，但不允许把字体文件作为独立资源再次分发，因此 Git 仓库不直接提交 `.ttf`。首次从源码构建前，请阅读[官方 MiSans 许可](https://hyperos.mi.com/font/en/download/)，接受后运行：

```powershell
.\scripts\Install-MiSans.ps1 -AcceptLicense
```

脚本从小米官网下载经校验的字体包，只提取程序使用的 Regular、Semibold 和 Bold 三个字重；下载的本地字体文件会被 Git 忽略。官方 GitHub Release 中的可执行文件已经嵌入字体，普通用户无需运行该脚本。

```powershell
dotnet build .\src\ModelMerger\ModelMerger.sln -c Release
dotnet test .\tests\ModelMerger.Core.Tests\ModelMerger.Core.Tests.csproj -c Release
dotnet test .\tests\ModelMerger.Gui.Tests\ModelMerger.Gui.Tests.csproj -c Release
```

生成项目唯一发布类型——不包含运行环境、需要 .NET 8 Desktop Runtime x64 的轻量单文件版本：

```powershell
dotnet publish .\src\ModelMerger.Gui\ModelMerger.Gui.csproj `
  -c Release `
  -r win-x64 `
  -p:SelfContained=false `
  -o .\artifacts\publish\portable-win-x64

Rename-Item `
  .\artifacts\publish\portable-win-x64\CastModelMerger.exe `
  CastModelMerger-portable-win-x64.exe
```

## 工程结构

```text
src/ModelMerger.Core   合并计划、任务调度、格式适配、合并与设置存储
src/ModelMerger.Gui    WPF 图形界面、五语语言目录与嵌入字体资源
src/ModelMerger        使用共享 Core 的 Cast / SEModel 命令行入口
tests/                 计划、调度、真实 Cast/SEModel、语言及 WPF 视觉冒烟测试
```

## 致谢与许可

原始 ModelMerger 由 Philip / Scobalula 开发，Cast 支持由 echo000 添加。本项目保留原作者署名并继续采用 [MIT License](LICENSE)。
