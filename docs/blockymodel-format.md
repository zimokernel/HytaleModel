# Hytale Blockbench 资产格式规范 —— `.blockymodel` / `.blockyanim`

> 面向「用 Rust + wgpu 自己写渲染器 / 播放器」的完整参考。
> 本文所有结论都经过实测验证，来源见附录 B。

---

## 目录

1. [先搞清楚文件扩展名](#1-先搞清楚文件扩展名)
2. [资产工作流全景](#2-资产工作流全景)
3. [坐标系与单位](#3-坐标系与单位)
4. [`.blockymodel` 结构](#4-blockymodel-结构)
5. [骨骼变换：bind pose 的精确公式](#5-骨骼变换bind-pose-的精确公式)
6. [几何形状：box / quad / none](#6-几何形状box--quad--none)
7. [UV 展开算法（最关键的一节）](#7-uv-展开算法最关键的一节)
8. [`.blockyanim` 结构](#8-blockyanim-结构)
9. [动画求值语义](#9-动画求值语义)
10. [循环、边界与尾帧](#10-循环边界与尾帧)
11. [踩坑清单](#11-踩坑清单)
12. [Rust/wgpu 实现要点](#12-rustwgpu-实现要点)
13. [附录 A：Pets/Dog 资产实测数据](#附录-apetsdog-资产实测数据)
14. [附录 B：参考资料与本地参考实现](#附录-b参考资料与本地参考实现)

> 配套实现：`crates/blockymodel`（解析 + 骨骼 + 网格 + 动画采样）与 `crates/player`（wgpu 27 + winit 0.30 交互播放器）。
> 两者都对齐 `D:\work\ttc\Aether_terrain` 的版本与数学类型，见第 12 节。

> 📄 **正式规范已拆分**：本文现在定位为「实现笔记 / 踩坑记录」。
> 逐字段的规范性定义见 [`blockymodel-spec.md`](./blockymodel-spec.md)（`.blockymodel`）与 [`blockyanim-spec.md`](./blockyanim-spec.md)（`.blockyanim`）；
> 两者包含 RFC 2119 措辞的强制等级、完整的采样伪代码、写入者清单与来源考证。

---

## 1. 先搞清楚文件扩展名

你在需求里写的是 `.blockmodel` / `.blockanim`，**实际格式的扩展名是**：

| 你以为 | 实际 | 内容 |
|---|---|---|
| `.blockmodel` | **`.blockymodel`** | 模型（骨架 + 形状 + UV 布局），纯 JSON |
| `.blockanim` | **`.blockyanim`** | 动画（关键帧轨道），纯 JSON |

两者都是**无压缩的 UTF-8 JSON**，没有二进制包装、没有魔数、不需要解压。可以直接 `serde_json` 解析。

`Pets/Dog` 的实际布局（这就是 Hytale 官方的标准资产排布）：

```
Pets/Dog/
├── Models/
│   ├── Model.blockymodel      # 98 KB / 2950 行 / 31 个 node
│   └── Texture.png            # 256 x 128 RGBA —— 整模型共用一张图集
└── Animations/
    ├── Idle.blockyanim        # 15 个动作
    ├── Walk.blockyanim
    ├── Attacks/Bite.blockyanim
    └── Idles/Sit.blockyanim
```

约定俗成的规则（不是格式强制的，但 Blockbench 插件按这个找）：

* 贴图命名为 `Texture.png`，或者 `<模型名>_Texture.png`，或者放在 `<模型名>_Textures/` 子目录里。
* `Animations/` 下的**一级子目录**（`Attacks/`、`Idles/`）只是分类，文件名才是动作名。

---

## 2. 资产工作流全景

Hytale 的美术管线是 **Blockbench 官方插件** `JannisX11/hytale-blockbench-plugin`（Hypixel Studios 自己维护，GPL 协议）。理解这点非常重要，因为：

> **`.blockymodel` 本质上就是 Blockbench 内部数据结构的序列化。**
> 你不必猜格式，直接读插件的 `compile()` / `parse()` 就能知道每个字段的确切含义。

数据流：

```
Blockbench (hytale_character / hytale_prop 格式)
   │  compile()                       parse()
   │──────────────────────────────▶  .blockymodel  ──────────────────────────────▶ Blockbench
   │  compileAnimationFile()          parseAnimationFile()
   │──────────────────────────────▶  .blockyanim
   ▼
Hytale 游戏客户端  /  blockymodel-merger (Go)  /  你的 Rust 渲染器
```

关键点：**Blockbench 的「模型空间」和文件里的数字是完全一致的**，没有额外缩放、没有轴交换。文件里 `position: {x: 0, y: 35, z: -20}` 就是 Blockbench 里骨骼原点在 (0, 35, -20)。这一点让反向工程变得简单：唯一需要逆向的是「Blockbench 怎么把这些数字变成三角形」。

---

## 3. 坐标系与单位

### 3.1 轴与手性

| 项 | 值 |
|---|---|
| 手性 | **右手系**（right-handed） |
| 上方向 | **+Y** |
| 前方向 | **+Z**（`Format.forward_direction = '+z'`） |
| 面朝向 | 逆时针（CCW）为正面 → wgpu 用 `FrontFace::Ccw` + `CullMode::Back` |
| 四元数 | `(x, y, z, w)`，**标量在后**，与 `glam::Quat` / glTF 一致 |

Blockbench 里「前」是 `south`（+Z），「后」是 `north`（−Z），「左」是 `west`（−X），「右」是 `east`（+X）。文件里的 `textureLayout` 用的却是 Hytale 自己的命名，映射关系是：

| `.blockymodel` 里的名字 | Blockbench 名字 | 法线 |
|---|---|---|
| `front` | south | **+Z** |
| `back` | north | **−Z** |
| `left` | west | **−X** |
| `right` | east | **+X** |
| `top` | up | **+Y** |
| `bottom` | down | **−Y** |

### 3.2 单位（最容易搞错的地方）

文件里的数字单位就是 Blockbench 的「像素」。官方的 Hytale 格式定义（`src/formats.ts`）声明：

```ts
new ModelFormat('hytale_character', { block_size: 64, ... })
new ModelFormat('hytale_prop',      { block_size: 32, ... })
```

`block_size` 表示 **一个世界方块等于多少个模型单位**。所以：

* **角色模型：1 方块 = 64 单位**
* **道具模型：1 方块 = 32 单位**

用 `Pets/Dog` 验证：整个模型包围盒是 `43.90 × 88.54 × 94.91` 单位，除以 64 得到 `0.69 × 1.38 × 1.48` 方块 —— 一只大约 1.4 格高、1.5 格长的狗，完全合理。如果按 1/16 算就是 2.7 格高、5.9 格长的巨兽，明显不对。

> ⚠️ **陷阱**：`hytale-tools/blockymodel-merger` 的 GLB 导出用的是 `scale = 1.0/16.0`（`pkg/export/glb.go`）。那是为了对齐 Minecraft 系模型的预览惯例写的常量，对 Hytale 角色相当于放大了 4 倍。**不要照抄这个数**。
>
> 好消息：如果你做的是自动取景的查看器，这个常数根本不重要 —— 直接按包围盒适配相机即可。本文的播放器就是这么做的，只在文档里记录真实比例。

---

## 4. `.blockymodel` 结构

### 4.1 顶层对象

```jsonc
{
  "nodes": [ /* 根节点数组，可以有多个 */ ],
  "lod": "auto",              // 可选
  "format": "character"       // 可选，"character" | "prop"
}
```

* `nodes`：**根节点数组**。Dog 只有 1 个根（`Pelvis`），但格式允许多个。
* `lod`：只见过 `"auto"`。渲染器可以完全忽略。
* `format`：Dog 里没有这个字段。只有 `"prop"` 时有意义（决定 Blockbench 用哪套 block_size）。渲染器可以忽略。

### 4.2 Node（骨骼）

```jsonc
{
  "id": "12",                       // 字符串！不是数字。用于合并时的唯一标识
  "name": "Pelvis",                 // 骨骼名，动画按这个名字绑定
  "position":  { "x": 0, "y": 35, "z": -20 },          // 相对父骨骼的平移
  "orientation": { "x": -0.051265, "y": 0, "z": 0, "w": 0.998685 },  // 旋转四元数
  "shape": { /* 见下 */ },
  "children": [ /* 同构的 Node 数组，可为空数组 */ ]
}
```

要点：

* **`id` 是字符串**（`"12"`），别用 `u32` 反序列化。
* `position` 和 `orientation` 在 Go 参考实现里都是 `Option`，**可能整个缺失**。缺失时按 `position = (0,0,0)`、`orientation = identity` 处理。
* `children` 可能缺失，也可能存在但是空数组 `[]`。
* **`position` 不是世界坐标，是相对父骨骼原点的偏移**（见第 5 节）。
* 官方限制：**模型最多 255 个节点**（所以索引可以用 `u8`/`u16`）。

### 4.3 Shape（形状）

```jsonc
"shape": {
  "offset":  { "x": 0, "y": 0, "z": 0 },     // 形状中心相对骨骼原点的偏移
  "stretch": { "x": 1, "y": 1.113824, "z": 1 },  // 缩放倍数（不是绝对值！）
  "type": "box",                              // "box" | "quad" | "none"
  "settings": { "size": { "x": 27, "y": 19, "z": 22 } },
  "textureLayout": { /* 见 4.4 */ },
  "unwrapMode": "custom",                     // 永远是 "custom"
  "visible": true,                            // 默认可见性
  "doubleSided": false,                       // 是否渲染背面
  "shadingMode": "flat"                       // "flat" | "standard" | "fullbright" | "reflective"
}
```

字段语义：

| 字段 | 语义 | 缺省 |
|---|---|---|
| `offset` | shape 几何中心相对**骨骼原点**的平移。**同时会传递给子骨骼**（见第 5 节），这是 Hytale 骨骼系统的核心机制 | `(0,0,0)` |
| `stretch` | **缩放倍数**。最终尺寸 = `settings.size * stretch`。默认是 `{1,1,1}`。**允许为负**（负值意味着镜像翻转几何体，会影响绕序！） | `(1,1,1)` |
| `type` | `box` / `quad` / `none`。`none` 表示这个节点没有几何体，只是一个骨骼/挂点 | — |
| `settings.size` | box 是 `{x,y,z}`；**quad 只有 `{x,y}`**；`none` 没有 `size` | — |
| `settings.normal` | **仅 quad**：`"+X" \| "-X" \| "+Y" \| "-Y" \| "+Z" \| "-Z"`，默认 `"+Z"` | `"+Z"` |
| `settings.isPiece` | `true` 表示这是个「附件挂点」，需要挂到主模型同名骨骼上 | `false` |
| `settings.isStaticBox` | `true` 表示这个 cube 直接代表了它的 group（没有独立 group），**只在需要重建编辑器层级时有意义**，渲染器可忽略 | — |
| `visible` | 初始可见性 | `true` |
| `doubleSided` | 双面渲染 | `false` |
| `shadingMode` | 着色模式。`flat` 是纯色/无光照；`standard` 受光；`fullbright` 全亮；`reflective` 反射 | `"flat"` |

> **重要**：即使 `type == "none"`，`shape.offset` **仍然要参与子骨骼的变换**。参考实现（`pkg/render/scene.go:71-79`）对任何 shape 都读取 offset 传给子节点，只有「是否生成网格」这一步才检查 type。

### 4.4 `textureLayout`

每个面一个条目。**面按需出现** —— 一个 box 可以只定义 3 个面，其余面不渲染。

```jsonc
"textureLayout": {
  "top": {
    "offset": { "x": 83, "y": 47 },          // 贴图左上角，单位=像素
    "mirror": { "x": false, "y": false },    // 是否反向采样
    "angle":  0                              // 0 | 90 | 180 | 270（只见过 0 和 180）
  },
  "right": { "offset": {"x":165,"y":1}, "mirror": {"x":true,"y":false}, "angle": 0 }
}
```

* `offset` 是 **UV 矩形的左上角，单位是贴图像素**（不是 0..1 归一化值）。
* `mirror.x = true` 时 U 轴反向（矩形向左延伸，即 `u1 = offset.x - width`）。
* `angle` 是 UV 的旋转。
* 还可能出现的可选字段（Blockbench 插件写入，参考实现忽略）：
  * `transparent: true` —— 该面强制透明混合
  * `lockUVs: true` —— 禁止自动重排 UV

**坑**：Hytale 官方资产在**每个** shape 上都写 `"textureLayout": {}`，即使没有 UV。游戏客户端在缺失这个 key 时会加载失败 —— 所以如果你要**写** `.blockymodel`，必须显式输出空对象（Go 参考实现在 `io.go` 里专门为此重写了 `MarshalJSON`）。读的时候则无所谓。

---

## 5. 骨骼变换：bind pose 的精确公式

这是整个格式的骨架。Blockbench 的层级是「group（骨骼）→ cube（形状）」两层，`.blockymodel` 把它压成了一层：**一个 Node 同时承担骨骼和形状两种角色**。

正确的递归（与 `pkg/render/scene.go` 和 `pkg/export/glb.go` 完全一致）：

```
bone_world(N)   = bone_world(P) · T(P.shape.offset + N.position) · R(N.orientation)
shape_world(N)  = bone_world(N) · T(N.shape.offset) · S(N.shape.stretch)
```

其中：

* `T` = 平移矩阵，`R` = 由四元数 `(x,y,z,w)` 构造的旋转矩阵，`S` = 分量缩放矩阵。
* **根节点**的 `P.shape.offset` 和 `P` 整体取单位变换，即 `bone_world(root) = T(root.position) · R(root.orientation)`。
* 传给子节点的父偏移是 **`P.shape.offset` 的原始值（未乘 stretch）**。

### 5.1 三条容易写错的规则

1. **父节点的 `shape.offset` 要加进子节点的 position 里**
   这不是笔误。Blockbench 的 group 有独立的 `origin`（枢轴点），而 cube 的中心是 `origin + offset + size/2`。换算下来，子 group 的 origin = `父origin + 父offset + position`。所以 `offset` 实际上是一个「枢轴偏移」，会影响整条子链。

2. **父节点的 `stretch` 不影响子节点**
   `S(stretch)` 只作用在该节点自己的网格上。

3. **旋转在平移之后**
   矩阵顺序必须是 `T(...) · R(...)`，不能写成 `R · T`。否则所有骨骼的位置都会跟着旋转飞掉。

### 5.2 网格顶点的生成

对 box：

```
half = settings.size / 2
vertex_local = shape.offset + stretch * (±half.x, ±half.y, ±half.z)
vertex_world = bone_world(N) · vertex_local
```

注意 `stretch` 直接乘在顶点上（等价于 `T(offset)·S(stretch)` 作用于中心在原点的立方体），而 **UV 尺寸用的是未乘 stretch 的 `size`** —— 这就是「拉伸几何但不拉伸 UV 采样窗口」，效果是贴图被拉长。这是 Blockbench 的既定行为。

`stretch` 分量为负时几何体镜像，**三角形绕序会翻转**。参考实现的做法（`pkg/export/glb.go:536`）：

```go
oddFlips := (flipX != flipY) != flipZ   // 负数轴个数为奇数
if oddFlips { /* 用反向索引 */ } else { /* 正常索引 */ }
```

---

## 6. 几何形状：box / quad / none

### 6.1 `box`

6 个面，每个面只有出现在 `textureLayout` 里才生成。每个面 4 个顶点（**顺序固定为从外部看：TL, TR, BL, BR**），2 个三角形。

六个面的顶点定义（`h` = half extent，即 `size*|stretch|/2`，`o` = `shape.offset`）—— 这张表直接抄自 `pkg/export/glb.go:447-495`，是**经过验证的权威版本**：

| 面 | 法线 | TL | TR | BL | BR |
|---|---|---|---|---|---|
| `right` (+X) | (1,0,0) | (+hx, +hy, +hz) | (+hx, +hy, −hz) | (+hx, −hy, +hz) | (+hx, −hy, −hz) |
| `left` (−X) | (−1,0,0) | (−hx, +hy, −hz) | (−hx, +hy, +hz) | (−hx, −hy, −hz) | (−hx, −hy, +hz) |
| `top` (+Y) | (0,1,0) | (−hx, +hy, −hz) | (+hx, +hy, −hz) | (−hx, +hy, +hz) | (+hx, +hy, +hz) |
| `bottom` (−Y) | (0,−1,0) | (−hx, −hy, +hz) | (+hx, −hy, +hz) | (−hx, −hy, −hz) | (+hx, −hy, −hz) |
| `front` (+Z) | (0,0,1) | (−hx, +hy, +hz) | (+hx, +hy, +hz) | (−hx, −hy, +hz) | (+hx, −hy, +hz) |
| `back` (−Z) | (0,0,−1) | (+hx, +hy, −hz) | (−hx, +hy, −hz) | (+hx, −hy, −hz) | (−hx, −hy, −hz) |

（每个顶点还要加上 `o`。）

**索引（法线朝向外的 CCW）**：`(0,2,1)` 和 `(2,3,1)`。

可以验证 `front` 面：`BL−TL = (0,−2hy,0)`，`TR−TL = (2hx,0,0)`，叉积 = `(0,0,4·hx·hy)` → +Z ✓ 正面朝外。

### 6.2 `quad`

一个四边形，几何上是一个「压扁的 box」：把法线轴方向的厚度设为 0。

* `settings.size` 只有 `{x, y}`，语义是**面内两个轴的尺寸**，顺序是：
  * 法线 `±Z`：x = 沿 **X** 轴的宽度，y = 沿 **Y** 轴的高度
  * 法线 `±X`：x = 沿 **Z** 轴的宽度，y = 沿 **Y** 轴的高度
  * 法线 `±Y`：x = 沿 **X** 轴的宽度，y = 沿 **Z** 轴的高度
* 顶点表直接复用上面对应 box 面的 TL/TR/BL/BR，把法线轴分量当作 0 即可（`h_法线轴 = 0`）。
* **quad 永远只查 `textureLayout["front"]`**。Blockbench 导出 quad 时只会写 `front` 这一个键；参考实现在找不到对应面时会回退到 `front`（`pkg/render/geometry.go:162-166`）。
* UV 尺寸 = `(settings.size.x, settings.size.y)`。

Dog 里 5 个 quad：`R-Eye`、`L-Eye`、`L-Ear2`、`R-Ear2`、`TongueDog2`。

### 6.3 `none`

没有几何体。仍然参与层级变换（它的 `offset` 会传递给子节点）。出现在：
* 纯骨骼节点
* 附件挂点（`settings.isPiece: true`）

### 6.4 双面（`doubleSided`）

`doubleSided: true` 时不剔除背面。两种实现方式：

* **简单做法**：`CullMode::None`，并在片元着色器里对背面翻转法线（`@builtin(front_facing)`）。推荐。
* **参考实现做法**：额外生成一份反向绕序的几何体（`pkg/render/geometry.go:171-185`）。多一倍顶点，不推荐。

Dog 里所有 quad 都是 `doubleSided: true`，所有 box 都是 `false`。

---

## 7. UV 展开算法（最关键的一节）

这是整个格式里最容易写错的部分。下面是 **Blockbench 官方插件 `parse()`（`src/blockymodel.ts:686-763`）的逐行等价实现**，也就是文件格式的权威定义。

### 7.1 第一步：确定 UV 矩形的像素尺寸

用的是 **未乘 stretch 的 `settings.size`**：

| 面 | uvWidth | uvHeight |
|---|---|---|
| `front` / `back` | `size.x` | `size.y` |
| `left` / `right` | `size.z` | `size.y` |
| `top` / `bottom` | `size.x` | `size.z` |
| quad（任意法线） | `size.x` | `size.y` |

### 7.2 第二步：算出 `[u0, v0, u1, v1]`

设 `ox, oy = textureLayout.offset`，`w, h = uvWidth, uvHeight`，
`mx = mirror.x ? -1 : +1`，`my = mirror.y ? -1 : +1`：

```
angle == 0:                     // 什么都不动
    [u0,v0,u1,v1] = [ ox,            oy,            ox + w*mx,     oy + h*my ]

angle == 90:                    // 交换 w/h 与 mx/my，再让 mx 取反
    swap(w,h); swap(mx,my); mx = -mx
    [u0,v0,u1,v1] = [ ox,            oy + h*my,     ox + w*mx,     oy ]

angle == 180:                   // mx、my 都取反
    mx = -mx; my = -my
    [u0,v0,u1,v1] = [ ox + w*mx,     oy + h*my,     ox,            oy ]

angle == 270:                   // 交换 w/h 与 mx/my，再让 my 取反
    swap(w,h); swap(mx,my); my = -my
    [u0,v0,u1,v1] = [ ox + w*mx,     oy,            ox,            oy + h*my ]
```

（`90` / `270` 分支里的 `w,h,mx,my` 已经是交换后的值。）

### 7.3 第三步：把矩形映射到 4 个角

**角点顺序与第 6.1 节的顶点顺序一一对应**：

```
corner[0] = TL = (u0, v0)
corner[1] = TR = (u1, v0)
corner[2] = BL = (u0, v1)
corner[3] = BR = (u1, v1)
```

然后按 `angle / 90` 次应用同一个轮换：

```
tmp = a[0]; a[0] = a[2]; a[2] = a[3]; a[3] = a[1]; a[1] = tmp;
// [TL, TR, BL, BR] → [BL, TL, BR, TR]
```

### 7.4 第四步：归一化

```
u_pixel / texture_width,  v_pixel / texture_height
```

**在 wgpu 里不需要翻转 V。** 说明：

* `.blockymodel` 的 `offset` 用的是「图片坐标系」（原点左上，Y 向下）。
* wgpu / WebGPU 的纹理坐标原点**也是左上**，和图片行序一致。
* 所以直接 `v = py / height` 即可。

> 参考实现的 GLB 导出里有一对看起来矛盾的 `1-v` 翻转（`glb.go` 的 step 2 和 step 3），它们**互相抵消**了 —— 那是为了绕开 glTF 生态的历史包袱。你写 wgpu 时不要照抄。

### 7.5 第五步：防止接缝渗色

相邻 UV 岛屿在 mipmap / 双线性采样下会互相渗色。两个缓解手段：

1. **把 UV 向内缩进约 1/8 像素**（参考实现的做法，`glb.go:626-641`）：
   ```rust
   let inset_u = (1.0 / 8.0) / tex_w;
   let inset_v = (1.0 / 8.0) / tex_h;
   // u0 < u1 时 u0 += inset_u, u1 -= inset_u；否则反向
   ```
2. **用 `Nearest` 采样**，或强制 `LodMinClamp`。

Hytale 的贴图是**像素艺术**（Dog 的 `Texture.png` 是 256×128），实际渲染**应该用 `Nearest` 采样**，渗色问题自然消失。

### 7.6 `shapeUvOffset` 动画通道的符号

如果实现 `shapeUvOffset`（滚动贴图 / 眨眼 / 电子屏）：

* 该通道的值**加到 UV 矩形上**（`uv[0] += dx; uv[2] += dx; uv[1] += dy; uv[3] += dy`）。
* Blockbench 在**导入**时做了 `y = -file.y`（导出时再取反），所以 **文件里的 `delta.y` 与「实际加到图片坐标上的偏移」符号相反**：
  ```
  实际像素偏移 = ( delta.x, -delta.y )
  ```

---

## 8. `.blockyanim` 结构

### 8.1 顶层

```jsonc
{
  "formatVersion": 1,
  "duration": 80,               // ★ 单位是「帧」，不是秒。帧率固定 60 fps
  "holdLastKeyframe": false,    // false = 循环播放；true = 停在最后一帧
  "nodeAnimations": {
    "Pelvis": {
      "position":       [ /* 关键帧 */ ],
      "orientation":    [ /* 关键帧 */ ],
      "shapeStretch":   [ /* 关键帧 */ ],
      "shapeVisible":   [ /* 关键帧 */ ],
      "shapeUvOffset":  [ /* 关键帧 */ ]
    }
  }
}
```

* `duration` 的范围在 Dog 里是 20 ~ 80（帧），即 0.33 ~ 1.33 秒。
* `nodeAnimations` 的 key 是**骨骼名字**，不是 id。
* **五个通道都可能缺失**，也常常是空数组 `[]`。
* 通道名与 Blockbench 通道的对应：

| `.blockyanim` | Blockbench | 语义 |
|---|---|---|
| `position` | position | 平移**增量** |
| `orientation` | rotation | 旋转**增量**（四元数） |
| `shapeStretch` | scale | 缩放**倍数** |
| `shapeVisible` | visibility | 可见性（布尔） |
| `shapeUvOffset` | uv_offset | UV 像素偏移 |

**Dog 的 15 个动作实际只用了 `orientation`(1000 帧)、`position`(118)、`shapeStretch`(50)**；
`shapeVisible` 和 `shapeUvOffset` 这两个**键存在于每个骨骼上，但数组总是空的**（官方插件的输出习惯是「有数据就补齐所有通道键」）。
不过实现时仍要支持它们 —— 别的资产（滚动贴图、眨眼、`Death` 之外的可见性切换）会用，见 9.2.3。

### 8.2 关键帧

```jsonc
{
  "time": 40,
  "delta": { "x": 0, "y": 0, "z": 0, "w": 1 },
  "interpolationType": "smooth"        // 可选："smooth" | "linear"，缺失视为 linear
}
```

* `time` 单位是**帧**（60 fps）。`time / 60.0` 秒。
* `delta` 的具体形状取决于通道（见 8.3）。
* `interpolationType` 在格式上是**可选的**（缺失视为 `"linear"`），所以 serde 结构体上必须带 `#[serde(default)]`。

> **实测更正**：本文早期版本称「`interpolationType` 只出现在部分关键帧上」，这是**错的**。
> 对整个 `Pets/Dog/Animations/` 做全量统计：**1168 个关键帧中 0 个缺失该字段，取值 100% 是 `"smooth"`**。
> 把它当可选处理仍然正确（Blockbench 可以写出 `"linear"`，官方插件也只写 `"smooth"` / `"linear"` 两种），但不要拿 Dog 当「字段缺失」的例子。

### 8.3 `delta` 的四种形状

```jsonc
"position":      { "x": -0.29, "y": 8.90, "z": -7.13 }              // Vec3
"orientation":   { "x": 0.04, "y": 0.0, "z": 0.0, "w": 0.999 }      // Quat
"shapeStretch":  { "x": 1.078, "y": 1.05, "z": 1.018 }              // Vec3，是倍数
"shapeVisible":  true                                               // ★ 裸 bool，不是对象
"shapeUvOffset": { "x": 0, "y": 0 }                                 // Vec2，整数像素
```

> **`shapeVisible` 的 `delta` 是裸布尔值**，不是 `{x,y,z}`。用 `serde` 时这个字段必须用 `serde_json::Value` 或 `#[serde(untagged)]` 枚举来解析，否则整个文件解析失败。

### 8.4 骨骼名不一定存在于模型里

Dog 的动画里出现了 `Collar`、`R-Ear2`、`TailDog` 这三个 **`Model.blockymodel` 里根本没有的骨骼名**（模型共 30 个名字，动画共引用 31 个）。

**结论：按名字绑定，找不到就静默跳过，绝对不能报错。** 这些动画是跨资产变体复用的。

反过来，**同名节点可能有多份**（合并附件后 `Cape1..Cape3` 之类的物理骨骼会有克隆）。参考实现的做法是**所有同名节点同时驱动**（`pkg/anim/anim.go:69-96`）。所以绑定表应该是 `HashMap<String, Vec<NodeIndex>>`。

---

## 9. 动画求值语义

### 9.1 时间 → 姿势

```
t_seconds = frame / 60.0
```

对每个通道、每条轨道，独立求值，然后**以 bind pose 为基准合成**：

| 通道 | 合成公式 | 说明 |
|---|---|---|
| `position` | `pos_final = node.position + delta` | **向量加法** |
| `orientation` | `q_final = node.orientation ⊗ q_delta` | **四元数乘法，bind 在左** |
| `shapeStretch` | `stretch_final = shape.stretch * delta` | **逐分量乘法**（不是加法！） |
| `shapeVisible` | `visible = delta` | 绝对值，非增量。**不做插值**，见 9.2.3 |
| `shapeUvOffset` | `uv_offset_px = (delta.x, -delta.y)` | 加到 UV 矩形上。**不做插值**，见 9.2.3 |

四元数乘法的顺序非常重要。`bind ⊗ delta` 表示「delta 是在骨骼自己的局部坐标系里施加的旋转」，这与 Blockbench 的 `bone.quaternion.multiply(q_delta)`（`this = this * q`）一致。

参考实现（Go）里的写法：

```go
q := mul(bind, delta)   // mul(a,b) = a ⊗ b
```

### 9.2 插值：`linear` vs `smooth`

先明确一个重要前提：Hytale 格式声明了 `quaternion_interpolation: true`（`src/formats.ts:39`）。这**改变了旋转通道的插值路径**。以下是从 Blockbench 引擎（`js/animations/timeline_animators.js:426-590`）逐行推导的结果。

设当前时间 `t` 落在相邻两个关键帧 `A`（前）和 `B`（后）之间，
`alpha = (t - A.time) / (B.time - A.time)` ∈ [0, 1]。

#### 9.2.1 旋转（`orientation`）—— 永远是 SLERP

因为 `use_quaternions = true`，旋转通道**总是走球面插值**：

```
if A.interpolation == "smooth" && B.interpolation == "smooth":
    alpha = weightedCubicBezier(alpha)      // 缓动
q = slerp(A.quat, B.quat, alpha)
```

**注意：旋转不使用 Catmull-Rom，也不使用任何邻居关键帧。** 只有这个缓动函数：

```rust
/// 加权三次贝塞尔缓动（Blockbench 插件 `src/animations.ts:137-157`）
fn weighted_cubic_bezier(t: f32) -> f32 {
    const P: [f32; 4] = [0.0, 0.05, 0.95, 1.0];
    const W: [f32; 4] = [2.0, 1.0, 2.0, 1.0];
    let mt = 1.0 - t;
    let b = [mt*mt*mt, 3.0*mt*mt*t, 3.0*mt*t*t, t*t*t];
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..4 { num += b[i]*W[i]*P[i]; den += b[i]*W[i]; }
    num / den
}
```

这是 `smoothstep` 的一个近似但不完全相同的版本 —— 用它，不要用 `t*t*(3-2t)`，否则会看到细微差别。

#### 9.2.2 走样条的通道（`position` / `shapeStretch`）

```
if A.interpolation == "linear" && (B.interpolation == "linear" || B.interpolation == "step"):
    value = lerp(A.value, B.value, alpha)          // 纯线性
else if A.interpolation == "smooth" || B.interpolation == "smooth":
    value = catmull_rom(P0, P1, P2, P3, alpha)     // 使用邻居关键帧
```

Catmull-Rom 的邻居选择（**注意循环包裹**）：

```
P1 = A
P2 = B
P0 = A 的前一个关键帧；如果没有：
       - 如果动画是循环且轨道上 >= 3 个关键帧 → 取倒数第二个关键帧
       - 否则 → P0 = P1
P3 = B 的后一个关键帧；如果没有：
       - 如果动画是循环且轨道上 >= 3 个关键帧 → 取第二个关键帧
       - 否则 → P3 = P2
```

插值核是 **THREE.js `SplineCurve` 的均匀 Catmull-Rom**（不是 centripetal，也不是按时间参数化的）：

```rust
fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
         + (-p0 + p2) * t
         + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
         + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}
```

> **反直觉但必须照做**：Blockbench 的 `SplineCurve` 在**索引空间**里参数化，而不是时间空间。也就是说，关键帧之间「时间间隔不均匀」并不会让样条在时间上被拉伸。上面的 `alpha` 就是段内归一化参数，直接喂给 `t` 即可 —— 这正是 Blockbench 的行为（`getCatmullromLerp`，`keyframe.js:227-239`）。
>
> 另一条同样反直觉的细节：缺邻居时 THREE.js 是**复制端点**（`P0 = P1` / `P3 = P2`）而不是线性外推，所以只有 2 个关键帧的轨道**不是**直线。`catmull_rom(1, 1, 3, 3, 0.25) = 1.40625`，线性值才是 `1.5`。

#### 9.2.3 不插值的通道（`shapeVisible` / `shapeUvOffset`）

这两个通道**完全不走 `interpolate()`**，而是各自有专门的显示回调：

```
value(t) = 满足 time <= t 的关键帧中 time 最大者的 delta
如果不存在这样的关键帧：
    shapeVisible  → 该通道「无值」→ 回退到模型里这个 shape 的 visible 标志
    shapeUvOffset → 偏移视为 (0, 0)
```

证据：

* `src/animations.ts` 的 `displayVisibility` / `displayUVOffset` 都只做「找最后一个 `time <= Timeline.time` 的关键帧」；
* Blockbench 的 `displayFrame()` 只对 `rotation` / `position` / `scale` 调用 `interpolate()`，剩下两个通道靠 `channels[channel].displayFrame` 回调（`timeline_animators.js:596-604`）。

> **实现更正**：本文早期版本把 `shapeUvOffset` 列进了「Catmull-Rom 插值」那一组，这是**错的** —— 它是抽帧（hold）语义，滚动贴图/眨眼不会平滑过渡。`shapeVisible` 也同理（早期版本说「取 A 的值」，只对了一半：正确的规则是「取最后一个 ≤ t 的关键帧」，而且在没有任何 ≤ t 的关键帧时回退到 shape 自身的 `visible`，而不是取轨道第一个关键帧）。

### 9.3 边界情况：单关键帧 / 时间在范围外

| 情况 | 行为 |
|---|---|
| 轨道只有 1 个关键帧 | 整个动画期间恒定为该值 |
| `t` 早于第一个关键帧 | 用第一个关键帧的值 |
| `t` 晚于最后一个关键帧，且 `holdLastKeyframe == true` | 用最后一个关键帧的值（钳制） |
| `t` 晚于最后一个关键帧，且 `holdLastKeyframe == false` | **回绕**，见第 10 节 |
| `t` 恰好等于某个关键帧时间 | epsilon 容差 `1/1200` 秒 ≈ 0.05 帧，直接用该关键帧 |

---

## 10. 循环、边界与尾帧

Hytale 格式声明了 `animation_loop_wrapping: true`（`src/formats.ts:38`），这带来了一个容易被忽略的行为：

**当动画是循环模式（`holdLastKeyframe == false`）时，最后一个关键帧到 `duration` 之间的这段时间不是「保持最后一帧」，而是「插值回第一个关键帧」。**

具体机制（`timeline_animators.js:466-476`）：

```
if loop_wrapping && animation.loop == 'loop' && track.keyframes.len >= 2 {
    if no keyframe <= t:  before = 最后一个关键帧;  before.time -= duration_frames
    if no keyframe >  t:  after  = 第一个关键帧;    after.time  += duration_frames
}
```

用 `Sleep.blockyanim` 验证：`duration = 80`，关键帧最大时间是 60。
所以在 `t ∈ [60, 80]` 区间，`after` 回绕到第一个关键帧（t=0），`after.time` 变成 80。于是 `alpha` 从 0 平滑走到 1，姿势从「最后一帧」渐变回「第一帧」—— 循环无缝衔接。

相反的极端：`Death.blockyanim` 的 `holdLastKeyframe = true`，`duration = 40` 但最后一个关键帧在 30。这时 `t ∈ [30, 40]` 就是**停在死亡姿势**。

**播放器实现清单：**

```rust
let t = (elapsed_seconds * speed) % (duration / 60.0);   // 循环
// 或者 holdLastKeyframe: t = elapsed.min(duration/60.0)
```

然后在采样时按上面的规则选 `before`/`after`，并附带一个 `loop_wrap: bool` 参数控制是否回绕。

---

## 11. 踩坑清单

按「会浪费你多少时间」排序：

1. **`id` 是字符串**，不是数字。
2. **`position` 是相对父节点的**，且要再加上**父节点的 `shape.offset`**。忘了加 offset，模型会整体塌陷成一团。
3. **`shape.stretch` 是倍数不是绝对值**，默认 `(1,1,1)` 而不是 `(0,0,0)`。
4. **`shapeVisible` 的 `delta` 是裸 bool**，会让朴素的 serde 结构体解析整个文件失败。
5. **动画可能引用不存在的骨骼名**（Dog 里的 `Collar`、`TailDog`），必须静默忽略。
6. **UV 的 `offset` 是像素，不是归一化值**；且 `mirror.x` 意味着矩形从 offset **向左**延伸。
7. **`shapeStretch` 动画是乘法合成**：`bind_stretch * delta`。
8. **四元数合成顺序是 `bind ⊗ delta`**，反过来会让所有旋转都跑到错误的轴上。
9. **三条插值路径**：旋转通道用 SLERP + 加权贝塞尔缓动；平移/缩放通道用 Catmull-Rom；**`shapeVisible` / `shapeUvOffset` 完全不插值**（取最后一个 `time <= t` 的关键帧）。把它们混成一类会得到错误的姿势。
10. **`duration` 是帧不是秒**，恒定 60 fps。
11. **循环动画的尾段会回绕到第一个关键帧**，不是简单钳制。
12. **quad 只查 `textureLayout["front"]`**，而且 size 只有两个分量。
13. **负的 `stretch` 会翻转绕序**，背面剔除会剔错面。
14. **写出模型时 `textureLayout` 必须存在**（哪怕是 `{}`），否则游戏客户端拒绝加载。
15. **`unwrapMode` 恒为 `"custom"`**，没有别的取值，可以直接忽略。
16. **WGSL 的 uniform 结构体里不要放尾部 `vec2`**。uniform 地址空间要求成员偏移是 16 的倍数，`naga` 会静默补齐，导致 WGSL 侧结构体和 Rust 侧大小不一致、数组步长错位、模型碎成一堆乱块。详见 §12.7。
17. **必须真的把窗口跑起来验证**。着色器布局错误、绑定组不匹配、绕序反了 —— `cargo check` 和单元测试**一个都发现不了**。

---

## 12. Rust/wgpu 实现要点

本节描述本仓库 `crates/` 下那套可运行实现的结构。它不是凭空设计的范式，而是**照着 `D:\work\ttc\Aether_terrain` 的 `vrage_render` / `vrage_anim` 写的**，目的是之后能整块搬过去。

### 12.1 版本与依赖（与 Aether_terrain 对齐）

| 项 | 取值 | 与目标项目一致？ |
|---|---|---|
| Rust 工具链 | `nightly-2026-06-13`（`rust-toolchain`） | ✅ 完全相同 |
| edition | `2024` | ✅ |
| `wgpu` | `27` | ✅（目标项目由 `yakui-wgpu` 锁到 27.0.1） |
| `winit` | `0.30.12`（`features = ["serde"]`） | ✅ |
| 数学库 | `vek` `0.17.1`（`features = ["serde", "mint"]`） | ✅ |
| 矩阵表示 | `vek::mat::repr_c::column_major::Mat4<f32>` | ✅ |
| `bytemuck` | `1.7` + `derive` | ✅ |
| `image` | `0.25`，`png` | ✅ |

`vek` 的类型必须从 `repr_c` 模块取（`vek::vec::repr_c::Vec3`、`vek::quaternion::repr_c::Quaternion`）。`vek` 还有一个 `repr_simd` 表示，顶层别名在开启 `repr_simd` 特性时会指向它 —— 显式写 `repr_c` 才能跨 crate 保证类型同一。

### 12.2 三个层次

```
crates/blockymodel/          # 与渲染无关，无 GPU 类型
├── json.rs                  # {x,y,z} 形状的 serde 镜像 + 默认值
├── math.rs                  # vek repr_c 的再导出 + 几个小工具
├── model.rs                 # .blockymodel 解析、Face6、Bounds、compose_bone
├── anim.rs                  # .blockyanim 解析 + 全部插值语义
├── skeleton.rs              # 展平骨骼树、姿势求值、骨矩阵
├── mesh.rs                  # box/quad → 顶点与索引、UV 展开
└── tests/dog_asset.rs       # 拿真实资产钉住上面所有容易写错的地方

crates/player/               # wgpu 27 + winit 0.30.12
├── camera.rs                # 轨道相机（含 vek 约定的一致性测试）
├── renderer.rs              # 管线、缓冲、每帧绘制
├── shader.wgsl
├── assets.rs                # 资产发现与加载
├── app.rs                   # 事件、播放状态、帧循环
└── main.rs                  # CLI
```

**解析层不引入任何 GPU 类型。** `Skeleton::bone_matrices()` 返回的是纯 `vek` 矩阵 + `Vec2`，由 `player` 自己打包成 `Pod` 结构上传。

### 12.3 展平骨架

递归树每帧遍历很浪费，而且父子关系要反复查。一次性展平：

```rust
pub struct Skeleton {
    pub bones: Vec<Bone>,                       // 前序：父索引一定小于子索引
    pub by_name: HashMap<String, Vec<usize>>,   // ★ 一个名字可能对应多个节点
    pub bounds: Bounds,                         // bind pose 包围盒
}
```

**前序展平是后面单趟循环的前提**：`for i in 0..n` 时 `parent < i` 必然成立，不需要递归也不需要拓扑排序。

`by_name` 必须是 `Vec<usize>`：Dog 里 `L-Ear2` 被两个节点共用。按名字绑定时要驱动**所有**同名节点。

### 12.4 每帧的姿势与矩阵

```rust
// 1. 采样：把动画的 delta 写进 Pose
skeleton.apply(&mut pose, Some(&anim), seconds);
// 2. 合成 bind pose + 累乘出世界矩阵
let bones: Vec<BoneMatrix> = skeleton.bone_matrices(&pose);
```

`apply` 内部：

```rust
let frame = if anim.hold_last_keyframe {
    (seconds * FPS).clamp(0.0, anim.duration)   // 钳制
} else {
    (seconds * FPS).rem_euclid(anim.duration)  // 循环
};
for (i, bone) in bones.iter().enumerate() {
    if let Some(tracks) = anim.node_animations.get(&bone.name) {  // 名字不在就跳过
        tracks.sample(frame, anim.duration, anim.wraps(), &mut pose.deltas[i]);
    }
}
```

`bone_matrices` 里每个骨骼：

```rust
let position    = bone.position + delta.position;
let orientation = (bone.orientation * delta.rotation).normalized();   // bind ⊗ delta
let stretch     = bone.shape_stretch * delta.stretch;                 // 逐分量乘

// 父 shape.offset 参与平移；父 stretch 不参与
bone_world[i] = parent_world
    * Mat4::translation_3d(parent_offset + position)
    * Mat4::from(orientation);

bone_mat    = bone_world[i] * Mat4::translation_3d(bone.shape_offset)
                             * Mat4::scaling_3d(stretch);
normals_mat = Mat4::from(orientation_world[i]) * Mat4::scaling_3d(recip(stretch));
```

**法线矩阵是解析出来的，不是求逆得到的。** 因为 `stretch` 只作用于该骨骼自己的网格、不传给子节点，所以累乘出来的姿态一定是刚体，线性部分就是 `R · S`，其逆转置就是 `R · S⁻¹`。这既省掉一次 4×4 求逆，也避免了 `Mat4` 里混入平移列。

### 12.5 网格策略：静态 VBO + 每骨骼矩阵

顶点只存**绑定空间**的坐标，每帧只上传几十个矩阵：

| | 顶点数据 | 每帧上传 |
|---|---|---|
| A（本实现采用） | 静态 VBO，绑定空间 | `255 × 144 B` 的骨矩阵数组 |
| B | 每帧 CPU 重建 | 全部顶点 |

顶点按「**烘焙**」处理：位置已经把 bind 的 `stretch` 乘进去了，所以顶点着色器只需要再乘 `S(动画 stretch)`：

```
v_world = bone_mat · v_baked
        = bone_world · T(offset) · S(anim_stretch) · (S(bind_stretch) · v_centered)
```

法线同理烘焙 `n / bind_stretch`（对角缩放的逆转置就是逐分量取倒数），运行时由 `normals_mat` 收尾。

**负 `stretch` 的绕序**：几何被镜像时绕序会翻转，所以在建网格时就按负数轴个数的奇偶把索引顺序换掉（与参考导出一致）。

**`shapeVisible` 的实现技巧**：不重建缓冲区，而是把该骨骼的矩阵换成一个「把所有顶点塌到原点、但齐次行保持 1」的矩阵：

```rust
Mat4::from_col_arrays([[0;4], [0;4], [0;4], [0.0, 0.0, 0.0, 1.0]])
```

退化三角形不会被光栅化，且不会出现 `w = 0` 导致的除零。可见性改回来时直接覆盖即可。

### 12.6 顶点格式

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],   // 绑定空间，已含 bind stretch
    pub normal:   [f32; 3],   // 绑定空间，已含符号翻转
    pub uv:       [f32; 2],   // 已归一化，原点左上
    pub bone:     u32,        // 骨骼索引
    pub unlit:    f32,        // 1.0 = flat/fullbright（不参与光照）
}
```

共 40 字节，5 个 attribute：

```rust
let vertex_attributes = [
    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset:  0, shader_location: 0 },
    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 24, shader_location: 2 },
    wgpu::VertexAttribute { format: wgpu::VertexFormat::Uint32,    offset: 32, shader_location: 3 },
    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32,   offset: 36, shader_location: 4 },
];
```

`shadingMode` 压成一个 float 塞进顶点，比按材质拆 draw call 简单得多。Hytale 自己的资产全是 `flat`（不参与光照），所以默认就是「原样输出贴图」。

### 12.7 骨骼数据布局 ⚠️ 最容易踩的坑

目标项目 `FigurePipeline` 的 `BoneData` 是 `{ bone_mat, normals_mat }`（两个 `[[f32;4];4]`）。本实现要额外带一个 UV 偏移，于是写成：

```rust
#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct BoneData {
    pub bone_mat:    [[f32; 4]; 4],
    pub normals_mat: [[f32; 4]; 4],
    pub uv_offset:   [f32; 4],   // 只用到 .xy
}
```

**为什么 `uv_offset` 是 `vec4` 而不是 `vec2`？**

WGSL 的 **uniform 地址空间**（不是 storage）要求结构体成员的偏移必须是 **16 的倍数**。如果写成 `vec2<f32>`，`naga` 会把它后面的字段对齐到 16 字节，导致 WGSL 侧的结构体大小和 Rust 侧不一致：

```
Rust  : 64 + 64 + 8  (+pad) = 144
WGSL  : 64 + 64 + 8, 但下一个成员必须落在 16 的倍数上 → 结构体变成 160
```

数组步长一旦不一致，`bones[in.bone]` 读到的是错位的字节 —— 表现是「模型碎成一堆乱块」，而**不会**有任何校验报错（如果报，往往只是 "type flags do not meet" 之类的间接信息）。本实现在开发过程中就真的踩了这一个，最终靠补一个 `--info`/截图对比才发现。

两条经验：

1. 结构体里**不要出现尾部 `vec2`**，用 `vec4` 或显式 padding 把每个成员都顶到 16 字节边界。
2. 写完着色器后**一定要真的把窗口跑起来**。`cargo check` 和单元测试都看不到这类布局错误。

### 12.8 管线设置（wgpu 27 写法）

```rust
let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("model pipeline layout"),
    bind_group_layouts: &[&camera_layout, &bone_layout],   // wgpu 27：&[&Layout]
    push_constant_ranges: &[],                             // wgpu 29 已改名为 immediate_size
});

primitive: wgpu::PrimitiveState {
    topology: wgpu::PrimitiveTopology::TriangleList,
    front_face: wgpu::FrontFace::Ccw,
    cull_mode: Some(wgpu::Face::Back),      // 双面形状单独一条 CullMode::None 的管线
    ..Default::default()
},
depth_stencil: Some(wgpu::DepthStencilState {
    format: wgpu::TextureFormat::Depth32Float,
    depth_write_enabled: true,               // wgpu 29 变成 Option<bool>
    depth_compare: wgpu::CompareFunction::Less,
    ..Default::default()
}),
multisample: wgpu::MultisampleState { count: 4, mask: !0, alpha_to_coverage_enabled: false },
multiview: None,
cache: None,
```

`doubleSided` 是**每形状**属性，所以建网格时就把索引分成两份（`indices_single` / `indices_double`），用两条管线、两个 draw call 画。比「全局关剔除 + FS 里按 `front_facing` 翻法线」省片元。

### 12.9 采样器与混合

```rust
mag_filter: wgpu::FilterMode::Nearest,
min_filter: wgpu::FilterMode::Nearest,
mipmap_filter: wgpu::FilterMode::Nearest,       // wgpu 27：FilterMode；29 变成 MipmapFilterMode
address_mode_*: wgpu::AddressMode::ClampToEdge,
```

**像素艺术必须用 `Nearest`**，否则 256×128 的贴图会糊成一片。

片元阶段采用 **alpha test**（`texel.a < 0.5 → discard`）而不是 alpha blending：Hytale 的贴图是硬边镂空，alpha test 让整个 pass 保持不透明，因此**与绘制顺序无关**，不需要排序。管线里 `blend: None`。

### 12.10 每帧流程

```
1. 累加 dt（超过 0.1s 截断，避免窗口拖动后跳帧）
2. 循环动画取模 / hold 动画钳制
3. skeleton.apply(&mut pose, Some(clip), time)
4. skeleton.bone_matrices(&pose)      → Vec<BoneMatrix>
5. queue.write_buffer(camera_buffer, view_proj.into_col_arrays())
   queue.write_buffer(bone_buffer,  &[BoneData; n])
6. 一次 render pass：清屏 → 画单面 → 画双面 → present
```

### 12.11 移植到 Aether_terrain 的清单

1. `crates/blockymodel` → 建议放 `vrage/anim/blockymodel/`（它不含 GPU 代码，和 `vrage_anim` 是同层）。
2. `crates/player/src/renderer.rs` → 按 `vrage_render/src/pipelines/figure.rs` 的形式改成 `struct BlockyModelPipeline`，复用它们的 `FigureLayout` / `Consts<T>` / `Bound<T>` 约定。
3. `BoneData` 直接换成 `vrage_render::pipelines::figure::BoneData`。如果不需要 `shapeUvOffset`，把字段删掉即可（`uv_offset` 是唯一超出目标结构体的部分）。
4. `camera.rs` 丢掉，用 `vrage_render` 已有的相机与 `Locals::model_mat`。
5. 着色器从 WGSL 改成 GLSL（`vrage/render` 用 `shaderc`，特性 `["spirv", "glsl"]`）。矩阵是列主序、`Mat4 * Vec4` 的标准约定，语义一一对应。
6. 纹理换到它们已有的图集（`atlas_offs`）时，把 `uv_offset` 的语义映射过去即可。
7. 删掉 `[workspace.dependencies]` 里重复的 `wgpu` / `winit` / `vek`，改引用工作区的版本。

---

## 附录 A：`Pets/Dog` 资产实测数据

作为实现时的对照基线：

| 项 | 值 |
|---|---|
| 文件 | `Pets/Dog/Models/Model.blockymodel` |
| 大小 | 98 196 字节 / 2950 行 |
| 顶层键 | `nodes`, `lod` |
| 根节点 | 1 个（`Pelvis`） |
| 节点总数 | 31 |
| `box` / `quad` / `none` | 26 / 5 / 0 |
| `unwrapMode` 取值 | 只有 `"custom"` |
| `shadingMode` 取值 | 只有 `"flat"` |
| `angle` 取值 | 只有 `0` 和 `180` |
| 贴图 | `Texture.png`，256 × 128，`Format32bppArgb` |
| 包围盒（bind pose，含旋转） | min `(-21.95, -0.34, -43.76)` / max `(21.95, 88.20, 51.15)` |
| 尺寸 | `43.90 × 88.54 × 94.91` 单位 ≈ `0.69 × 1.38 × 1.48` 方块 |
| 生成几何 | 148 个面 → 592 顶点 / 296 三角形（26 个 box 共 156 个潜在面中 13 个没有 UV 条目，不生成；5 个 quad 全部生成） |
| 重名骨骼 | `L-Ear2`（`R-Ear` 的子节点被误命名成了 `L-Ear2`） |
| `settings` 里的键 | 只有 `size`（没有 `isPiece`，也没有 `isStaticBox`） |
| 有 `children` 键的节点 | 31 / 31（官方插件会在没有子节点时省略该键，游戏资产不省） |
| 负 `stretch` 的形状 | **8 个**，全部是 `x < 0`（见下） |
| `visible` 取值 | 全部 `true` |
| quad 的 `settings.normal` | **全部缺失**（按缺省 `+Z` 处理） |

> **负 `stretch` 是真实存在的，不是理论边界。** `Pets/Dog` 里有 8 个形状的 `stretch.x` 为负，
> 而且**全部是奇数个负轴**（`oddFlips = 1`）：`L-Ear`、`L-Ear2`、`R-UpperLeg`、`R-Leg`、`R-Foot`、
> `R-BackUpperLeg`、`R-BackLeg`、`R-BackFoot`。
>
> 也就是说，**如果一个渲染器忘了按奇偶翻转绕序，这条狗的整个右半边（加左耳）会全部被背面剔除掉**，
> 而 `cargo check`、单元测试、甚至「看起来能跑」都不会告诉你。测试里必须钉住这一点。

> 上面这些数字可以用本文配套的播放器一键复现：
> `cargo run -p blockyanim-player -- --info`

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

> 注意 `R-Ear` 的子节点名字也叫 `L-Ear2`（原作者的命名错误）。**这证明骨骼名不保证唯一** —— 所以绑定必须是 `name → Vec<index>`，不能是 `name → index`。

15 个动作的 `duration`（帧）与最大关键帧时间：

| 动作 | duration | maxKeyTime | holdLastKeyframe |
|---|---|---|---|
| `Hurt` | 20 | 15 | false |
| `Run` | 20 | 20 | false |
| `Jump` / `Jump_Far` | 30 | 25 | false |
| `Bite` / `Bite_Move` | 30 | 25 | false |
| `Alerted` | 40 | 40 | false |
| `Wag` | 40 | 35 | false |
| `Death` | 40 | 30 | **true** |
| `Walk` / `Walk_Backward` | 48 | 45 | false |
| `Fall` | 60 | 30 | false |
| `Sit` | 60 | 45 | false |
| `Idle` / `Sleep` | 80 | 77 / 60 | false |

---

## 附录 B：参考资料与本地参考实现

本文结论的原始出处：

**权威（格式的定义者）**

* [JannisX11/hytale-blockbench-plugin](https://github.com/JannisX11/hytale-blockbench-plugin) — Hypixel Studios 官方 Blockbench 插件
  * `src/blockymodel.ts` — `.blockymodel` 的 `compile()` / `parse()`，UV 映射的权威定义
  * `src/blockyanim.ts` — `.blockyanim` 的导入导出，60 fps 与 `smooth ↔ catmullrom` 的映射
  * `src/animations.ts` — `weightedCubicBezier` 缓动、`shapeStretch` 的乘法语义
  * `src/formats.ts` — `block_size`、`quaternion_interpolation`、`animation_loop_wrapping` 等格式开关
* [Blockbench](https://github.com/JannisX11/blockbench) — 插件的宿主引擎
  * `js/animations/timeline_animators.js` — `interpolate()`，插值分支的权威定义
  * `js/animations/keyframe.js` — `getCatmullromLerp` / `getLerp` / `getBezierLerp`
  * `js/preview/preview.ts` — `block_size` 如何影响预览尺度

**独立实现（可用于交叉验证）**

* [hytale-tools/blockymodel-merger](https://github.com/hytale-tools/blockymodel-merger) — Go 实现，含 CPU 光栅化器与 GLB 导出
  * `pkg/blockymodel/types.go` — 结构体定义
  * `pkg/render/scene.go` — 骨骼变换累乘（本文第 5 节的公式来源）
  * `pkg/render/geometry.go` — box / quad 面顶点表
  * `pkg/export/glb.go` — 顶点顺序、UV 映射、绕序翻转
  * `pkg/anim/anim.go` — 动画的 bind pose 合成规则

**文档**

* [Hytale 官方博客 — Introduction to Making Models for Hytale](https://hytale.com/news/2025/12/an-introduction-to-making-models-for-hytale) —— **可信**。给出「只用 cube 与 quad、不允许球体」、贴图宽高必须是 32 的整数倍、64/32 密度、`stretch` 建议 0.7–1.3×、不用 PBR
* [Hytale Wiki — Technical:Documentation](https://hytalewiki.org/w/Technical:Documentation) —— 官方「正在准备 GitBook 文档，但目前没有官方技术文档」的出处
* `ref/hytale-blockbench-plugin/dist/about.md` —— **可信，且是一级来源**。插件发行包内的官方格式指南：255 节点规则与计数口径、Position/Rotation 打 group 而 Scale/Visibility/UV Offset 只打 shape
* [HytaleModding 社区文档](https://github.com/HytaleModding/site) —— 可信（工作流与集成示例）
* ⚠️ [Hytale Docs — 3D Models](https://hytale-docs.com/docs/modding/art-assets/models) —— **不可信**。该页印的是一套虚构的 Minecraft Bedrock / GeckoLib 风格 schema（`format_version`、`model.identifier`、`bones[].pivot`、`inflate`、「1 pixel = 1/16 block」），与真实格式的每一个特征字段都矛盾。**不要引用、不要照它实现。** 逐条对照见 [`blockymodel-spec.md` 附录 E](./blockymodel-spec.md#附录-e伪造的公开-schema-警示)
* ⚠️ [Hytale Docs — Textures](https://hytale-docs.com/docs/modding/art-assets/textures) —— 掺入无出处的说法（512×512 上限、`_emissive.png`、动画贴图 JSON、必须为 2 的幂），无法佐证

> 完整的公开资料调研（逐条引用、非存在页面的验证、来源权威性分级）见 [`hytale-format-public-docs-survey.md`](./hytale-format-public-docs-survey.md)。
