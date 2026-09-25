# BO / Krig 6 部件拼接检查

检查日期：2026-09-10；软件提交：`3e0d8f0ec2996244bb2bf8be5cd208ac7e3bb44b`。

## 结论

这五个 CAST 文件不能依靠现有“同名部件根骨骼对齐”算法完成可靠定位。仅增加模式名称或更换主模型选择算法不能解决挂点缺失。尚未修改生产拼接逻辑或增加 BO 模式；需要补充部件的挂点、平移和旋转配置，或者明确采用人工配置/对齐的模式。

## 输入和证据

输入目录：`D:/_tiqu/Saluki/exported_files/bocw/models/wpn_t9_ar_krig6_dragon_view`。该目录包含用户指定的五个 CAST 文件和 `_images`，未发现配件布局配置。源文件未修改。

主模型：`wpn_t9_ar_krig6_dragon_view_LOD0.cast`，20 个网格、34,520 个顶点，根骨骼 `tag_weapon`。

| 部件文件前缀 | 部件根骨骼 | 枪身包含同名骨骼 | 网格 / 顶点 |
|---|---|---|---|
| `attach_t9_barrel_long_01_pro` | `tag_barrel_long_attach` | 否 | 9 / 13,921 |
| `attach_t9_mixclip_01` | `tag_clip` | 否 | 3 / 6,701 |
| `attach_t9_mixhandle_01_pro` | `tag_mixhandle_attach` | 否 | 1 / 4,108 |
| `attach_t9_mixstock_01_pro` | `tag_mixstock_attach` | 否 | 6 / 5,177 |

四个部件的根骨骼位置均为 `(0,0,0)`，旋转均为单位四元数。枪身含 `tag_barrel`、`tag_stock` 等其他骨骼，但不能据名称相近推定它们就是部件的目标挂点。

枪管部件和枪身均含 `tag_barrel_tip`，位置分别为 `(52.583916,0,0)` 和 `(62.146545,0,8.733105)`。这是枪口端标记，并非部件根骨骼；不同长度的枪管不能未经验证就通过枪口端位置对齐。

## 实际运行现有引擎

用临时 Rust 示例调用生产 `prepare(...).execute(...)`，分别测试：

1. 按上述目录文件名排序输入，`RootSelection::Automatic`：选中枪管作为主模型，产生 4 条 `NoAttachmentBone` 警告。
2. 同样输入，`RootSelection::Manual` 指定枪身：主模型选择正确，四个部件仍分别产生 `NoAttachmentBone` 警告。

两次均输出 39 个网格、59 根骨骼；文件可写出不代表位置正确。现有 `domain::merge_model` 在找不到同名根骨骼时添加独立根骨骼，这些部件会保留原坐标，而不是自动找到枪身上的安装位置。临时示例和两份试拼输出已清理。

## BO 模式需要明确的契约

### 加入手臂文件后的对比

补充文件：`D:/_tiqu/Saluki/exported_files/bocw/models/c_t9_uk_pl_mi6_derby_viewarms/c_t9_uk_pl_mi6_derby_viewarms_LOD0.cast`。

实测该文件有 166 根骨骼、9 个网格、17,660 个顶点。根骨骼为 `tag_view`；与枪身及四个配件分别进行精确骨骼名交集比较，五次交集均为空。

| 需要的定位骨骼 | 手臂文件中是否存在 |
|---|---|
| `tag_weapon`（枪身根） | 否 |
| `tag_barrel_long_attach` | 否 |
| `tag_clip` | 否 |
| `tag_mixhandle_attach` | 否 |
| `tag_mixstock_attach` | 否 |

手臂含 `tag_weapon_left` 和 `tag_weapon_right`，二者均以 `tag_torso` 为直接父骨骼，不是 `tag_weapon` 的同名匹配。其文件内世界空间数据为：

