# `.blockymodel` 格式规范

**Hytale 静态模型交换格式 —— 第 1 版**

| 项 | 值 |
|---|---|
| 状态 | 稳定（逆向自官方工具链，逐字段与源码核对） |
| 适用对象 | Hytale 游戏客户端、Blockbench Hytale 插件、任何第三方读取器/写入器 |
| 介质类型 | `application/json`，UTF-8，无 BOM，无压缩 |
| 文件扩展名 | `.blockymodel` |
| 配套文档 | [`blockyanim-spec.md`](./blockyanim-spec.md)（动画）、[`blockymodel-format.md`](./blockymodel-format.md)（实现笔记） |

> **关于「专业格式文档」的说明。** 截至本文撰写，Hypixel Studios **没有发布** `.blockymodel` / `.blockyanim` 的正式规范（JSON Schema、IDL 或 RFC）。
> Hytale Wiki 的 [Technical:Documentation](https://hytalewiki.org/w/Technical:Documentation) 直接写明：官方「正在准备托管在 GitBook 上的公开创作者文档」，但**目前没有官方技术文档**。插件仓库没有 `docs/`、没有 wiki、没有 schema 文件；`docs.hytale.com` 只有运行时/服务端类的 Javadoc，没有文件格式页。
>
> 因此**事实上的规范就是官方 Blockbench 插件 `JannisX11/hytale-blockbench-plugin` 的 `compile()` / `parse()`**，配合插件发行包内的 [`dist/about.md`](https://github.com/JannisX11/hytale-blockbench-plugin)（Hypixel Studios 自己撰写的格式指南）。本文以这两者为一级来源，以 Go 实现 `hytale-tools/blockymodel-merger` 与游戏自带资产 `Pets/Dog` 为交叉验证。
>
> ⚠️ **公开资料中有伪造的 schema。** 搜索引擎排名最高的 [hytale-docs.com「3D Models」页](https://hytale-docs.com/docs/modding/art-assets/models) 印的是一套**完全虚构的 Minecraft Bedrock / GeckoLib 风格 schema**（`format_version`、`model.identifier`、`bones[].pivot/cubes`、cube 的 `origin/size/uv/inflate/mirror`、`visible_bounds_*`、`display.hand.*`、「1 pixel = 1/16 block」）。它的**每一个**特征字段都与插件源码矛盾。**不要引用它，也不要照它实现。** 详见 [附录 E](#附录-e伪造的公开-schema警示)。

本文用 RFC 2119 的措辞约定：

* **必须 / MUST** —— 违反会导致 Hytale 客户端拒绝加载，或任何合规读取器产生错误几何。
* **应该 / SHOULD** —— 违反通常仍能加载，但会产生不符合官方工具链的行为。
* **可以 / MAY** —— 完全自由，读取器不应依赖。

---

## 目录

1. [术语](#1-术语)
2. [坐标系、单位与手性](#2-坐标系单位与手性)
3. [文件结构](#3-文件结构)
4. [顶层对象](#4-顶层对象)
5. [Node（骨骼）](#5-node骨骼)
6. [Shape（形状）](#6-shape形状)
7. [textureLayout（UV 布局）](#7-texturelayoutuv-布局)
8. [UV 求解算法](#8-uv-求解算法)
9. [几何生成](#9-几何生成)
10. [bind pose 变换](#10-bind-pose-变换)
11. [约束与资源限制](#11-约束与资源限制)
12. [写入者清单](#12-写入者清单)
13. [写入者禁忌](#13-写入者禁忌)
14. [与其他实现的差异](#14-与其他实现的差异)
15. [附录 A：完整示例](#附录-a完整示例)
16. [附录 B：参考资产实测数据](#附录-b参考资产实测数据)
17. [附录 C：JSON 语法摘要](#附录-cjson-语法摘要)
18. [附录 D：参考来源](#附录-d参考来源)
19. [附录 E：伪造的公开 schema 警示](#附录-e伪造的公开-schema-警示)

---

## 1. 术语

| 术语 | 含义 |
|---|---|
| **模型单位（unit）** | 文件里所有数字的单位，等于 Blockbench 的「像素」。见 §2.4。 |
| **节点 / node** | 模型层级中的一个元素。在 `.blockymodel` 里它**同时**是骨骼和（可选的）形状载体。 |
| **骨骼 / bone** | 节点的变换语义角色：它有自己的局部坐标系，子节点相对它定位，动画按它的名字绑定。 |
| **形状 / shape** | 节点携带的几何体：`box`、`quad` 或 `none`。 |
| **bind pose** | 模型文件本身描述的静止姿势，即所有节点都不施加动画时的姿势。 |
| **unwrap / UV 展开** | 把形状的面映射到贴图像素矩形上的过程。 |
| **主形状 / main shape** | Blockbench 编辑器概念：一个 group 下第一个「旋转为 0」的直接子 cube。它是该骨骼的几何来源，也是 `stretch` / `visible` / UV 动画的作用对象。写出的文件里这个概念已经消失（被压平成节点），只在理解导出语义时需要。 |

---

## 2. 坐标系、单位与手性

### 2.1 轴与手性

| 项 | 值 | 依据 |
|---|---|---|
| 手性 | **右手系** | 官方插件 `formats.ts` 的 `forward_direction: '+z'` |
| 上方向 | **+Y** | 同上 |
| 前方向 | **+Z** | 同上 |
| 面朝向 | 逆时针（CCW）为正面 | §9.4 |
| 四元数分量序 | `(x, y, z, w)`，**标量在后** | `IQuaternion` 类型定义；与 `glam::Quat`、glTF 一致 |

### 2.2 面命名映射

`.blockymodel` 用的是 Hytale 自己的面命名，而 Blockbench 内部用 Minecraft 系（`north/south/east/west/up/down`）。映射是**规范的一部分**，不是实现细节：

| `.blockymodel` | Blockbench | 外法线 | 沿面内轴 |
|---|---|---|---|
| `front` | `south` | **+Z** | 宽 → X，高 → Y |
| `back` | `north` | **−Z** | 宽 → X，高 → Y |
| `right` | `east` | **+X** | 宽 → Z，高 → Y |
| `left` | `west` | **−X** | 宽 → Z，高 → Y |
| `top` | `up` | **+Y** | 宽 → X，高 → Z |
| `bottom` | `down` | **−Y** | 宽 → X，高 → Z |

> 最容易记错的一条：**`front` 是 +Z**，也就是「Hytale 的前方」。在 Blockbench 的默认视角里它叫 `south`。

### 2.3 `settings.normal` 取值

`quad` 的法线用带符号的轴名字符串：

```
"+X" | "-X" | "+Y" | "-Y" | "+Z" | "-Z"
```

缺省（字段缺失或为空串）视为 **`"+Z"`**。

### 2.4 单位

文件里的数字就是 Blockbench 的模型单位。官方插件用 `ModelFormat.block_size` 声明「一个世界方块等于多少模型单位」：

| 格式 ID | `block_size` | 用途 |
|---|---|---|
| `hytale_character` | **64** | 角色、生物、附件 |
| `hytale_prop` | **32** | 道具、方块模型 |

`block_size` **不写在文件里**，读取器要靠上下文（通常是资产目录，或顶层 `format` 字段）自行判断。它只影响「模型单位 → 世界方块」的换算：`blocks = units / block_size`。

> ⚠️ `hytale-tools/blockymodel-merger` 的 GLB 导出硬编码了 `scale = 1/16`（`pkg/export/glb.go`），那是为 Minecraft 系预览习惯写的，对 Hytale 角色相当于放大 4 倍。**不要照抄这个常数。**

### 2.5 贴图约定

* 一个模型同一时刻**只使用一张贴图**（`single_texture_default: true`）。
* 默认环绕模式是 **ClampToEdge**（`texture_wrap_default: 'clamp'`）。
* 贴图的 **UV 尺寸（`uv_width`/`uv_height`）必须等于它的像素分辨率**。官方插件的校验器会因此报错：「确保像素密度是角色 64、道具 32」。换句话说，贴图分辨率与 `block_size` 无关，但作者通常让贴图尺寸是 64/32 的整数倍。
* 官方插件按下述顺序**自动发现**贴图（`discoverTexturePaths`）：
  1. 与模型同目录、以**模型文件名开头**的 `.png`，或名为 `Texture.png` 的 `.png`；
  2. `<模型文件名>_Textures/` 子目录下的所有 `.png`。
* 贴图是**像素艺术**，采样器应该用 `Nearest`（见 §8.5）。

---

## 3. 文件结构

* 编码：**UTF-8**，无 BOM。
* 内容：**单个 JSON 对象**（不是数组、不是 JSON Lines）。
* 无魔数、无头部、无压缩、无版本号字段区分。
* 缩进与换行不受限制；官方 Blockbench 插件输出的格式由用户的 `json_compile_options` 决定，游戏自带资产是 2 空格缩进。
* 未知键**必须**被忽略（前向兼容），不得报错。

---

## 4. 顶层对象

```jsonc
{
  "nodes": [ /* Node，1 个或多个根节点 */ ],
  "format": "character",     // 可选
  "lod": "auto"              // 可选
}
```

| 字段 | 类型 | 出现 | 语义 |
|---|---|---|---|
| `nodes` | `Node[]` | **必须** | **根节点数组**。格式允许多个根；官方资产 `Pets/Dog` 只有 1 个（`Pelvis`）。空数组是合法的「空模型」。 |
| `format` | `"character"` \| `"prop"` | 可选 | 官方插件**总是**写出该字段，取值为 `Format.id == 'hytale_prop' ? 'prop' : 'character'`。它决定编辑器用哪套 `block_size`。读取器可以忽略。 |
| `lod` | `"auto"` | 可选 | 官方插件总是写 `"auto"`。至今只见过这一个取值。读取器可以完全忽略。 |

读取器对 `nodes` 缺失时的建议行为：视为空模型（渲染出空场景），而不是报错。

---

## 5. Node（骨骼）

```jsonc
{
  "id": "12",
  "name": "Pelvis",
  "position":    { "x": 0, "y": 35, "z": -20 },
  "orientation": { "x": -0.051265, "y": 0, "z": 0, "w": 0.998685 },
  "shape": { /* §6 */ },
  "children": [ /* Node[] */ ]
}
```

| 字段 | 类型 | 出现 | 缺省 | 语义 |
|---|---|---|---|---|
| `id` | **字符串** | 应该 | — | 节点标识。**是字符串，不是数字**（官方插件写 `"1"`, `"2"`, …）。只在同一文件内唯一；**不保证连续、不保证从 1 开始**（见 §13.2）。合并工具用它做去重键。 |
| `name` | 字符串 | 必须 | — | 骨骼名。**动画按名字绑定**，因此这是文件里最重要的字段。**名字不保证唯一**（见 §11.7）。 |
| `position` | `Vec3` | 可选 | `(0,0,0)` | **相对父骨骼原点的平移**，见 §10。 |
| `orientation` | `Quat` | 可选 | 单位四元数 | 该节点自身的旋转。写作者**应该**输出归一化四元数。 |
| `shape` | `Shape` | 可选 | 无 = 视觉上等价于 `type: "none"` | 见 §6。 |
| `children` | `Node[]` | 可选 | 空 | 子节点。**允许缺失，也允许存在但是 `[]`。** 两者语义相同。 |

### 5.1 `id` 的字符串性

`id` 必须是 JSON 字符串。用强类型语言解析时**不要**直接反序列化成整数——官方插件从 `node_id.toString()` 写出，而游戏自带资产（以及任何做过合并的文件）里的 `id` 是一组稀疏数字串。

`Pets/Dog/Models/Model.blockymodel` 的实例证据：31 个节点，`id` 集合为

```
1, 2, 4, 12, 58, 59, 75, 79, 80, 84, 85, 88, 89, 90, 92,
107, 108, 109, 110, 112, 113, 114, 118, 119, 120, 121, 122, 123, 124, 130, 131
```

—— 最小 1、最大 131、缺号极多。**任何把 `id` 当作数组下标的实现都会崩。**

### 5.2 名字到节点的绑定不唯一

动画轨道按 `name` 绑定，而 `name` **不保证唯一**：同一个模型里可以有多个同名节点，且这是官方插件**有意支持**的功能（设置项 `hytale_duplicate_bone_names`，描述原文：「Multiple groups with the same name can be used to apply animations to multiple nodes at once」）。

因此正确的绑定表是 `HashMap<String, Vec<NodeIndex>>`，采样时驱动**全部**同名节点。`Pets/Dog` 里 `L-Ear2` 就出现了两次（其中一次是原作者把 `R-Ear` 的子节点误命名成了 `L-Ear2`）。

---

## 6. Shape（形状）

```jsonc
"shape": {
  "offset":  { "x": 0, "y": 0, "z": 0 },
  "stretch": { "x": 1, "y": 1.113824, "z": 1 },
  "type": "box",
  "settings": { "size": { "x": 27, "y": 19, "z": 22 } },
  "textureLayout": { /* §7 */ },
  "unwrapMode": "custom",
  "visible": true,
  "doubleSided": false,
  "shadingMode": "flat"
}
```

| 字段 | 类型 | 出现 | 缺省 | 语义 |
|---|---|---|---|---|
| `offset` | `Vec3` | 应该 | `(0,0,0)` | 形状几何中心相对**本节点原点**的平移。**同时是子节点的枢轴偏移**，见 §10.2。 |
| `stretch` | `Vec3` | 应该 | `(1,1,1)` | **缩放倍数**，不是绝对尺寸。最终尺寸 = `settings.size * stretch`。**允许为负**（镜像几何体，翻转绕序，见 §9.4）。 |
| `type` | `"box"` \| `"quad"` \| `"none"` | 应该 | 视为 `none` | 形状类别。 |
| `settings` | 对象 | 可选 | `{}` | 见 §6.1。 |
| `textureLayout` | 对象 | **必须**（写入时） | `{}` | 面 → UV 矩形。**写入者必须在每个 shape 上显式输出该键，哪怕是 `{}`**，见 §12。 |
| `unwrapMode` | `"custom"` | 可选 | — | 至今只见过 `"custom"`。读取器可以完全忽略。 |
| `visible` | 布尔 | 可选 | `true` | 初始可见性。 |
| `doubleSided` | 布尔 | 可选 | `false` | 是否渲染背面，见 §9.5。 |
| `shadingMode` | 枚举 | 可选 | `"flat"` | 着色模式，见 §6.2。 |

> **关键**：即使 `type == "none"`，`shape.offset` **仍然必须参与子节点的变换**。参考实现（Go `pkg/render/scene.go`）对任何 shape 都读取 offset 传给子节点，只有「是否生成网格」这一步才检查 `type`。

### 6.1 `settings`

| 字段 | 类型 | 适用 | 缺省 | 语义 |
|---|---|---|---|---|
| `size` | `Vec3` 或 `Vec2` | `box` / `quad` | 无 | **`box` 写 `{x,y,z}`；`quad` 只写 `{x,y}`**（Z 分量不写）。`none` 没有 `size`。这是**未乘 `stretch` 的绝对尺寸**。 |
| `normal` | `"+X"` 等 | 仅 `quad` | `"+Z"` | 四边形所在平面的外法线。 |
| `isPiece` | 布尔 | 任意 | `false` | `true` 表示这是**附件挂点**：该节点在作为 blockymodel 附件导入时，会挂到主模型同名骨骼上。 |
| `isStaticBox` | `true` | 任意 | `false`（不写） | 编辑器专用：表示这个 cube 直接代表它的 group（没有独立 group），用于重建 Blockbench 层级。**渲染器必须忽略它。** |

写入者注意：官方插件**总是**写出 `settings.isPiece`（即使是 `false`），而游戏自带资产**完全没有** `settings` 里的 `isPiece`/`isStaticBox` 键。读取器必须把两者都当作正常输入。

### 6.2 `shadingMode`

| 值 | 编辑器显示名 | 语义 |
|---|---|---|
| `flat` | Flat | **默认**。不参与光照，贴图原样输出。 |
| `standard` | Standard | 参与方向光漫反射。 |
| `fullbright` | Always Lit | 明确的全亮（同样不参与光照）。 |
| `reflective` | Reflective | 不参与光照，由游戏渲染环境反射/镜面效果。 |

Hytale 官方资产在 `Pets/Dog` 里**全部**是 `flat`。对第三方渲染器的最小实现：把 `flat` / `fullbright` / `reflective` 一律按「不参与光照」处理（等价于一个 `unlit` 标志），只对 `standard` 做光照。

---

## 7. `textureLayout`（UV 布局）

`textureLayout` 把**面名**映射到该面在贴图里的 UV 矩形。**面按需出现**——一个 `box` 可以只定义 3 个面，其余面不生成几何、不渲染。

```jsonc
"textureLayout": {
  "top":   { "offset": { "x": 83, "y": 47 }, "mirror": { "x": false, "y": false }, "angle": 0 },
  "right": { "offset": { "x": 165, "y": 1 }, "mirror": { "x": true,  "y": false }, "angle": 0 }
}
```

### 7.1 条目字段

| 字段 | 类型 | 出现 | 缺省 | 语义 |
|---|---|---|---|---|
| `offset` | `Vec2` | 必须 | `(0,0)` | **UV 矩形的左上角，单位是贴图像素**（不是 0..1 归一化值）。 |
| `mirror` | `{x: bool, y: bool}` | 可选 | 两者 `false` | 反向采样。`mirror.x = true` 表示矩形从 `offset` 向 **−U** 方向延伸，即 `u1 = offset.x − width`。 |
| `angle` | `0` \| `90` \| `180` \| `270` | 可选 | `0` | UV 旋转。官方资产里只出现 `0` 和 `180`，但实现必须支持四个值。 |
| `transparent` | 布尔 | 可选 | `false` | 该面强制使用透明混合而非 alpha test。 |
| `lockUVs` | 布尔 | 可选 | `false` | 编辑器里禁止自动重排 UV。**渲染器必须忽略。** |

### 7.2 面键名

键名必须是 §2.2 表格中的 Hytale 面名之一（`front` / `back` / `left` / `right` / `top` / `bottom`）。未知键名**应该**被忽略。

### 7.3 `quad` 的特殊规则

`quad` 只写 **`front`** 一个键——官方插件在导出 quad 时把方向强制成 `front`（`blockymodel.ts` 的 `if (node.shape.type == 'quad') direction = 'front'`）。

读取器在 quad 的 `settings.normal` 所对应的自然面**没有** UV 条目时，**应该**回退到 `front`。参考实现（Go `pkg/render/geometry.go`）就是这么做的。对官方插件产出的资产，这个回退总是会命中，因为自然面条目根本不存在。

### 7.4 `textureLayout` 必须存在

Hytale 官方资产在**每一个** shape 上都写 `"textureLayout": {}`，即使该形状没有任何 UV。游戏客户端在缺失这个 key 时会加载失败。

* **读取器**：缺失 = 空布局，不得报错。
* **写入器**：**必须**在每个 `shape` 上显式输出 `"textureLayout"`，哪怕值是 `{}`。用 Go/Rust 的 `omitempty` 序列化会自动丢掉空 map，这是一个真实的踩坑点——参考实现在 `pkg/blockymodel/io.go` 里专门重写了 `MarshalJSON` 来强制补上这个键。

---

## 8. UV 求解算法

本节是 `.blockymodel` 里最容易实现错误的部分。下面的算法等价于官方插件 `src/blockymodel.ts` 的 `parse()`（UV 部分）+ Blockbench 的 `getUVArray`，并已与 Go 参考实现交叉验证。

### 8.1 第一步：UV 矩形的像素尺寸

用**未乘 `stretch` 的 `settings.size`**：

| 面 | `uvWidth` | `uvHeight` |
|---|---|---|
| `front` / `back` | `size.x` | `size.y` |
| `left` / `right` | `size.z` | `size.y` |
| `top` / `bottom` | `size.x` | `size.z` |
| `quad`（任意法线） | `size.x` | `size.y` |

> 因为尺寸取自 `size` 而非 `size * stretch`，`stretch` 会**拉伸几何但保持 UV 采样窗口不变**，效果是贴图被拉长。这是 Blockbench 的既定行为，不是 bug。

### 8.2 第二步：计算未旋转的 `[u0, v0, u1, v1]`

设 `ox, oy = textureLayout.offset`，`w, h = uvWidth, uvHeight`，
`mx = mirror.x ? -1 : +1`，`my = mirror.y ? -1 : +1`：

```
angle == 0:
    [u0,v0,u1,v1] = [ ox,           oy,           ox + w*mx,     oy + h*my ]

angle == 90:
    swap(w,h); swap(mx,my); mx = -mx
    [u0,v0,u1,v1] = [ ox,           oy + h*my,    ox + w*mx,     oy ]

angle == 180:
    mx = -mx; my = -my
    [u0,v0,u1,v1] = [ ox + w*mx,    oy + h*my,    ox,            oy ]

angle == 270:
    swap(w,h); swap(mx,my); my = -my
    [u0,v0,u1,v1] = [ ox + w*mx,    oy,           ox,            oy + h*my ]
```

（`90` / `270` 分支里的 `w,h,mx,my` 已经是交换后的值。）

`v` 轴向下（**图片坐标系**）：`v0` 是矩形的上边，`v1` 是下边。

**等价形式**（Go 参考实现采用，两者可互相推导）：先按 `angle == 0` 求出面内参数 `(u,v) ∈ [0,1]²` 对应的贴图像素

```
tu = ox + u * (mirror.x ? -w : +w)
tv = oy + v * (mirror.y ? -h : +h)
```

再把 `(tu, tv)` 绕 `(ox, oy)` 旋转 `angle` 度（图像坐标系下顺时针）：

```
90  → (ox - (tv - oy), oy + (tu - ox))
180 → (ox - (tu - ox), oy - (tv - oy))
270 → (ox + (tv - oy), oy - (tu - ox))
```

### 8.3 第三步：把矩形映射到 4 个角

面顶点按**从外部看**的顺时针顺序 `[TL, TR, BL, BR]` 排列（`TL` = 左上 = 最小 U、最小 V）。先做**无旋转**的角点赋值：

```
corner[0] = TL = (u0, v0)
corner[1] = TR = (u1, v0)
corner[2] = BL = (u0, v1)
corner[3] = BR = (u1, v1)
```

然后按 `angle / 90` 次应用同一个**角点轮换**：

```
tmp = a[0]; a[0] = a[2]; a[2] = a[3]; a[3] = a[1]; a[1] = tmp;
// [TL, TR, BL, BR] → [BL, TL, BR, TR]
```

即 `angle == 90` 轮换 1 次，`180` 轮换 2 次，`270` 轮换 3 次。

### 8.4 第四步：归一化

```
u_tex = u_pixel / texture_width
v_tex = v_pixel / texture_height
```

**在 wgpu / WebGPU / OpenGL-with-top-left-UV 约定下不需要翻转 V。** `.blockymodel` 的 `offset` 用的是「图片坐标系」（原点左上、Y 向下），wgpu 的纹理坐标原点**也是左上**，行序一致，直接除即可。

> 参考实现的 GLB 导出里有一对看起来矛盾的 `1 - v` 翻转（`glb.go` 的 step 2 和 step 3），它们**互相抵消**——那是为绕开 glTF 生态的历史包袱。不要在 wgpu 里照抄。

### 8.5 第五步：防止接缝渗色

相邻 UV 岛屿在 mipmap / 双线性采样下会互相渗色。两个手段：

1. **把 UV 向内缩进约 1/8 像素**（参考实现的做法，`glb.go`）：
   ```
   inset = 0.125 / texture_size
   u0 < u1 ? (u0 += inset_u, u1 -= inset_u) : (u0 -= inset_u, u1 += inset_u)
   v0 < v1 ? (v0 += inset_v, v1 -= inset_v) : (v0 -= inset_v, v1 += inset_v)
   ```
2. **用 `Nearest` 采样**，或强制 `LodMinClamp = 0`。

Hytale 的贴图是**像素艺术**（参考资产是 256×128），实际渲染**应该**用 `Nearest`。

---

## 9. 几何生成

### 9.1 `box`

6 个面，每个面**只有出现在 `textureLayout` 里才生成**。每个面 4 个顶点（顺序固定为从外部看 `TL, TR, BL, BR`）、2 个三角形。

设 `h = size * |stretch| / 2` 为半尺寸，`o = shape.offset`。六个面的**规范顶点表**（每个顶点再加 `o`）：

| 面 | 法线 | TL | TR | BL | BR |
|---|---|---|---|---|---|
| `right` (+X) | (1,0,0) | (+hx, +hy, +hz) | (+hx, +hy, −hz) | (+hx, −hy, +hz) | (+hx, −hy, −hz) |
| `left` (−X) | (−1,0,0) | (−hx, +hy, −hz) | (−hx, +hy, +hz) | (−hx, −hy, −hz) | (−hx, −hy, +hz) |
| `top` (+Y) | (0,1,0) | (−hx, +hy, −hz) | (+hx, +hy, −hz) | (−hx, +hy, +hz) | (+hx, +hy, +hz) |
| `bottom` (−Y) | (0,−1,0) | (−hx, −hy, +hz) | (+hx, −hy, +hz) | (−hx, −hy, −hz) | (+hx, −hy, −hz) |
| `front` (+Z) | (0,0,1) | (−hx, +hy, +hz) | (+hx, +hy, +hz) | (−hx, −hy, +hz) | (+hx, −hy, +hz) |
| `back` (−Z) | (0,0,−1) | (+hx, +hy, −hz) | (−hx, +hy, −hz) | (+hx, −hy, −hz) | (−hx, −hy, −hz) |

**三角形索引（从外部看 CCW）**：`(0,2,1)` 和 `(2,3,1)`。

验证 `front` 面：`BL−TL = (0,−2hy,0)`，`TR−TL = (2hx,0,0)`，叉积 `= (0,0,4·hx·hy)` → +Z，正面朝外 ✓

`stretch` 直接乘在顶点上（等价于 `T(offset) · S(stretch)` 作用于中心在原点的立方体），但 **UV 尺寸用的是未乘 stretch 的 `size`**（§8.1）。

### 9.2 `quad`

一个四边形，几何上等于把法线轴方向的厚度设为 0 的 `box` 面。

* `settings.size` 只有 `{x, y}`，语义是**面内两个轴的尺寸**：
  * 法线 `±Z`：x = 沿 X 的宽度，y = 沿 Y 的高度
  * 法线 `±X`：x = 沿 **Z** 的宽度，y = 沿 Y 的高度
  * 法线 `±Y`：x = 沿 X 的宽度，y = 沿 **Z** 的高度
* 顶点表直接复用 §9.1 中对应 `box` 面的 TL/TR/BL/BR，把法线轴分量置 0。等价地：`half` 在法线轴上是 0，在另外两轴上按上表的映射取值。

  注意映射不是「`size.x` 永远给 X」：对 `±X` 法线，`size.x` 落在 **Z** 轴（半尺寸 `size.x/2`），`size.y` 落在 Y 轴；对 `±Y` 法线，`size.x` 落在 X 轴，`size.y` 落在 **Z** 轴。
* UV 尺寸 = `(settings.size.x, settings.size.y)`。
* UV 条目**只查 `front`**（§7.3）。

参考资产 `Pets/Dog` 里的 5 个 quad 是：`R-Eye`、`L-Eye`、`L-Ear2`、`R-Ear2`、`TongueDog2`。它们**都没有 `settings.normal` 字段**，即全部按缺省的 `+Z` 处理。

### 9.3 `none`

不生成任何几何体，但仍然参与层级变换（它的 `offset` 会传递给子节点）。出现在：

* 纯骨骼节点（挂点、物理骨、终结节点）；
* 附件挂点（`settings.isPiece: true`）。

### 9.4 绕序与负 `stretch`

`stretch` 分量为负时几何体沿该轴镜像，**三角形绕序会翻转**。规则是：

```
oddFlips = (stretch.x < 0) + (stretch.y < 0) + (stretch.z < 0)  为奇数
```

`oddFlips` 为真时，该形状的所有面改用反向索引（等价于交换三角形的后两个顶点）。参考实现见 Go `pkg/export/glb.go` 与 `pkg/render/render.go`：

```go
oddFlips := (flipX != flipY) != flipZ
```

**法线也要一并取符号**：`normal = face_normal * sign(stretch)`，逐分量。

### 9.5 `doubleSided`

`doubleSided: true` 时不剔除背面。两种实现方式：

* **推荐**：该形状走 `CullMode::None` 的管线，并在片元着色器里对背面翻转法线（WGSL 的 `@builtin(front_facing)`）。
* **参考实现做法**：额外生成一份反向绕序的几何体（Go `pkg/render/geometry.go` 的 `reversedFaces`）。顶点数翻倍，不推荐。

`doubleSided` 是**每形状**属性。`Pets/Dog` 里所有 `quad` 都是 `true`，所有 `box` 都是 `false`——这不是巧合，quad 是零厚度的面，没理由只渲染一面。

---

## 10. bind pose 变换

这是整个格式的骨架。Blockbench 的层级是「group（骨骼）→ cube（形状）」两层；`.blockymodel` 把它压成了一层：**一个 Node 同时承担骨骼和形状两种角色**。

### 10.1 递归公式

```
bone_world(N)  = bone_world(P) · T(P.shape.offset + N.position) · R(N.orientation)
shape_world(N) = bone_world(N) · T(N.shape.offset) · S(N.shape.stretch)
```

其中：

* `T` = 平移矩阵；`R` = 由四元数 `(x,y,z,w)` 构造的旋转矩阵；`S` = 分量缩放矩阵。
* **根节点**没有父节点，取 `bone_world(root) = T(root.position) · R(root.orientation)`——即把 `P.shape.offset` 与 `bone_world(P)` 都当作单位变换。
* 传给子节点的父偏移是 **`P.shape.offset` 的原始值（未乘 stretch）**。

等价的分步写法（`Pets/Dog` 的 `Pelvis` 是根）：

```
Pelvis:  bone = T(0,35,-20) · R(orientation)
Tail:    bone = bone(Pelvis) · T(Pelvis.shape.offset + Tail.position) · R(Tail.orientation)
```

### 10.2 三条容易写错的规则

1. **父节点的 `shape.offset` 必须加进子节点的 `position`。**
   这不是笔误。Blockbench 的 group 有独立的 `origin`（枢轴点），而 cube 的中心是 `origin + offset + size/2`。换算下来，子 group 的 origin = `父 origin + 父 offset + 子 position`。所以 `shape.offset` 实际上是一个「枢轴偏移」，会影响整条子链。**漏掉这一项，模型会整体塌陷成一团。**

2. **父节点的 `stretch` 不传给子节点。**
   `S(stretch)` 只作用在该节点自己的网格上。这带来一个有用的推论：**沿层级累乘出来的姿态一定是刚体**，线性部分只有 `R`（加上每个形状自己的 `S`）。

3. **旋转在平移之后。**
   矩阵顺序必须是 `T(...) · R(...)`，不能写成 `R · T`。否则所有骨骼的位置都会跟着自己的旋转飞掉。

### 10.3 网格顶点的生成

对 `box`：

```
half          = settings.size / 2
vertex_local  = shape.offset + stretch * (±half.x, ±half.y, ±half.z)
vertex_world  = bone_world(N) · vertex_local
```

法线矩阵可以**解析地**写出来，不必求逆：

```
normals_mat = R_world(N) · S(1/stretch)
```

因为 `stretch` 只作用于该骨骼自己的网格、不传给子节点，累乘出来的姿态是刚体（见规则 2），线性部分就是 `R · S`，其逆转置正是 `R · S⁻¹`。

---

## 11. 约束与资源限制

### 11.1 节点数上限

**一个模型最多 255 个节点。**

一级来源（插件发行包内的 `dist/about.md`，Hypixel Studios 撰写）：

> *"Models should not have more than 255 nodes. Nodes are a concept in the Hytale model format, **each group counts as a node, and each cube/quad counts except if it's the first cube in a respective group**. The number of nodes can be seen by clicking on the element counter above the outliner."*

插件侧由 `src/validation.ts` 强制（`MAX_NODE_COUNT = 255`），校验消息为：「The model contains N nodes, which exceeds the maximum of 255 that Hytale will display.」

校验器的计数规则（写入者应该按同样规则计数）：

```
对每个 Group（跳过 export=false 的、被 Collection 引用的）：
    count += 1
    对它的每个直接子 Cube（跳过 export=false 的）：
        如果该 Cube 是「主形状」（直接子节点中第一个旋转全为 0 的 cube）→ 跳过
        否则 count += 1
对 Outliner 根上的每个 Cube（export=true）：
    count += 1
```

「主形状不计数」源于格式把 group 和 cube 折叠成一个 Node（§6.1 的 `isStaticBox`、§10.1）：**一个 Node 的几何不额外占一个节点名额**，只有多出来的 cube 才会被导出成独立节点。

**推论**：255 意味着节点索引可以用 `u8` 或 `u16` 表示。

### 11.2 贴图

* 一个模型一张贴图（§2.5）。
* 贴图 UV 尺寸必须等于像素分辨率。`dist/about.md` 原文：*"UVs must always match the dimensions of their face, the format does not consider custom UV sizes."*
* 官方博客给出的密度约定：**角色/附件 64 单位 = 1 方块，道具/方块 32 单位 = 1 方块**；「引擎会自动把角色/附件缩小以匹配世界尺寸」。
* 官方博客给出的尺寸约束：*"Textures can be non-square and must be multiples of 32px (32, 64, 96, 128, etc.)"* —— 即宽和高各自必须是 32 的整数倍，但**不要求是 2 的幂、也不要求是正方形**。
* 官方博客明确禁用的东西：*"No spheres allowed!"*、*"No edge loops, no special topology, no triangles, pyramids…"* —— 只有 `box` 与 `quad` 两种图元。
* 不使用 PBR：*"We aren't using the industry standard PBR workflows (roughness, normal maps, displacement, etc.)"*。

### 11.3 尺寸

官方插件有一个 `hytale_integer_size` 开关（默认开），把 cube 尺寸限制为**整数**。原文说明：「Float values are technically supported but make UV mapping more difficult. Using stretch is recommended instead.」

* **写入者应该**输出整数 `settings.size`，用 `stretch` 表达非整数缩放。
* **读取者必须**接受浮点 `size`。

### 11.4 `stretch` 的取值建议

官方美术指南（[官方博客](https://hytale.com/news/2025/12/an-introduction-to-making-models-for-hytale)）只给了一个**美术风格建议**：

> *"we avoid going under 0.7X and over 1.3X the stretch for a node in one axis."*

这是**建议，不是格式约束**：schema 接受任意浮点数，包括负数。写入者可以超出这个范围，读取器**必须**接受任何值。

### 11.5 膨胀（inflate）

Hytale 格式**没有** inflate 概念，插件会强制把 `cube.inflate` 归零，并禁用相关的 UI。文件里没有对应字段，不要试图添加。

### 11.6 Box UV

`box_uv: false`、`optional_box_uv: true`：默认使用**逐面 UV**；quad 上禁止使用 Box UV。`textureLayout` 的逐面结构已经体现了这一点。

### 11.7 骨骼名

* **不保证唯一**（§5.2）。绑定表必须是 `name → Vec<index>`。
* **不保证在动画里出现的名字都存在于模型里**，见 `blockyanim-spec.md`。
* 官方插件在写文件名时会剥掉 `name` 里第一个 `:` 之前的前缀（`name.replace(/^.+:/, '')`），这是附件命名空间的处理。写入者应该输出不含 `:` 前缀的裸名字。
* 官方美术指南：*"When creating characters, we carefully name each node to make it compatible with our animation system."* —— 名字是动画跨资产复用的唯一契约。

### 11.8 层级

* 允许**多个根节点**。
* 允许任意深度嵌套（受 255 节点总数限制）。
* 允许空模型（`nodes: []`）。

### 11.9 动画能驱动什么

`dist/about.md` 明确了动画的靶点，读取器必须按这个粒度实现：

> *"Hytale uses 'Nodes', which are a combination of a group (for the transformation and hierarchy) and a cube (for the visual part). **Position and Rotation animations target the group**, so all child groups and cubes inherit it. However, **Scale, Visibility and UV Offset only target the Shape itself**, so it only applies to the cube."*

对应到本格式：

| 通道 | 靶点 | 是否影响子节点 |
|---|---|---|
| `position`、`orientation` | 节点自身的变换（`bone_world`） | **是**，子节点继承 |
| `shapeStretch` | 该节点的形状（`S(stretch)`） | **否** |
| `shapeVisible` | 该节点的形状 | **否** |
| `shapeUvOffset` | 该节点的形状的面 UV | **否** |

同一段文字还确认了 `stretch` 的传播规则：

> *"keep in mind that child bones and other cubes are not affected by the scale."*

---

## 12. 写入者清单

按重要性排序。前 4 条不满足会导致游戏客户端拒绝加载或模型损坏。

1. **每个 `shape` 都必须输出 `textureLayout`**，哪怕是 `{}`。（§7.4）
2. **`shape.offset` 必须正确参与子节点定位**——写出时它等于「子节点的枢轴相对本节点的偏移」。（§10.2）
3. **`stretch` 是倍数，缺省 `(1,1,1)` 而不是 `(0,0,0)`；`orientation` 缺省是单位四元数而不是零四元数。**
4. **`id` 必须是字符串**，且在同一文件内唯一。（§5.1）
5. 输出顶层 `format`（`"character"` 或 `"prop"`）——官方插件总是写它。
6. 输出顶层 `lod: "auto"`——官方插件总是写它。
7. 每个 `shape` 输出 `unwrapMode: "custom"`。
8. 每个 `shape` 输出 `type`、`settings.size`、`stretch`、`visible`、`doubleSided`、`shadingMode`，即使取缺省值——这是官方插件的输出形态，能让 diff 更稳定。
9. 节点数 ≤ 255。（§11.1）
10. 四元数归一化后再写出。
11. 面键名只用 §2.2 的六个名字。
12. `textureLayout.offset` 用**像素**，`angle` 用 `0/90/180/270` 的整数。

---

## 13. 写入者禁忌

### 13.1 不要用 `omitempty` 序列化 `textureLayout`

Go 的 `json:"textureLayout,omitempty"`、Rust 的 `#[serde(skip_serializing_if = "HashMap::is_empty")]` 都会把空布局整个丢掉，从而产出游戏客户端拒绝加载的文件。参考实现为此专门实现了自定义 `MarshalJSON`。

### 13.2 不要假设 `id` 是连续整数

`id` 是稀疏的字符串集合（§5.1）。写入时可以自行分配（官方插件从 `"1"` 开始递增，但重新导出时**不复用**原有 id）。

### 13.3 不要把 `isPiece` / `isStaticBox` 当几何信息

`isStaticBox` 纯粹是编辑器重建层级用的提示，**渲染器必须忽略**。`isPiece` 只影响附件的挂接行为。

### 13.4 不要对 `shape.offset` 乘 `stretch`

传给子节点的是**原始 offset**。参考实现的所有路径都一致。

### 13.5 不要丢掉 `format` / `lod`

Go 参考实现的 `BlockyModel` 结构体只有 `lod` 和 `nodes` 两个字段，**没有 `format`**——用它做「读入 → 写出」会静默丢失 `format` 字段。自定义实现不要重复这个疏漏。

---

## 14. 与其他实现的差异

| 主题 | 官方插件 | Go `blockymodel-merger` | 本仓库 `crates/blockymodel` |
|---|---|---|---|
| `format` 字段 | 总是写出 | 结构体里没有，写回时丢失 | 解析并保留（`Option<String>`） |
| `settings.isPiece` | 总是写出 | 在 `settings` map 里保留 | 解析为 `bool`，缺省 `false` |
| `isStaticBox` | 条件写出 | 保留在 `settings` map | 解析为 `bool`，缺省 `false` |
| GLB 缩放 | 无（编辑器） | `1/16`（与 Hytale 角色不符） | 不缩放，按单位直出 |
| UV 旋转 | 四分支公式 | 绕 `offset` 旋转已映射的点 | 四分支公式 + 角点轮换 |
| `sphere`/`cylinder` | 不支持 | `HasGeometry()` 里出现但几何生成不支持 | 不支持（`ShapeType::Unknown` 被忽略） |
| V 轴翻转 | 不适用 | GLB 路径有一对互相抵消的 `1-v` | 不翻转（`Nearest` + 左上原点） |

> `sphere` / `cylinder`：Go 的 `Node.HasGeometry()` 把它们列为「有几何」，但 `generateGeometry()` 的 switch 里没有分支，实际不产出任何面。Hytale 至今**没有**在静态模型里使用这两个类型；读取器可以安全地把它们当作 `none`。

---

## 附录 A：完整示例

一个双骨骼模型：根骨骼 `Root`（无几何），子骨骼 `Body`（一个 8×12×6 的盒子，`stretch.y = 1.5`，带贴图）。

```json
{
  "nodes": [
    {
      "id": "1",
      "name": "Root",
      "position":    { "x": 0, "y": 0, "z": 0 },
      "orientation": { "x": 0, "y": 0, "z": 0, "w": 1 },
      "shape": {
        "offset":  { "x": 0, "y": 0, "z": 0 },
        "stretch": { "x": 1, "y": 1, "z": 1 },
        "type": "none",
        "settings": { "isPiece": false },
        "textureLayout": {},
        "unwrapMode": "custom",
        "visible": true,
        "doubleSided": false,
        "shadingMode": "flat"
      },
      "children": [
        {
          "id": "2",
          "name": "Body",
          "position":    { "x": 0, "y": 6, "z": 0 },
          "orientation": { "x": 0, "y": 0, "z": 0, "w": 1 },
          "shape": {
            "offset":  { "x": 0, "y": 0, "z": 0 },
            "stretch": { "x": 1, "y": 1.5, "z": 1 },
            "type": "box",
            "settings": { "size": { "x": 8, "y": 12, "z": 6 }, "isPiece": false },
            "textureLayout": {
              "front":  { "offset": { "x": 0,  "y": 0  }, "mirror": { "x": false, "y": false }, "angle": 0 },
              "back":   { "offset": { "x": 24, "y": 0  }, "mirror": { "x": false, "y": false }, "angle": 0 },
              "left":   { "offset": { "x": 16, "y": 0  }, "mirror": { "x": false, "y": false }, "angle": 0 },
              "right":  { "offset": { "x": 8,  "y": 0  }, "mirror": { "x": false, "y": false }, "angle": 0 },
              "top":    { "offset": { "x": 8,  "y": 12 }, "mirror": { "x": false, "y": false }, "angle": 0 },
              "bottom": { "offset": { "x": 16, "y": 12 }, "mirror": { "x": false, "y": false }, "angle": 0 }
            },
            "unwrapMode": "custom",
            "visible": true,
            "doubleSided": false,
            "shadingMode": "flat"
          },
          "children": []
        }
      ]
    }
  ],
  "format": "character",
  "lod": "auto"
}
```

`Body` 的世界变换：

```
bone_world(Body) = T(0,0,0) · R(identity)          // Root
                 · T(Root.shape.offset + Body.position) · R(identity)
                 = T(0, 6, 0)

shape_world(Body) = bone_world(Body) · T(0,0,0) · S(1, 1.5, 1)
```

几何半尺寸 = `(8,12,6)/2 * (1,1.5,1)` = `(4, 9, 3)`，所以 `Body` 占据世界空间的 `y ∈ [6−9, 6+9] = [−3, 15]`。

`front` 面的 UV：`uvWidth = size.x = 8`，`uvHeight = size.y = 12`，`angle = 0`，无镜像 →
`[u0,v0,u1,v1] = [0, 0, 8, 12]`，角点 `TL=(0,0)`、`TR=(8,0)`、`BL=(0,12)`、`BR=(8,12)` 像素，
归一化时除以贴图尺寸（假定 64×64）→ `TL=(0,0)`、`TR=(0.125,0)`、`BL=(0,0.1875)`、`BR=(0.125,0.1875)`。

---

## 附录 B：参考资产实测数据

`Pets/Dog/Models/Model.blockymodel`——Hytale 官方宠物资产，可直接用作实现的对照基线。

| 项 | 值 |
|---|---|
| 字节数 | 98 196 |
| 行数 | 2 950 |
| 顶层键 | `nodes`, `lod`（**没有 `format`**） |
| 根节点数 | 1（`Pelvis`） |
| 节点总数 | 31 |
| `type` 分布 | `box` 26 / `quad` 5 / `none` 0 |
| `id` 取值范围 | `"1"` .. `"131"`，稀疏 |
| `unwrapMode` 取值 | 只有 `"custom"` |
| `shadingMode` 取值 | 只有 `"flat"` |
| `angle` 取值 | 只有 `0` 和 `180` |
| `settings` 里的键 | 只有 `size`（无 `isPiece`、无 `isStaticBox`） |
| quad 的 `settings.normal` | **全部缺失**（按缺省 `+Z` 处理） |
| 每个 shape 都有 `textureLayout` | 是（31/31；无空对象——所有形状都至少有一个面） |
| 潜在面数 / 实际面数 | 26×6 + 5 = 161 潜在 → **148 实际**（13 个面缺 UV 条目，不生成） |
| 生成几何 | 148 面 → 592 顶点 / 296 三角形 |
| **负 `stretch` 的形状** | **8 个，全部 `x < 0`，负轴个数全为奇数**（见下） |
| `visible` 取值 | 全部 `true` |
| 贴图 | `Models/Texture.png`，256 × 128，32 bpp RGBA |
| 重名骨骼 | `L-Ear2` × 2（其中一次是原作者把 `R-Ear` 的子节点误命名） |
| bind pose 包围盒 | min `(−21.95, −0.34, −43.76)` / max `(21.95, 88.20, 51.15)` |
| 尺寸 | `43.90 × 88.54 × 94.91` 单位 ≈ `0.69 × 1.38 × 1.48` 方块（÷64） |

骨骼树（缩进 = 父子关系）：

```
Pelvis
├── Tail → Tail2 → Tail3
└── Chest
    ├── Neck → Neck2 → Head
    │   ├── Snout → Jaw
    │   │   ├── TongueDog → TongueDog2 (quad)
    │   │   └── Teeth
    │   ├── R-Eye (quad)
    │   ├── L-Ear → L-Ear2 (quad)
    │   ├── L-Eye (quad)
    │   └── R-Ear → (子节点也叫 L-Ear2, quad)
    ├── R-UpperLeg → R-Leg → R-Foot
    ├── L-UpperLeg → L-Leg → L-Foot
    ├── R-BackUpperLeg → R-BackLeg → R-BackFoot
    └── L-BackUpperLeg → L-BackLeg → L-BackFoot
```

**负 `stretch` 的实例**（§9.4 的回归测试基线）：

| 节点 | `stretch` | 负轴个数 |
|---|---|---|
| `L-Ear` | `(−1.411, 0.932, 0.85)` | 1（奇） |
| `L-Ear2` | `(−1.520332, 1.293538, 0.1)` | 1（奇） |
| `R-UpperLeg` | `(−1, 1, 1.028986)` | 1（奇） |
| `R-Leg` | `(−1, 1, 1)` | 1（奇） |
| `R-Foot` | `(−1, 1, 1)` | 1（奇） |
| `R-BackUpperLeg` | `(−1, 1.264197, 1.049551)` | 1（奇） |
| `R-BackLeg` | `(−1, 1.159007, 1)` | 1（奇） |
| `R-BackFoot` | `(−1, 1, 1)` | 1（奇） |

**这 8 个形状全部需要翻转绕序。** 忘了这一步，狗的整个右半边（加左耳）会在背面剔除下整块消失——而静态检查、单元测试、乃至「窗口能打开」都不会报错。这正是 §9.4 必须写进回归测试的原因。

---

## 附录 C：JSON 语法摘要

非规范（normative）的紧凑语法，供实现者快速对照：

```
document   := { "nodes": [ node, ... ], "format"?: string, "lod"?: string }

node       := {
  "id":          string,
  "name":        string,
  "position"?:   vec3,
  "orientation"?:quat,
  "shape"?:      shape,
  "children"?:   [ node, ... ]
}

shape      := {
  "offset"?:       vec3,
  "stretch"?:      vec3,                       // 缺省 (1,1,1)，可为负
  "type"?:         "box" | "quad" | "none",
  "settings"?:     {
      "size"?:        vec3 | vec2,             // box: vec3；quad: vec2
      "normal"?:      "+X"|"-X"|"+Y"|"-Y"|"+Z"|"-Z",   // 仅 quad，缺省 "+Z"
      "isPiece"?:     bool,
      "isStaticBox"?: true
  },
  "textureLayout": { <facename>: uvface, ... },// 写入时必需，可为 {}
  "unwrapMode"?:   "custom",
  "visible"?:      bool,
  "doubleSided"?:  bool,
  "shadingMode"?:  "flat" | "standard" | "fullbright" | "reflective"
}

uvface     := {
  "offset":   vec2,                            // 贴图像素，左上角
  "mirror"?:  { "x": bool, "y": bool },
  "angle"?:   0 | 90 | 180 | 270,
  "transparent"?: bool,
  "lockUVs"?:     bool
}

facename   := "front" | "back" | "left" | "right" | "top" | "bottom"
vec2       := { "x": number, "y": number }
vec3       := { "x": number, "y": number, "z": number }
quat       := { "x": number, "y": number, "z": number, "w": number }   // 标量在后
```

---

## 附录 D：参考来源

**一级来源（格式的定义者）**

* [JannisX11/hytale-blockbench-plugin](https://github.com/JannisX11/hytale-blockbench-plugin) — Hypixel Studios 官方 Blockbench 插件（GPL-3.0）
  * `src/blockymodel.ts` —— `.blockymodel` 的 `compile()` / `parse()`，UV 映射、四元数、`--C<n>` 子 cube 命名的权威定义
  * `src/formats.ts` —— `block_size` = 64/32、`forward_direction: '+z'`、`animation_loop_wrapping`、`quaternion_interpolation`、`stretch_cubes`、`uv_rotation`、`integer_size`、`single_texture_default`、`texture_wrap_default: 'clamp'`
  * `src/validation.ts` —— `MAX_NODE_COUNT = 255` 及其计数规则
  * `src/element.ts` —— `shading_mode` 四值枚举与显示名、`double_sided`、`is_piece`、`transparent`、`uv_lock`、inflate 被禁用
  * `src/util.ts` —— `qualifiesAsMainShape` / `getMainShape` / `cubeIsQuad`（主形状与 quad 判定）
  * **`dist/about.md`**（随插件发行包分发的官方格式指南）—— 255 节点规则及其计数口径、`grid = 1 方块 = 64/32 像素`、*"UVs must always match the dimensions of their face"*、Position/Rotation 打 group 而 Scale/Visibility/UV Offset 只打 shape 的官方说明
* [Hytale 官方博客 — An Introduction to Making Models for Hytale](https://hytale.com/news/2025/12/an-introduction-to-making-models-for-hytale)（2025-12-22，美术总监 Thomas Frick）—— 唯二由 Hypixel 撰写的散文式指南。给出：只用 cube / quad、*"No spheres allowed!"*、贴图宽高必须是 32 的整数倍、64/32 密度、`stretch` 建议区间 0.7–1.3×、不用 PBR、着色模式需在编辑器外验证

**二级来源（交叉验证）**

* [hytale-tools/blockymodel-merger](https://github.com/hytale-tools/blockymodel-merger) — Go 实现（GPL-3.0）
  * `pkg/blockymodel/types.go` —— 结构体定义、`HasGeometry`
  * `pkg/blockymodel/io.go` —— `textureLayout` 强制写出的自定义 `MarshalJSON`
  * `pkg/render/scene.go` —— 骨骼变换累乘
  * `pkg/render/geometry.go` / `pkg/render/render.go` —— 面顶点表、UV 镜像/旋转/缩进、负 stretch 绕序
  * `pkg/export/glb.go` —— 顶点顺序、绕序翻转（其中的 `1/16` 缩放与 `1-v` 翻转不要照抄）
  * README —— 资产目录与贴图命名约定（`X.blockymodel` + `X_Texture.png`）、`-pose <file.blockyanim>`「把任意动画的第 0 帧当静态姿势」
* `Pets/Dog/Models/Model.blockymodel` —— Hytale 官方资产，本文所有实测数据的来源
* [Blockbench](https://github.com/JannisX11/blockbench) — 插件宿主引擎（`js/preview/preview_scenes.ts` 的 `getUVArray` 是角点轮换的来源）

**三级来源（第三方实现，仅供对照，不得作为规范依据）**

* [blockymodel-web](https://www.npmjs.com/package/blockymodel-web)（MIT）—— 公开的 `.blockymodel` TS 类型；与插件一致，但**多发明了 `unwrapMode: "auto"`**（插件只会写 `"custom"`）
* [blockymodel-texture](https://www.npmjs.com/package/blockymodel-texture) —— 公开记录 UV 算法；与 §8 一致，并解释了 V 轴翻转的三方约定差异

**官方但非格式相关**

* [docs.hytale.com — `server.core.asset.type.model.config.ModelAsset`](https://docs.hytale.com/api/com/hypixel/hytale/server/core/asset/type/model/config/ModelAsset) —— **服务端** `Assets/Server/Models/*.json` 的 Javadoc。它才是 `Scale`、`EyeHeight`、`CrouchOffset`、`AnimationSetMap`、`HitBox`、`Attachments`、`Light` 等字段的归宿；这些字段**不在** `.blockymodel` 里

**需要避开的来源**

* ⚠️ [hytale-docs.com — 3D Models](https://hytale-docs.com/docs/modding/art-assets/models) —— **印的是虚构的 Minecraft Bedrock 风格 schema**。见 [附录 E](#附录-e伪造的公开-schema-警示)
* ⚠️ 同站的 [Textures](https://hytale-docs.com/docs/modding/art-assets/textures) —— 掺入了无出处的说法（512×512 上限、`_emissive.png`、动画贴图 JSON、必须为 2 的幂），插件源码、官方博客与本文实测都无法佐证

> 完整的公开资料调研（含逐条引用、非存在页面的验证、以及每个来源的权威性分级）见 [`hytale-format-public-docs-survey.md`](./hytale-format-public-docs-survey.md)。

> **结论**：`.blockymodel` **没有**公开的正式规范。任何第三方实现的事实基准只能是上述一级来源的源码，加上用真实资产做的回归测试。

---

## 附录 E：伪造的公开 schema 警示

搜索引擎里「Hytale model format documentation」排名最高的页面是 **[hytale-docs.com/docs/modding/art-assets/models](https://hytale-docs.com/docs/modding/art-assets/models)**。它的「File Format → `.blockymodel` Structure」一节给出的 schema **是虚构的**：每一个特征字段都来自 Minecraft Bedrock / GeckoLib 系，而不是 Hytale。

（下表由逐字比对 `timiliris/Hytale-Docs@master:content/docs/en/modding/art-assets/models.md` 的原文与插件源码得出。）

| 该页声称 | 插件源码的真实情况 |
|---|---|
| 顶层 `format_version` + `model` 包装对象 | `{ nodes, format, lod }`，无 `format_version`、无包装 |
| `model.identifier`（`namespace:name`） | **没有**标识符字段；身份来自文件路径 / 服务端资产 id |
| `model.texture_width` / `texture_height` | **不存在**；模型文件从不存图集尺寸 |
| `model.bones[]`，含 `pivot` / `cubes` / `children` | `nodes[]`，含 `position`（相对原点）/ `orientation`（四元数）/ `shape` / `children` |
| cube 的 `origin` / `size` / `uv` / `inflate` / `mirror` | `shape.offset` / `shape.settings.size` / `shape.stretch` / `shape.textureLayout[面].{offset,mirror,angle}`。**没有 `inflate`**，也没有 cube 级 `mirror` 布尔 |
| 逐面 `uv: [u1,v1,u2,v2]` | 逐面 `{offset:{x,y}, mirror:{x,y}, angle}`——是**一个点 + 旋转**，不是一个矩形 |
| `visible_bounds_width/height/offset`、`display.hand.*` | **不存在** |
| `mods/<mod>/assets/models/{blocks,items,entities}/` | 游戏资产包内的 `Assets/{Characters,Cosmetics,Blocks,Items}/…` |
| *"1 pixel = 1/16 block"* | 密度是**道具 32 单位 / 角色 64 单位 = 1 方块**；与 1/16 无关 |

**给实现者的建议**：

1. **不要**把该页当作格式来源；它的字段名一个都不要抄。
2. 如果必须在文档里提到它，只能作为「网上有伪造 schema」的反面例子（就像本附录）。
3. 任何以「`format_version`」或「`model.identifier`」开头的 Hytale 模型 JSON，基本都是把它当成 Minecraft Bedrock 模型生成的——Hytale 游戏客户端**不会**接受。
4. 同站的贴图页同样不可信（512×512 上限、`_emissive.png`、动画贴图 JSON、必须为 2 的幂）——这些说法在插件源码里没有任何对应实现。
