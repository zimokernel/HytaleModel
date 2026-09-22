# `.blockyanim` 格式规范

**Hytale 骨骼动画格式 —— 第 1 版**

| 项 | 值 |
|---|---|
| 状态 | 稳定（逆向自官方工具链与宿主引擎，逐分支与源码核对） |
| 适用对象 | Hytale 游戏客户端、Blockbench Hytale 插件、任何第三方播放器 |
| 介质类型 | `application/json`，UTF-8，无 BOM，无压缩 |
| 文件扩展名 | `.blockyanim` |
| 帧率 | **恒定 60 fps**（格式内不存储，见 §8） |
| 配套文档 | [`blockymodel-spec.md`](./blockymodel-spec.md)（模型）、[`blockymodel-format.md`](./blockymodel-format.md)（实现笔记） |

> **关于「专业格式文档」的说明。** 截至本文撰写，Hypixel Studios **没有发布** `.blockyanim` 的正式规范。
> Hytale Wiki 的 [Technical:Documentation](https://hytalewiki.org/w/Technical:Documentation) 直接写明：官方「正在准备托管在 GitBook 上的公开创作者文档」，但**目前没有官方技术文档**。公开资料（[Hytale Docs](https://hytale-docs.com/docs/modding/art-assets/models)、[HytaleModding 社区文档](https://github.com/HytaleModding/site)、[Hytale Wiki](https://hytalewiki.org/w/Animation)）只讲工作流与美术约定，不给字段级定义。
>
> 事实上的规范由两段代码共同定义：
> 1. **官方 Blockbench 插件** `JannisX11/hytale-blockbench-plugin` 的 `src/blockyanim.ts` / `src/animations.ts` —— 定义文件结构与字段映射；
> 2. **Blockbench 宿主引擎** `js/animations/timeline_animators.js` 、`js/animations/keyframe.js` —— 定义**求值与插值的全部语义**。
>
> 只读插件源码是不够的：插值路径写在宿主引擎里，而且被格式的两个开关（`quaternion_interpolation`、`animation_loop_wrapping`）改变了行为。本文把两者合并成一份可实现的规范。
>
> ⚠️ **另需注意**：公开的第三方实现里出现过与插件不符的字段（例如给 `interpolationType` 加上 `"step"`），见 [附录 C](#附录-c参考来源)。来源清单与权威性分级也见该附录，完整调研见 [`hytale-format-public-docs-survey.md`](./hytale-format-public-docs-survey.md)。

本文用 RFC 2119 的措辞约定：

* **必须 / MUST** —— 违反会产生与 Hytale 官方播放器不一致的姿势。
* **应该 / SHOULD** —— 违反通常仍可播放，但会产生可察觉的差异。
* **可以 / MAY** —— 完全自由。

---

## 目录

1. [术语](#1-术语)
2. [文件结构](#2-文件结构)
3. [顶层对象](#3-顶层对象)
4. [nodeAnimations](#4-nodeanimations)
5. [通道与 delta 类型](#5-通道与-delta-类型)
6. [关键帧对象](#6-关键帧对象)
7. [时间基准](#7-时间基准)
8. [名字绑定](#8-名字绑定)
9. [与 bind pose 的合成](#9-与-bind-pose-的合成)
10. [采样算法](#10-采样算法)
11. [边界情况](#11-边界情况)
12. [参考伪代码](#12-参考伪代码)
13. [写入者清单](#13-写入者清单)
14. [已知实现分歧](#14-已知实现分歧)
15. [附录 A：参考资产实测数据](#附录-a参考资产实测数据)
16. [附录 B：JSON 语法摘要](#附录-bjson-语法摘要)
17. [附录 C：参考来源](#附录-c参考来源)

---

## 1. 术语

| 术语 | 含义 |
|---|---|
| **clip / 动作** | 一个 `.blockyanim` 文件所描述的一段动画，有一个 `duration`。 |
| **轨道 / track** | 某个骨骼（`nodeAnimations` 的一个键）的某个通道（如 `orientation`）上的关键帧序列。 |
| **通道 / channel** | 五条之一：`position`、`orientation`、`shapeStretch`、`shapeVisible`、`shapeUvOffset`。 |
| **delta** | 关键帧的值。对大多数通道它是**相对 bind pose 的增量**，不是绝对值。 |
| **bind pose** | 模型文件描述的静止姿势（见 `blockymodel-spec.md` §10）。 |
| **帧 / frame** | 时间单位：`1 帧 = 1/60 秒`。 |
| **hold 通道** | 不做插值、只取「最后一个时间 ≤ t 的关键帧」的通道。见 §10.8。 |

---

## 2. 文件结构

* 编码：**UTF-8**，无 BOM。
* 内容：**单个 JSON 对象**（不是数组、不是 JSON Lines）。
* 无魔数、无头部、无压缩。
* 缩进与换行不受限制。游戏自带资产是 2 空格缩进；JSON **键的顺序无意义**（官方资产把 `formatVersion` 放在对象末尾，官方插件放在开头）。
* 未知键**必须**被忽略（前向兼容）。
* 一个文件 = 一个动画。多个动画就是多个文件，按目录分类（见 §2.1）。

### 2.1 资产目录约定

```
Pets/Dog/
├── Models/Model.blockymodel
└── Animations/
    ├── Idle.blockyanim
    ├── Walk.blockyanim
    ├── Attacks/Bite.blockyanim      # 一级子目录只是分类
    └── Idles/Sit.blockyanim         # 文件名才是动作名
```

* **动作名 = 文件名去掉 `.blockyanim`**，不是目录名。
* `Animations/` 下的**一级子目录**（`Attacks/`、`Idles/`）纯粹是分类，官方插件递归读取时会把它们下面的所有 `.blockyanim` 都载入。
* 官方插件在打开模型时，如果开启了 `auto_load_hytale_animations`，会自动扫描 `<模型目录>/../Animations/` 下的一级子目录并载入其中的 `.blockyanim`。**注意**：它只扫描一级子目录，直接放在 `Animations/` 根下的文件**不会**被自动载入。

---

## 3. 顶层对象

```jsonc
{
  "formatVersion": 1,
  "duration": 80,
  "holdLastKeyframe": false,
  "nodeAnimations": { /* §4 */ }
}
```

| 字段 | 类型 | 出现 | 缺省 | 语义 |
|---|---|---|---|---|
| `formatVersion` | 整数 | 应该 | `1` | 至今**所有**已知文件都是 `1`。读取器**应该**接受缺失（视为 1），并在遇到未知版本时尽力解析而不是拒绝。 |
| `duration` | 数值 | 必须 | — | 动作长度，**单位是帧**，不是秒。`seconds = duration / 60`。 |
| `holdLastKeyframe` | 布尔 | 应该 | `false` | `false` → 循环播放；`true` → 停在最后一帧。**它同时决定了 §10.3 的尾段回绕行为**。 |
| `nodeAnimations` | 对象 | 必须 | `{}` | 骨骼名 → 通道集合。见 §4。 |

### 3.1 `duration` 的细节

* 官方插件写出的规则是 `Math.round(animation.length * FPS) || FPS * 2`——即当动作长度算出来是 0 时，回退为 **120 帧**（2 秒）。
* `duration` **不需要**等于最后一个关键帧的时间，也不必是最大的时间。**两者不等的两种情况都是常态**：
  * `duration > maxKeyTime`：尾段要么回绕到第一帧（循环），要么保持最后一帧（`holdLastKeyframe`）。见 §10.3。
  * `duration < maxKeyTime`：理论上不该出现；实用上应把 `duration` 当作唯一的循环周期。
* 60 fps 是**格式硬编码**的，文件里没有任何字段可以覆盖它。

### 3.2 `.blockyanim` **不包含**的东西

以下属性常被误以为在动画文件里，实际上它们属于**服务端**的模型定义 JSON（`Assets/Server/Models/*.json`），由动画**名字**关联。官方 Javadoc 的 [`ModelAsset.Animation`](https://docs.hytale.com/api/com/hypixel/hytale/server/core/asset/type/model/config/ModelAsset.Animation) 列出了它们：

| 字段 | 含义 | 归宿 |
|---|---|---|
| `animation` | 动画**名字**（对应 `.blockyanim` 的文件名） | 服务端 JSON |
| `speed` | 播放速率倍率 | 服务端 JSON |
| `looping` | 是否循环（**独立于** `holdLastKeyframe`） | 服务端 JSON |
| `blendingDuration` | 混合时长 | 服务端 JSON |
| `weight` | 动画权重 | 服务端 JSON |
| `footstepIntervals` | 脚步声帧位 | 服务端 JSON |
| `soundEventId` | 音效事件 | 服务端 JSON |
| `passiveLoopCount` | 被动循环次数 | 服务端 JSON |

换句话说：**`.blockyanim` 只描述骨骼姿势曲线**；「放多快、循环几遍、混多久、配什么音效」全部由服务端的模型定义决定。一个希望还原游戏内表现的播放器需要注意这一点。

（服务端 JSON 的完整字段名/JSON 形状没有公开参考，只有 Javadoc 的 getter 列表。）

---

## 4. `nodeAnimations`

```jsonc
"nodeAnimations": {
  "Pelvis": {
    "position":      [ /* PositionKey[]    */ ],
    "orientation":   [ /* OrientationKey[] */ ],
    "shapeStretch":  [ /* StretchKey[]     */ ],
    "shapeVisible":  [ /* VisibleKey[]     */ ],
    "shapeUvOffset": [ /* UvOffsetKey[]    */ ]
  }
}
```

* **键是骨骼名字**（`string`），**不是 `id`**。名字与模型节点的对应规则见 §8。
* 对象的值可以有 5 个键，**每一个都可以缺失，也可以是空数组 `[]`**。两种写法语义相同（该通道没有数据）。
* 官方插件有一个怪癖：**只要该骨骼有任意通道有数据，它就会保证 `shapeUvOffset` 键存在**（补成 `[]`）。这不是格式要求，读取器不应依赖。
* 骨骼顺序无意义。

---

## 5. 通道与 delta 类型

五个通道的名字、对应的 Blockbench 通道名、以及 `delta` 的 JSON 形状：

| `.blockyanim` 通道 | Blockbench 通道 | `delta` 形状 | 合成语义（相对 bind pose） |
|---|---|---|---|
| `position` | `position` | `{x,y,z}` | **向量加法**：`pos = bind.pos + delta` |
| `orientation` | `rotation` | `{x,y,z,w}` 四元数 | **四元数乘法**：`q = bind.q ⊗ delta`（bind 在**左**） |
| `shapeStretch` | `scale` | `{x,y,z}` 倍数 | **逐分量乘法**：`stretch = bind.stretch * delta` |
| `shapeVisible` | `visibility` | **裸布尔** `true`/`false` | **绝对值**：`visible = delta` |
| `shapeUvOffset` | `uv_offset` | `{x,y}` 整数像素 | **加**到该面的 UV 矩形上，**Y 分量取反**（见 §5.3） |

### 5.1 `shapeVisible` 的 `delta` 是裸布尔

```jsonc
"shapeVisible": [
  { "time": 0,  "delta": false, "interpolationType": "smooth" },
  { "time": 40, "delta": true,  "interpolationType": "smooth" }
]
```

**不是** `{ "x": false, "y": false, "z": false }`，也不是 `{"visibility": true}`。用强类型语言解析时，这个字段必须用「联合类型 + 未标记枚举」或动态 JSON 值来承接——否则一个 `shapeVisible` 关键帧会让**整个文件**解析失败。

### 5.2 `shapeStretch` 是倍数，不是增量

`delta` 的值是**乘数**：`{x:1,y:1,z:1}` 表示不改变，`{x:2,y:1,z:1}` 表示沿 X 拉长一倍。这与 `position` 的加法语义不同，是最容易写错的一条。

官方插件的预览实现印证了这一点（`src/animations.ts` 的 `displayScale`）：

```js
target_shape.stretch = initial_stretch * (1 + (array - 1) * multiplier)   // multiplier = 1 → initial * array
```

### 5.3 `shapeUvOffset` 的 Y 符号

该通道的值最终会加到面的 UV 矩形的四个分量上：

```
u0 += dx;  u1 += dx;  v0 += dy;  v1 += dy
```

但**文件里的 `delta.y` 与「实际加到图像坐标上的偏移」符号相反**。官方插件的导入/导出两端各做了一次取反：

```js
// 导入（blockyanim.ts）
data_point = { x: delta.x, y: -delta.y }
// 导出（blockyanim.ts）
delta = { x: Math.round(x), y: -Math.round(y) }
```

因为 `.blockymodel` 的 UV 用的是**图片坐标系**（原点左上、Y 向下），而 Blockbench 的 UV 面板用数学坐标系。对第三方播放器，正确的换算是：

```
实际像素偏移 = ( delta.x, -delta.y )
```

### 5.4 `orientation` 的 delta 语义

`delta` 是**相对于 bind pose 的增量旋转**，且施加在**骨骼自身的局部坐标系**里：

```
q_final = q_bind ⊗ q_delta
```

由于 bind 在左，`delta` 是「先做 bind、再在 bind 之后的局部系里转」——这与 Blockbench 的 `bone.quaternion.multiply(q2)`（即 `this = this * q`）一致。**顺序反过来会让所有旋转都跑到错误的轴上。**

> 编辑器里的欧拉角往返：官方插件导入时把四元数转成欧拉角（硬编码 `'ZYX'` 序）、导出时再转回四元数。**这是编辑器内部表示，`.blockyanim` 读取器不应该复现它**——文件里存的就是四元数，直接 slerp 即可，没有欧拉角参与。

---

## 6. 关键帧对象

```jsonc
{
  "time": 40,
  "delta": { "x": 0, "y": 0, "z": 0, "w": 1 },
  "interpolationType": "smooth"
}
```

| 字段 | 类型 | 出现 | 缺省 | 语义 |
|---|---|---|---|---|
| `time` | 数值 | 必须 | — | **帧**（60 fps）。官方插件写 `Math.round(kf.time * 60)`，所以实际文件里几乎总是整数。 |
| `delta` | 见 §5 | 必须 | — | 关键帧的值。形状随通道而变。 |
| `interpolationType` | `"smooth"` \| `"linear"` | 可选 | `"linear"` | 该关键帧的**出向**插值风格。见 §6.1。 |

### 6.1 `interpolationType` 的语义

它不是「这个关键帧自己怎么被插值」，而是**这个关键帧之后的那一段怎么被插值**：

| 文件值 | Blockbench 内部值 | 对非旋转通道的含义 |
|---|---|---|
| `"linear"` 或缺失 | `linear` | 该关键帧到下一个关键帧之间线性插值 |
| `"smooth"` | `catmullrom` | 该关键帧到下一个关键帧之间走 **Catmull-Rom 样条**（会用到相邻关键帧） |
| 任何其它字符串 | 未知 | 读取器**应该**按 `"linear"` 处理（见 §14.2） |

对旋转通道，`"smooth"` 的含义不同：**不是样条，而是缓动**（见 §10.7）。

官方插件**只**会写 `"smooth"` 或 `"linear"` 两个值——它把 Blockbench 的 `catmullrom` 映射为 `"smooth"`，把**其它一切**（包括 `linear`、`step`、`bezier`）都映射为 `"linear"`：

```js
interpolationType: kf.interpolation == 'catmullrom' ? 'smooth' : 'linear'
```

**推论**：Blockbench 编辑器里的 `step` 与 `bezier` 关键帧**无法通过 `.blockyanim` 往返**，导出时会被降级成线性。第三方写入器不需要支持它们。

> ⚠️ 关于「`interpolationType` 是否可缺失」：格式上它**可以**缺失（缺省 `linear`），所以读取器必须把它当可选字段处理。但在官方资产 `Pets/Dog` 的 **1168 个关键帧中，没有任何一个缺失该字段**，且取值**全部**是 `"smooth"`。

---

## 7. 时间基准

| 项 | 值 |
|---|---|
| 帧率 | **60 fps**（格式硬编码） |
| `duration` 单位 | 帧 |
| `time` 单位 | 帧 |
| 换算 | `seconds = frames / 60`、`frames = seconds * 60` |
| 关键帧时间量化 | 官方插件按 60 fps 吸附（`snapping: FPS`），因此 `time` 实际总是整数 |
| 时间比较容差 | `epsilon = 1/1200` 秒 = **0.05 帧**（Blockbench `interpolate()` 的 `Math.epsilon` 容差） |

播放一个 clip：

```
未归一化的经过帧数  f = elapsed_seconds * speed * 60
循环（holdLastKeyframe == false）  t = f mod duration
保持（holdLastKeyframe == true）   t = clamp(f, 0, duration)
```

`t` 单位是帧，取值范围 `[0, duration)`（循环）或 `[0, duration]`（保持）。

---

## 8. 名字绑定

轨道按**骨骼名**绑定到模型节点。这条规则有三条反直觉但必须实现的推论：

### 8.1 找不到的名字必须静默忽略

**动画可以引用模型里不存在的骨骼名。** 这些动画是为跨资产变体复用的（例如同一个 `Bite` 动作同时给有项圈和没项圈的宠物用）。

`Pets/Dog` 的实例证据：15 个动作里共出现 31 个骨骼名，而模型只有 30 个，多出来的三个是

```
Collar, R-Ear2, TailDog
```

—— 模型里一个都没有。**读取器绝对不能在遇到未知名字时报错或中断加载**，必须跳过该轨道。

反过来，有 2 个模型骨骼**完全没有**被任何动作驱动：`TongueDog`、`Teeth`。这同样正常。

### 8.2 同名节点必须全部被驱动

**骨骼名不保证唯一**（`blockymodel-spec.md` §5.2），而这是官方插件有意支持的特性：同名骨骼可以一次驱动多个节点。

因此绑定表必须是 `name → Vec<NodeIndex>`，采样时对**所有**同名节点写入同一个 delta。

官方插件甚至会在编辑时把动画复制到所有同名骨骼上（`src/name_overlap.ts` 的 `copyAnimationToGroupsWithSameName`），以保证导出结果与预览一致。

### 8.3 绑定是「模型 → 动画」方向的

实现上的正确做法是：

```
for bone in skeleton.bones:                       # 遍历模型节点
    if let Some(tracks) = node_animations.get(&bone.name):
        tracks.sample(t, ...)                     # 只有命中才采样
```

而**不是**遍历 `nodeAnimations` 去找节点——后者会丢掉同名节点的第二份以后。

---

## 9. 与 bind pose 的合成

采样得到的是 `ChannelDelta`（见 §5 表格）。求最终姿势时：

| 通道 | 公式 |
|---|---|
| `position` | `pos_final = node.position + delta.position` |
| `orientation` | `q_final = node.orientation ⊗ delta.rotation`（**bind 在左**） |
| `shapeStretch` | `stretch_final = shape.stretch * delta.stretch`（**逐分量乘**） |
| `shapeVisible` | `visible = delta`（绝对替换） |
| `shapeUvOffset` | `uv_px = (delta.x, -delta.y)`，加到 UV 矩形四分量上 |

### 9.1 靶点粒度（子节点是否继承）

官方文档（插件发行包内的 `dist/about.md`）明确了每条通道作用在层级的哪一层：

> *"Position and Rotation animations **target the group**, so all child groups and cubes inherit it. However, **Scale, Visibility and UV Offset only target the Shape itself**, so it only applies to the cube."*

| 通道 | 靶点 | 子节点是否继承 |
|---|---|---|
| `position`、`orientation` | 节点自身的变换（进入 `bone_world`） | **是** |
| `shapeStretch` | 仅该节点的形状 | **否** |
| `shapeVisible` | 仅该节点的形状 | **否** |
| `shapeUvOffset` | 仅该节点的形状的面 UV | **否** |

这与 `blockymodel-spec.md` §10.2 规则 2（父节点的 `stretch` 不传给子节点）是同一件事的两种表述。

### 9.2 缺失通道的缺省值

| 通道 | 缺省 delta | 效果 |
|---|---|---|
| `position` | `(0,0,0)` | bind 位置不变 |
| `orientation` | 单位四元数 | bind 旋转不变 |
| `shapeStretch` | `(1,1,1)` | bind 缩放不变 |
| `shapeVisible` | 「无值」 | 使用 `shape.visible`（**不是**假定为 `true`） |
| `shapeUvOffset` | `(0,0)` | 无偏移 |

然后按 `blockymodel-spec.md` §10 的公式重新累乘世界矩阵：

```
bone_world(N)  = bone_world(P) · T(P.shape.offset + position_final) · R(orientation_final)
shape_world(N) = bone_world(N) · T(N.shape.offset) · S(stretch_final)
```

---

## 10. 采样算法

本节是规范的核心。它完整描述了 Blockbench `BoneAnimator.interpolate()` 在 Hytale 格式配置下的行为。Hytale 格式设置的两个开关决定了整条路径：

| 开关（`src/formats.ts`） | 值 | 影响 |
|---|---|---|
| `quaternion_interpolation` | `true` | 旋转通道走 **SLERP**，永远不走样条；并启用官方插件的**加权贝塞尔缓动**钩子 |
| `animation_loop_wrapping` | `true` | 循环 clip 的尾段**回绕**插值到第一个关键帧，而不是钳制 |

### 10.1 第一步：选取包围关键帧 before / after

设当前时间 `t`（帧），轨道上所有关键帧已按 `time` 升序排列。

```
before = time <  t 的关键帧中 time 最大的一个      （严格小于）
after  = time >= t 的关键帧中 time 最小的一个      （大于等于）
```

注意 `before` 用的是**严格小于**、`after` 用的是**大于等于**：`t` 恰好落在某个关键帧上时，它会被选为 `after`。

### 10.2 第二步：回绕（仅循环 clip）

**仅当**下列条件全部满足时执行回绕：

* 格式开关 `animation_loop_wrapping` 为真（Hytale 恒为真）；
* `holdLastKeyframe == false`（即 `loop == 'loop'`）；
* 该轨道至少有 **2** 个关键帧。

```
if before 缺失:  before  = 最后一个关键帧;  before.time -= duration
if after  缺失:  after   = 第一个关键帧;    after.time += duration
```

回绕**发生在容差判断之前**。这是「循环动画的尾段不是保持最后一帧，而是插值回第一帧」这个行为的来源。

用 `Sleep.blockyanim` 验证：`duration = 80`，该轨道最大关键帧时间是 60。于是在 `t ∈ [60, 80]` 区间里 `after` 回绕到第一个关键帧（`time = 0`），其有效时间变成 `80`。`alpha` 从 0 平滑走到 1，姿势从最后一帧渐变回第一帧——循环无缝衔接。

反例：`Death.blockyanim` 的 `holdLastKeyframe = true`，`duration = 40`，最后一个关键帧在 30。`t ∈ [30, 40]` 就是**停在死亡姿势**，不参与回绕。

### 10.3 第三步：容差吸附与退化分支

按顺序判断，**第一个命中者胜出**：

| # | 条件 | 结果 |
|---|---|---|
| 1 | `before` 存在 且 `\|before.time − t\| ≤ ε` | 直接用 `before` 的值 |
| 2 | `after` 存在 且 `\|after.time − t\| ≤ ε` | 直接用 `after` 的值 |
| 3 | `before` 存在 且 `before.interpolationType` 为 hold 类（`"step"`） | 用 `before` 的值（**不会到达这里**，见 §14.2） |
| 4 | `before` 存在 但 `after` 不存在 | 用 `before` 的值（**钳制**） |
| 5 | `after` 存在 但 `before` 不存在 | 用 `after` 的值（**钳制**） |
| 6 | 两者都不存在 | 无值（轨道为空） |
| 7 | 否则 | 进入 §10.4 的插值 |

其中 `ε = 1/1200 秒 = 0.05 帧`。

分支 4 / 5 是钳制语义：`t` 早于第一个关键帧时用第一个关键帧的值；`t` 晚于最后一个关键帧且**不回绕**（`holdLastKeyframe = true`）时用最后一个关键帧的值。

### 10.4 第四步：按通道选择插值类别

`alpha = (t − before.time) / (after.time − before.time)`，钳制到 `[0, 1]`。

| 通道 | 插值类别 |
|---|---|
| `orientation` | **永远 SLERP**（§10.7），不查其它关键帧 |
| `shapeVisible` | **hold**：取 `time ≤ t` 的最后一个关键帧（§10.8） |
| `shapeUvOffset` | **hold**：同上（§10.8） |
| `position`、`shapeStretch` | 按下面的分支：线性 或 Catmull-Rom（§10.5 / §10.6） |

对后两个通道：

```
if  before.interpolationType == "linear"
and (after.interpolationType == "linear" or after.interpolationType 缺失):
        value = lerp(before.value, after.value, alpha)              // §10.5
else if before.interpolationType == "smooth"
     or after.interpolationType  == "smooth":
        value = catmull_rom(P0, P1, P2, P3, alpha)                  // §10.6
else:
        value = lerp(before.value, after.value, alpha)              // 兜底
```

注意条件是 **`before` 是 `linear` 且 `after` 不是 `smooth`**。只要**任意一端**是 `"smooth"`，就走样条。

### 10.5 线性插值

```
value = before.value + (after.value − before.value) * alpha
```

逐分量作用于向量。

### 10.6 Catmull-Rom 插值

**邻居关键帧的选择**（`P1 = before`、`P2 = after`）：

```
index = 排序后轨道中 before 的下标
P0 = 轨道[index − 1]；若不存在：
       若（clip 是循环 且 轨道关键帧数 >= 3）→ P0 = 轨道[倒数第 2 个]
       否则                                → P0 = P1
P3 = 轨道[index + 2]；若不存在：
       若（clip 是循环 且 轨道关键帧数 >= 3）→ P3 = 轨道[第 2 个]
       否则                                → P3 = P2
```

（“clip 是循环”即 `holdLastKeyframe == false`。）

**插值核**是 THREE.js `SplineCurve` 的**均匀 Catmull-Rom**（不是 centripetal，也不是按时间参数化的）：

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

逐分量作用于向量（`shapeUvOffset` 若参与插值则同理，但它实际是 hold 通道）。

**三条必须照做的细节：**

1. **参数按「关键帧索引」而不是「时间」参数化。**
   Blockbench 的做法是：把 `P0..P3`（实际存在的那些）组成一条 `THREE.SplineCurve`，然后用
   ```
   spline_param = (alpha + (P0 存在 ? 1 : 0)) / (控制点个数 − 1)
   ```
   取样。由于 `SplineCurve.getPoint` 内部再乘 `(控制点个数 − 1)`，最终落在 `before→after` 这一段上的局部参数**恰好就是 `alpha`**。
   换言之：**关键帧之间的时间间隔不均匀，不会让样条在时间上被拉伸**。直接把 `alpha` 喂给上面的核函数即可——这正是 Blockbench 的行为。

2. **缺少邻居时是「复制端点」，不是「外推」。**
   THREE.js `SplineCurve.getPoint` 在段的两端把 `P0` 复制成 `P1`、`P3` 复制成 `P2`（clamp），而不是做线性外推。
   差别是可测量的：`catmull_rom(1, 1, 3, 3, 0.25) = 1.40625`，而线性值应当是 `1.5`。**不要**用 «P0 = 2·P1 − P2» 去近似——那会退化成直线。
   下面的情形都会退化为「复制端点」，从而让该段看起来比线性更「贴」：
   * 轨道只有 2 个关键帧；
   * `before` 是第一个关键帧且不满足回绕条件；
   * `after` 是最后一个关键帧且不满足回绕条件。

3. **回绕时 `P0` / `P3` 也回绕**（见上面的选择规则），所以循环 clip 在首尾接缝处是**平滑**的，不是折角。

### 10.7 旋转通道：SLERP + 加权三次贝塞尔缓动

因为格式开启了 `quaternion_interpolation`，旋转通道**永远**是球面插值，**永远不会**走 Catmull-Rom，**永远不查**邻居关键帧。

```
alpha2 = alpha
if before.interpolationType == "smooth" and after.interpolationType == "smooth":
        alpha2 = weighted_cubic_bezier(alpha)
q = slerp(before.delta, after.delta, alpha2)
```

**缓动函数**（官方插件 `src/animations.ts` 的 `weightedCubicBezier`，通过 Blockbench 的 `interpolate_keyframes` 事件注入）：

```rust
/// 加权三次贝塞尔缓动。接近 smoothstep 但**不等价** ——
/// 不要用 t*t*(3-2t) 代替，差异肉眼可见。
fn weighted_cubic_bezier(t: f32) -> f32 {
    const P: [f32; 4] = [0.0, 0.05, 0.95, 1.0];   // 控制点
    const W: [f32; 4] = [2.0, 1.0,  2.0, 1.0];    // 权重
    let mt = 1.0 - t;
    let b = [mt*mt*mt, 3.0*mt*mt*t, 3.0*mt*t*t, t*t*t];   // 三次伯恩斯坦基
    let (mut num, mut den) = (0.0, 0.0);
    for i in 0..4 { num += b[i] * W[i] * P[i]; den += b[i] * W[i]; }
    num / den
}
```

要点：

* 缓动**只作用于旋转**。官方插件的钩子会检查 `arg.use_quaternions`，非旋转通道直接返回、不缓动。
* 缓动**只在两端都是 `"smooth"` 时**生效（插件的钩子要求 `keyframe_before.interpolation == 'catmullrom' && keyframe_after.interpolation == 'catmullrom'`，而 `'catmullrom'` 正是文件里的 `"smooth"`）。
* 缓动**不改变端点**：`weighted_cubic_bezier(0) = 0`、`weighted_cubic_bezier(1) = 1`，单调递增。
* 缓动后的 `alpha2` 用于 SLERP 的插值参数，**不是**用来改时间的。

**SLERP 的输入是 `delta` 四元数本身**（不是与 bind 相乘之后的四元数）。bind 的乘法在采样之后统一进行（§9）。

**关于短弧**：标准 SLERP 应当在两个四元数点积为负时取反其中一个，走短弧。Blockbench 走的是 THREE.js `Quaternion.slerp`，其行为与标准一致。实现时应该保证走短弧，否则快速的 180° 以上旋转会绕远路。

### 10.8 hold 通道：`shapeVisible` 与 `shapeUvOffset`

这两个通道**不做任何插值**。它们的取值规则是「**最后一个 `time ≤ t` 的关键帧**」：

```
value(t) = 满足 time <= t 的关键帧中 time 最大者的 delta
如果不存在这样的关键帧：
    shapeVisible  → 该通道「无值」→ 回退到模型里这个 shape 的 visible 标志
    shapeUvOffset  → 偏移视为 (0, 0)
```

`shapeVisible` 的官方实现（`src/animations.ts` 的 `displayVisibility`）在找不到「time ≤ t」的关键帧时，用的是 `group.visibility`——也就是模型里该形状自带的 `visible` 字段，**不是**轨道里第一个关键帧的值。

`shapeUvOffset` 的官方实现（`displayUVOffset`）在找不到时重置为 `(0, 0)`。

> 这是一个容易被忽略的语义：滚动贴图 / 眨眼这类 UV 动画是**抽帧**的，不是平滑过渡的。

---

## 11. 边界情况

| 情况 | 行为 |
|---|---|
| 轨道只有 1 个关键帧 | 整个 clip 期间恒定为该值（`before` 或 `after` 单独存在 → §10.3 分支 4/5） |
| 轨道为空数组 | 该通道无数据，用 §9 的缺省 delta |
| 轨道缺失 | 同上 |
| `t` 早于第一个关键帧，且循环回绕生效 | `before` 回绕成最后一个关键帧，其时间被减掉一个 `duration`；正常插值 |
| `t` 早于第一个关键帧，且不回绕 | 用第一个关键帧的值 |
| `t` 晚于最后一个关键帧，`holdLastKeyframe == true` | 用最后一个关键帧的值（钳制） |
| `t` 晚于最后一个关键帧，`holdLastKeyframe == false` | `after` 回绕成第一个关键帧，其时间加一个 `duration`（参见 §10.2） |
| `t` 恰好等于某个关键帧时间（容差 ±0.05 帧内） | 直接取该关键帧的值，不插值 |
| `before.time == after.time`（重帧） | `alpha` 的分母为 0；实现应把 `alpha` 置 0 而不是产生 `NaN` |
| `duration == 0` | 视为退化；实现应至少保证不除零 |
| `nodeAnimations` 引用了不存在的骨骼名 | 静默忽略（§8.1） |
| 一个骨骼名对应多个模型节点 | 全部驱动（§8.2） |
| `interpolationType` 取值未知 | 按 `"linear"` 处理（§14.2） |
| `delta` 是四元数但未归一化 | 使用前归一化 |

---

## 12. 参考伪代码

把 §10 完整落到代码：

```rust
const FPS: f32 = 60.0;
const EPSILON_FRAMES: f32 = FPS / 1200.0;      // 0.05 帧

/// 时间归一化：秒 → clip 内的时间（帧）
fn clip_time(clip: &Clip, seconds: f32) -> f32 {
    let f = seconds * FPS;
    if clip.hold_last_keyframe {
        f.clamp(0.0, clip.duration)             // 保持
    } else {
        f.rem_euclid(clip.duration)             // 循环
    }
}

/// 逐骨骼采样。★ 遍历的是**模型骨骼**，不是 nodeAnimations 的键（§8.3）。
fn sample_clip(clip: &Clip, seconds: f32) -> Pose {
    let t = clip_time(clip, seconds);
    let mut pose = Pose::bind(&clip.skeleton);
    for (i, bone) in clip.skeleton.bones.iter().enumerate() {
        let Some(tracks) = clip.node_animations.get(&bone.name) else { continue };
        tracks.sample(t, clip.duration, !clip.hold_last_keyframe, &mut pose.deltas[i]);
    }
    pose
}

/// 包围关键帧，附带**平移后**的时间。
/// 回绕出来的那一端的时间会被整体加减一个 duration，这是 alpha 正确的前提。
struct Bracket {
    before: Option<(usize, f32)>,
    after:  Option<(usize, f32)>,
}

fn bracket(keys: &[Key], t: f32, duration: f32, wrap: bool) -> Bracket {
    let mut before: Option<usize> = None;        // time <  t，取最大
    let mut after:  Option<usize> = None;        // time >= t，取最小
    for i in 0..keys.len() {
        if keys[i].time < t {
            if before.is_none_or(|b| keys[i].time > keys[b].time) { before = Some(i); }
        } else if after.is_none_or(|a| keys[i].time < keys[a].time) {
            after = Some(i);
        }
    }

    let mut before_time = before.map(|i| keys[i].time).unwrap_or(0.0);
    let mut after_time  = after .map(|i| keys[i].time).unwrap_or(0.0);

    // 回绕：仅循环 clip 且轨道 >= 2 个关键帧（§10.2）
    if wrap && keys.len() >= 2 {
        if before.is_none() {
            let last = argmax_time(keys);
            before = Some(last);
            before_time = keys[last].time - duration;   // ★ 减一个周期
        }
        if after.is_none() {
            let first = argmin_time(keys);
            after = Some(first);
            after_time = keys[first].time + duration;   // ★ 加一个周期
        }
    }

    Bracket {
        before: before.map(|i| (i, before_time)),
        after:  after .map(|i| (i, after_time)),
    }
}
```

采样单条非旋转轨道：

```rust
fn sample_spline(keys: &[Key], t: f32, duration: f32, wrap: bool) -> Option<Vec3> {
    if keys.is_empty() { return None; }
    let b = bracket(keys, t, duration, wrap);

    let (Some((bi, bt)), Some((ai, at))) = (b.before, b.after) else {
        // §10.3 分支 4/5：只有一端 → 钳制
        return b.before.or(b.after).map(|(i, _)| keys[i].value);
    };
    if (t - bt).abs() <= EPSILON_FRAMES { return Some(keys[bi].value); }
    if (at - t).abs() <= EPSILON_FRAMES { return Some(keys[ai].value); }

    let alpha = ((t - bt) / (at - bt)).clamp(0.0, 1.0);

    let ends_smooth = keys[bi].interp == Smooth || keys[ai].interp == Smooth;
    if !ends_smooth {
        return Some(lerp(keys[bi].value, keys[ai].value, alpha));
    }

    // Catmull-Rom：邻居按索引取，缺失则复制端点（§10.6）
    let p0 = neighbour_before(keys, bi, wrap);
    let p3 = neighbour_after(keys, ai, wrap);
    Some(catmull_rom(keys[p0].value, keys[bi].value, keys[ai].value, keys[p3].value, alpha))
}

/// `before` 的前一个关键帧，缺失时返回 `i` 本身（复制端点，不是外推）。
fn neighbour_before(keys: &[Key], i: usize, wrap: bool) -> usize {
    if i > 0 { return i - 1; }
    if wrap && keys.len() >= 3 { return keys.len() - 2; }   // 倒数第二个
    i
}

/// `after` 的后一个关键帧，缺失时返回 `i` 本身。
fn neighbour_after(keys: &[Key], i: usize, wrap: bool) -> usize {
    if i + 1 < keys.len() { return i + 1; }
    if wrap && keys.len() >= 3 { return 1; }                // 第二个
    i
}
```

采样旋转轨道：

```rust
fn sample_rotation(keys: &[Key], t: f32, duration: f32, wrap: bool) -> Option<Quat> {
    if keys.is_empty() { return None; }
    let b = bracket(keys, t, duration, wrap);

    let (Some((bi, bt)), Some((ai, at))) = (b.before, b.after) else {
        return b.before.or(b.after).map(|(i, _)| keys[i].quat);
    };
    if (t - bt).abs() <= EPSILON_FRAMES { return Some(keys[bi].quat); }
    if (at - t).abs() <= EPSILON_FRAMES { return Some(keys[ai].quat); }

    let mut alpha = ((t - bt) / (at - bt)).clamp(0.0, 1.0);
    if keys[bi].interp == Smooth && keys[ai].interp == Smooth {
        alpha = weighted_cubic_bezier(alpha);       // ★ 缓动，不是样条
    }
    Some(slerp_shortest(keys[bi].quat, keys[ai].quat, alpha))
}
```

hold 轨道：

```rust
fn sample_visible(keys: &[Key], t: f32) -> Option<bool> {
    keys.iter().filter(|k| k.time <= t).max_by(|x, y| x.time.total_cmp(&y.time)).map(|k| k.value)
    // None → 调用方回退到 shape.visible
}
```

---

## 13. 写入者清单

1. **`duration` 用帧**（`round(length_seconds * 60)`），不是秒。长度为 0 时回退 120 帧。
2. **`time` 用帧**，整数。
3. **`formatVersion: 1`** 必须写出。
4. **`holdLastKeyframe` 必须写出**——它同时控制循环模式与尾段回绕。
5. 每个有数据的骨骼都要写全五个通道键（没有数据的写 `[]`）——这是官方插件的输出形态。
6. **`shapeVisible.delta` 写裸布尔**，不是对象。
7. **`shapeUvOffset.delta.y` 写出时要取反**（与内部约定相反，见 §5.3）。
8. **`shapeStretch.delta` 写乘数**，缺省基准是 `(1,1,1)`。
9. `orientation.delta` 写**归一化**四元数 `{x,y,z,w}`（标量在后）。
10. `interpolationType` 只写 `"smooth"` 或 `"linear"`。
11. 按 `time` 升序写出关键帧（读取器不应依赖顺序，但排序能让 diff 稳定）。
12. 骨骼名要**与模型里的名字逐字符一致**——绑定是大小写敏感的精确字符串比较。

---

## 14. 已知实现分歧

本节记录「规范要求的行为」与「常见实现的实际行为」的差异，供移植与修 bug 时对照。

### 14.1 `shapeUvOffset` 被误当作插值通道

规范（§10.8）：`shapeUvOffset` 是 **hold** 通道，取最后一个 `time ≤ t` 的关键帧。

本仓库 `crates/blockymodel/src/anim.rs` 的现状：把 `shape_uv_offset` 交给 `sample_spline()`**插值**。当关键帧之间才发生 UV 位移时，这会产出 Blockbench 里不存在的中间值。修正方式是把该通道改走 hold 路径，只在最后做 `(x, -y)` 的符号翻转。

### 14.2 `interpolationType: "step"` 的处理

Blockbench 引擎在插值分派**之前**有一个分支：`before.interpolation == step` → 直接沿用 `before`（保持）。

官方插件永远不写 `"step"`（导出时降级为 `"linear"`），所以这条路径在 `.blockyanim` 里实际不可达。一个**符合 Blockbench** 的读取器可以把它实现成 hold；把它当作 `"linear"` 是一个无害的近似。

### 14.3 `shapeVisible` 无匹配关键帧时的回退值

规范（§10.8）：回退到模型里该 shape 的 `visible` 标志。

本仓库实现的现状：回退到轨道里**第一个**关键帧的值。当第一个关键帧的 `time > 0` 时两者不同。正确做法是让采样返回「无值」（`Option::None`），由调用方回退到 `shape.visible`。

### 14.4 旋转的欧拉角往返

官方插件（编辑器侧）会把四元数转成欧拉角再转回来，并在 `Math.roundTo(..., 3)` 处损失精度。**读取 `.blockyanim` 时不应复现**——直接 slerp `delta` 四元数（§5.4）。

### 14.5 `step` / `bezier` 不可往返

编辑器里的 `step` 与 `bezier` 关键帧导出后都变成 `"linear"`，重新导入时不会恢复。这是格式的既有约束，不是 bug。

---

## 附录 A：参考资产实测数据

`Pets/Dog/Animations/` —— 15 个官方动作文件。

| 动作 | `duration`（帧） | 最大关键帧时间 | `holdLastKeyframe` | 文件字节数 |
|---|---|---|---|---|
| `Hurt` | 20 | 15 | false | 13 143 |
| `Run` | 20 | 20 | false | 17 584 |
| `Jump` | 30 | 25 | false | 18 851 |
| `Jump_Far` | 30 | 25 | false | 18 144 |
| `Bite` | 30 | 25 | false | 20 378 |
| `Bite_Move` | 30 | 25 | false | 12 111 |
| `Alerted` | 40 | 40 | false | 24 661 |
| `Wag` | 40 | 35 | false | 31 075 |
| `Death` | 40 | 30 | **true** | 19 487 |
| `Walk` | 48 | 45 | false | 25 167 |
| `Walk_Backward` | 48 | 45 | false | 25 515 |
| `Fall` | 60 | 30 | false | 14 140 |
| `Sit` | 60 | 45 | false | 17 515 |
| `Idle` | 80 | 77 | false | 19 896 |
| `Sleep` | 80 | 60 | false | 14 024 |

聚合统计：

| 项 | 值 |
|---|---|
| `formatVersion` | 全部为 `1`（写在对象**末尾**） |
| 顶层键 | `duration`, `holdLastKeyframe`, `nodeAnimations`, `formatVersion` |
| 关键帧总数 | 1 168 |
| 各通道关键帧数 | `orientation` 1 000、`position` 118、`shapeStretch` 50、`shapeVisible` 0、`shapeUvOffset` 0 |
| `interpolationType` 取值分布 | `"smooth"` × 1 168（**全部**） |
| `interpolationType` 缺失的关键帧 | **0** |
| 五个通道键是否总是存在 | 是（无数据的写 `[]`） |
| 动画里出现的骨骼名总数 | 31 |
| 模型里的骨骼名总数 | 30 |
| **动画引用但模型里不存在** | `Collar`、`R-Ear2`、`TailDog` |
| **模型里存在但从未被动画驱动** | `TongueDog`、`Teeth` |
| 循环接缝实例 | `Sleep`：`duration = 80`、最大关键帧 60 → `t ∈ [60, 80]` 回绕到第一帧 |
| 钳制实例 | `Death`：`holdLastKeyframe = true`、`duration = 40`、最大关键帧 30 → `t ∈ [30, 40]` 停在死亡姿势 |

> 这批数据可以复现：`cargo run -p blockyanim-player -- --info`。

---

## 附录 B：JSON 语法摘要

```
document    := {
  "formatVersion"?:   integer,                  // 恒为 1
  "duration":         number,                   // 帧
  "holdLastKeyframe"?: bool,                    // 缺省 false
  "nodeAnimations":   { <boneName>: tracks, ... }
}

tracks      := {
  "position"?:      [ positionKey, ... ],
  "orientation"?:   [ orientationKey, ... ],
  "shapeStretch"?:  [ stretchKey, ... ],
  "shapeVisible"?:  [ visibleKey, ... ],
  "shapeUvOffset"?: [ uvOffsetKey, ... ]
}

positionKey     := { "time": number, "delta": vec3, "interpolationType"?: "smooth"|"linear" }
orientationKey  := { "time": number, "delta": quat, "interpolationType"?: "smooth"|"linear" }
stretchKey      := { "time": number, "delta": vec3, "interpolationType"?: "smooth"|"linear" }
visibleKey      := { "time": number, "delta": bool, "interpolationType"?: "smooth"|"linear" }
uvOffsetKey     := { "time": number, "delta": vec2, "interpolationType"?: "smooth"|"linear" }

vec2        := { "x": number, "y": number }
vec3        := { "x": number, "y": number, "z": number }
quat        := { "x": number, "y": number, "z": number, "w": number }   // 标量在后
boneName    := string     // 必须与 .blockymodel 中的 node.name 精确匹配
```

---

## 附录 C：参考来源

**一级来源（格式的定义者）**

* [JannisX11/hytale-blockbench-plugin](https://github.com/JannisX11/hytale-blockbench-plugin) — Hypixel Studios 官方 Blockbench 插件（GPL-3.0）
  * `src/blockyanim.ts` —— `parseAnimationFile()` / `compileAnimationFile()`：`FPS = 60`、`duration` 与 `time` 的帧换算、`holdLastKeyframe ↔ loop` 映射、`smooth ↔ catmullrom` 映射、`shapeVisible` 裸布尔、`shapeUvOffset` 的 Y 取反、`duration` 为 0 时回退 120
  * `src/animations.ts` —— `weightedCubicBezier()` 缓动、`interpolate_keyframes` 钩子（只作用于四元数、只作用于两端都 smooth 的情况）、`displayVisibility` 与 `displayUVOffset` 的 hold 语义、`displayScale` 的乘法语义
  * `src/name_overlap.ts` —— 同名骨骼共享动画
  * `src/formats.ts` —— `quaternion_interpolation: true`、`animation_loop_wrapping: true`、`animation_grouping: 'custom'`、`animation_files: true`
* [Blockbench](https://github.com/JannisX11/blockbench) — 插件宿主引擎
  * `js/animations/timeline_animators.js` —— `BoneAnimator.interpolate()`，§10.1–§10.4 的全部分支与 `epsilon = 1/1200`
  * `js/animations/keyframe.js` —— `getCatmullromLerp()`（`SplineCurve` 构造与索引空间参数化）、`getLerp()`、`getBezierLerp()`
  * `js/preview/preview_scenes.ts` —— 参考用的 UV 角点轮换实现

**三级来源（依赖库行为）**

* [three.js `SplineCurve`](https://unpkg.com/three@0.160.0/src/extras/curves/SplineCurve.js) —— 均匀 Catmull-Rom 核，以及**端点复制（clamp）而非外推**的行为（r110 与 r160 一致）

**二级来源（交叉验证）**

* [hytale-tools/blockymodel-merger](https://github.com/hytale-tools/blockymodel-merger) — Go 实现（GPL-3.0）
  * `pkg/anim/anim.go` —— bind pose 合成规则
  * README —— `-pose <file.blockyanim>`「把任意动画的第 0 帧当静态姿势」，即「第 0 帧 = 一个静态姿势」是共享约定
* `Pets/Dog/Animations/*.blockyanim` —— Hytale 官方资产，本文所有实测数据的来源

**官方但非格式相关**

* [docs.hytale.com — `ModelAsset.Animation`](https://docs.hytale.com/api/com/hypixel/hytale/server/core/asset/type/model/config/ModelAsset.Animation) / [`protocol.Animation`](https://docs.hytale.com/api/com/hypixel/hytale/protocol/Animation) —— 证明 `speed`、`looping`、`blendingDuration`、`weight`、`footstepIntervals`、`soundEventId`、`passiveLoopCount` 属于**服务端**模型 JSON（§3.2），不在这份文件里

**三级来源（第三方实现，仅供对照，不得作为规范依据）**

* [`@hytale-tools/skin-viewer`](https://www.npmjs.com/package/@hytale-tools/skin-viewer)（GPL-3.0）—— 公开了目前最清晰的 `.blockyanim` 接口（`BlockyAnimData` / `Keyframe<T>` / 五通道），并与本文一致地写明 60 fps、按名字绑定、「找不到的轨道静默丢弃」。**但它的 `interpolationType?: "smooth"|"linear"|"step"` 多出了一个 `"step"`**——插件的类型联合只有 `'smooth' | 'linear'`，且会把一切非 Catmull-Rom 的曲线（含编辑器里的 step / bezier）降级成 `"linear"`。`"step"` 描述的是**编辑器状态**，不是文件内容。其 `1/16` 位移缩放同样是 three.js 查看器的显示约定，不是 Hytale 的世界单位。
* [blockymodel-web](https://www.npmjs.com/package/blockymodel-web) —— 配套的 `.blockymodel` TS 类型

**公开文档（无格式细节，仅工作流）**

* [Hytale 官方博客 — An Introduction to Making Models for Hytale](https://hytale.com/news/2025/12/an-introduction-to-making-models-for-hytale)
* [Hytale Wiki — Animation](https://hytalewiki.org/w/Animation) —— *"all of this information takes place in 60 frames a second"*，以及武器动作的帧预算
* [Hytale Wiki — Blockbench](https://hytalewiki.org/w/Blockbench) —— 转述插件自带的格式指南（Position/Rotation 打 group，Scale/Visibility/UV Offset 只打 shape）
* [HytaleModding 社区文档](https://github.com/HytaleModding/site) —— 含把 `.blockyanim` 接进方块 JSON 的真实可用示例（`CustomModelAnimation`、`Looping`、`RequiresAlphaBlending`）

**需要避开的来源**

* ⚠️ [hytale-docs.com — 3D Models](https://hytale-docs.com/docs/modding/art-assets/models) —— 印的是虚构的 Minecraft Bedrock 风格 schema，与 `.blockymodel` / `.blockyanim` 都无关。详见 `blockymodel-spec.md` 附录 E。

> 完整的公开资料调研（含逐条引用、非存在页面的验证、以及每个来源的权威性分级）见 [`hytale-format-public-docs-survey.md`](./hytale-format-public-docs-survey.md)。

> **结论**：`.blockyanim` **没有**公开的正式规范。它的完整语义分布在「官方插件」与「Blockbench 宿主引擎」两处源码里，任何第三方播放器都需要同时读这两处，并用真实资产做回归测试。