- `tag_weapon_left`：位置 `(35.328674,45.91386,-44.02424)`，旋转 `(0,0,0,1)`。
- `tag_weapon_right`：位置 `(35.239655,-45.66975,-44.124893)`，旋转 `(-0.3045908,0.26373592,-0.0969258,0.91009516)`。

因此手臂文件没有补齐四个配件的挂点，也没有建立现有算法可以直接识别的枪身连接。可以设计显式“枪身根 → 左/右武器挂点”映射，但具体选择、附加变换和持枪姿态仍需配置或参考数据验证，不能简单重命名骨骼，也不能把静态文件中的挂点坐标当作已经验证的 Krig 6 持枪姿态。

### 加入对应动画后的对比

动画文件：`D:/_tiqu/Saluki/exported_files/bocw/animations/vm_ar_accurate_t9_inspect_dragon.cast`，211,092 字节。

使用仓库 `cast-codec` 完整解码：30 FPS，非循环；包含 292 条曲线（193 条 `absolute`、99 条 `relative`），对应 193 个不同的目标骨骼名，另有 3 条通知轨道。`end` 通知位于第 358 帧。文件中没有 `skel` / `bone` 节点，也没有曲线模式覆盖节点，因而不提供额外的父子层级或静止骨架。

| 模型 | 有动画轨道的骨骼 / 总骨骼 |
|---|---|
| 手臂 | 153 / 166 |
| 枪身 | 26 / 48 |
| 弹匣 | 8 / 8 |
| 枪管 | 0 / 2 |
| 握把 | 0 / 1 |
| 枪托 | 0 / 1 |

覆盖数仅表示有至少一个同名属性通道，不表示有完整位置和旋转，也不意味着未覆盖骨骼无效；无轨道的骨骼可保留静止局部变换并随父骨骼运动。

关键通道：

- `tag_weapon`：只有第 0 帧的单位四元数绝对旋转通道，没有平移轨道。
- `tag_clip`：第 0 帧单位四元数绝对旋转；相对平移为 `(0.0062944316,-0.0000006055832,-0.6644453)`，位置通道仅一帧。
- `tag_weapon_right`：有绝对旋转和相对平移轨道。第 0 帧相对位移为 `(-8.695602,36.934284,28.295067)`，绝对旋转为单位四元数。
- `tag_barrel_long_attach`、`tag_mixhandle_attach`、`tag_mixstock_attach`：动画中没有对应曲线。

根据 [CAST 官方 Curve 规范](https://github.com/dtzxporter/cast#curve)，曲线在节点空间中求值，`relative` 加在该属性的静止值上，`absolute` 指定该帧的属性值，不等同于世界空间坐标。因此 `tag_clip` 的位移是动画相对量，不能据此确定它相对枪身的基础安装位置；`tag_weapon_right` 的轨道同样不能单独声明枪身应成为它的子节点。

动画还有 6 个目标名不在当前六份模型骨架中：`j_index_tendon_le_end`、`j_mid_tendon_le_end`、`j_pinky_tendon_le_end`、`j_ring_tendon_le_end`、`j_thumb_tendon_le_end`、`tag_fast_mag_attach`。这是具体的名称覆盖差异，不能仅据此断言整份动画不匹配。

更新结论：动画补充了手臂、枪身和弹匣的动作数据，但没有补齐装配所需的静止挂接关系，尤其是枪管、握把、枪托的位置。可靠实现仍需静止装配配置或已正确组装的参考文件；不能把检视动画第一帧的相对变化当作装配偏移。

### 加入基础枪身和原装弹匣

补充文件：

- `D:/_tiqu/Saluki/exported_files/bocw/models/wpn_t9_ar_krig6_view/wpn_t9_ar_krig6_view_LOD0.cast`
- `D:/_tiqu/Saluki/exported_files/bocw/models/wpn_t9_ar_krig6_view/wpn_t9_ar_krig6_mag_view_LOD0.cast`

基础枪身有 43 根骨骼、13 个网格、28,575 个顶点，与龙版枪身共有 24 个骨骼名。它同样不含 `tag_clip`、`tag_barrel_long_attach`、`tag_mixhandle_attach`、`tag_mixstock_attach`。

原装第一人称弹匣有 7 根骨骼、2 个网格、2,530 个顶点。其 7 个骨骼名在改装弹匣中全部存在，按骨骼名解析的父子关系一致，局部和世界位置分量差异均小于 `0.000001`，旋转和缩放一致；改装弹匣额外有 `tag_tac_mag_attach`。

这证明两种弹匣的共同绑定骨架和局部坐标约定一致，可以共用经验证的弹匣安装变换；但两份弹匣的 `tag_clip` 都是原点处的独立根，基础枪身中也没有它，所以尚未提供安装变换本身。两份模型节点未提供额外位置、旋转或实例变换。

### 加入世界枪身和世界弹匣

补充文件：

- `D:/_tiqu/Saluki/exported_files/bocw/models/wpn_t9_ar_krig6_world/wpn_t9_ar_krig6_world_LOD0.cast`
- `D:/_tiqu/Saluki/exported_files/bocw/models/wpn_t9_ar_krig6_mag_world/wpn_t9_ar_krig6_mag_world_LOD0.cast`

| 数据 | 基础第一人称枪身 | 基础世界枪身 | 第一人称弹匣 | 世界弹匣 |
|---|---:|---:|---:|---:|
| 骨骼 | 43 | 43 | 7 | 1 |
| 网格 | 13 | 13 | 2 | 2 |
| 顶点 | 28,575 | 28,575 | 2,530 | 1,906 |
| 三角面 | 34,099 | 34,099 | 3,396 | 2,500 |

逐网格直接比较 CAST 数组，基础第一人称和世界枪身的 `vp`（顶点位置）、`vn`（法线）与 `f`（三角面索引）全部精确相等，并非仅包围盒相近。

按骨骼名比较变换属性，排除因顺序不同产生的父索引数字差异后，仅三个骨骼的变换不同：`tag_ik_loc_le`、`tag_ik_loc_le_foregrip`、`tag_ik_loc_ri`。世界枪身同样不包含四个配件根骨骼，不存在额外的模型/实例位置变换。

世界弹匣只有 `tag_clip` 一根骨骼，仍位于 `(0,0,0)`，没有相对枪身的父骨骼。它与第一人称弹匣的包围盒 X/Y 范围完全一致，Z 顶部不同：第一人称 `5.849637`、世界 `5.733945`。它的顶点与面数更少，但没有因此变成已装配的弹匣模型。

综合结论：这些补充文件确认了基础/龙版武器及原装/改装弹匣之间的对应关系，但仍未提供缺失的静止装配挂点。不得将原装与改装弹匣同时并入结果来充当定位步骤；它们是候选替换件，挂接布局仍需明确来源。

### 加入普通换弹和空仓换弹动画

文件位于 `D:/_tiqu/Saluki/exported_files/bocw/animations/`：`vm_ar_accurate_t9_reload.cast`、`vm_ar_accurate_t9_reload_empty.cast`。完整解码并与检视动画比较，不改写输入文件。

| 项目 | 普通换弹 | 空仓换弹 |
|---|---:|---:|
| FPS | 60 | 60 |
| 曲线 / 目标骨骼名 | 302 / 203 | 320 / 203 |
| absolute / relative 曲线 | 203 / 99 | 203 / 117 |
| `clip_out` 事件帧 | 32 | 31 |
| `clip_in` 事件帧 | 93 | 91 |
| `end` 事件帧 | 145 | 205 |

两份均不循环，没有嵌入骨架或曲线模式覆盖。`tag_clip` 都有 `rq` 绝对旋转及 `tx/ty/tz` 相对平移；`tag_weapon` 仍只有第 0 帧单位四元数旋转。三份动画均没有 `tag_barrel_long_attach`、`tag_mixhandle_attach`、`tag_mixstock_attach` 曲线。

弹匣首末关键帧的平移值在各自换弹动画内部精确相等：

- 普通换弹：`(0.006361046,-0.000029219389,-0.66443133)`。
- 空仓换弹：`(0.006537876,-0.00038325845,-0.6644459)`。
- 对照检视动画的固定值：`(0.0062944316,-0.0000006055832,-0.6644453)`。

普通换弹首末旋转均为 `(0,0,0,1)`；空仓换弹末尾为 `(0,0,0,-1)`，与开头的 `(0,0,0,1)` 表示同一旋转，不能误判为旋转未归位。

新增证据确认两种换弹动作的弹匣都回到各自起始局部状态，并且与检视动作的起始相对位移很接近。但仍只能得到 `局部动画位置(t) = 静止位置 + 相对位移(t)`；多份相对轨道不能独立确定缺失的静止挂接位置及父节点。因此可以验证弹匣相对运动与归位，却不能仅靠这些动画自动推导四个配件的装配布局。

### 加入 ADS idle 和 active idle

补充动画：`vm_ar_accurate_t9_ads_idle.cast`、`vm_ar_accurate_t9_active_idle.cast`，均位于前述 animations 目录。

| 项目 | ADS idle | active idle |
|---|---:|---:|
| FPS | 30 | 30 |
| 曲线 / 目标骨骼名 | 286 / 193 | 206 / 197 |
| absolute / relative 曲线 | 193 / 93 | 197 / 9 |
| `loop_end` 通知帧 | 1 | 758 |

两份文件的 `lo` 属性实际为 0，不能因为名称含 idle 或存在 `loop_end` 通知就把文件循环标志解释成开启。两份均没有嵌入骨架。

- ADS idle：`tag_clip` 有第 0 帧单位四元数绝对旋转，以及固定的相对平移 `(0.006295643,0,-0.66444343)`，与检视、换弹的起始值接近。
- active idle：`tag_clip` 只有第 0 帧单位四元数旋转，没有平移通道。无平移通道不代表安装位置为零。
- 两份 `tag_weapon` 都只有单位四元数旋转，没有平移。三类改装根 `tag_barrel_long_attach`、`tag_mixhandle_attach`、`tag_mixstock_attach` 仍没有轨道。
- ADS idle 的 `tag_weapon_right` 相对平移为 `(-8.683607,36.93785,28.310265)`；active idle 第一帧仅为 `(-0.0055109016,0.007233313,-0.005264713)`，后续有变化。不能把这两种不同用途的相对轨道不加区分地当作同一个静止安装矩阵。

截至这五份动画的综合结论：ADS idle 提供了一个接近换弹归位状态的相对姿态对照，active idle 提供另一组动态变化，但没有任何一份补齐配件静止挂接关系。继续加入相对动画不能唯一求出缺失的静止位置；仍需挂接配置或正确组装参考。

### 实现前需确定的数据

- 以枪身为主模型。
- 每个部件提供目标父骨骼，以及相对该骨骼的位置、旋转；明确坐标和角度约定。
- 使用目标变换乘源挂点逆变换定位网格，同时处理骨骼层级、权重、法线和形变数据。
- 若配件替换枪身原有网格，需要明确替换规则，不能通过名称猜测删除。
- 缺少挂点配置时显示缺失项，不静默把部件堆在原点。

可以通过武器/附件配置、包含完整挂点的骨架或正确拼装的参考文件确定布局。若这些数据不可获得，应由用户确认采用“手动挂点与偏移”工作流，不能宣称自动还原原游戏装配。

## 外部资料的适用范围

[Modme 的 BO3 附件配置说明](https://github.com/dtzxporter/ModmeWiki/blob/master/wiki/black_ops_3/guides/Setting-Up-Weapon-Attachments.md) 明确区分附件模型、父关节、位置偏移和旋转偏移。该资料说明了这类配置的形式，但不是这组 BOCW / Krig 6 文件的实际偏移来源，不能从中推导本样本的具体数值。
