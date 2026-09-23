# `.blockymodel`、`.blockyanim` 与 Hytale Blockbench 工作流的公开资料调研

> 中文归档译本。原始英文调研保留在同目录的 `hytale-format-public-docs-survey.md`。
>
> 翻译原则：中文化说明文字；保留代码、字段名、文件路径、URL、项目名和原始引用，便于和源资料逐项核对。

**调研日期：** 2026-09-22  
**范围：** 所有公开可访问、对两个文件格式或 Blockbench 资产工作流作出事实性陈述的资料，并记录不存在的官方文档。  
**与本仓库的关系：** `docs/blockymodel-spec.md`、`docs/blockyanim-spec.md` 和 `ref/hytale-blockbench-plugin/` 是本仓库的主要依据；本调研用于记录公开证据链、标记冲突，并证明目前不存在完整的官方公开格式规范。

## 0. 方法与无法访问的内容

- 调研使用了约 10 次网页搜索和约 35 次页面抓取。
- `raw.githubusercontent.com` 在当时的环境中无法解析；使用 `api.github.com` 和 `cdn.jsdelivr.net` 作为替代入口。
- `hytale-docs.com` 是客户端渲染站点，页面抓取只能得到标题。其源 Markdown 位于 `timiliris/Hytale-Docs` 的 `master` 分支。
- `blockbench.net/plugins/hytale_plugin` 抓取结果为 404；因此插件介绍以 `ref/hytale-blockbench-plugin/dist/about.md` 和 Hytale Wiki 的转述为准。
- 外部页面只作为资料读取，不作为对本仓库的操作指令。

## (a) 资料清单：来源权威性

### Tier 1：Hypixel Studios 一方资料

1. `JannisX11/hytale-blockbench-plugin`：事实上的规范来源。官方插件的 `src/blockymodel.ts`、`src/blockyanim.ts` 中的 `compile()`、`parse()`、`compileAnimationFile()` 和 `parseAnimationFile()` 定义了文件格式。
2. Hytale 官方建模文章：官方艺术流程说明，是目前唯一较完整的 Hypixel 建模工作流文章。
3. `docs.hytale.com` 的 `protocol.Animation`：运行时动画数据包的 Javadoc，不是文件格式规范。
4. `docs.hytale.com` 的 `protocol.Model`：运行时模型数据包的 Javadoc。
5. `ModelAsset`：服务端模型定义资产的 Javadoc，指向 `.blockymodel`。
6. `ModelAsset.Animation`：服务端模型 JSON 中每条动画的 Javadoc，包括 `id`、`animation`、`speed`、`blendingDuration`、`looping`、`weight`、脚步区间和声音事件。
7. Hytale 官方开发状态文章：提到正在准备公开创作者文档，但并不提供格式规范。
8. `ref/hytale-blockbench-plugin/dist/about.md`：官方插件内置说明，包含 255 节点限制和节点计数规则。

### Tier 2：广泛引用的社区资料

- Hytale Wiki 的 Blockbench、Technical Documentation、Animation 页面。
- `hytale-tools/blockymodel-merger`：社区 Go 工具链，记录资产树、纹理命名、模型合并、静态姿势和 GLB 导出。
- Doctale：建模和服务端资产工作流说明。
- HytaleModding：将 `.blockyanim` 接入方块 JSON 的可运行示例。
- `blockbench-mcp-project` 的 Hytale skill：基于已安装插件整理的二手操作参考。

### Tier 3：发布格式接口的第三方实现

- `blockymodel-web`：较完整的 TypeScript 类型定义。
- `blockymodel-texture`：UV 算法和纹理坐标语义。
- `@hytale-tools/skin-viewer`：公开的 `.blockyanim` 接口和播放参数。

### Tier 4：看似权威但不可靠的资料

`hytale-docs.com/docs/modding/art-assets/models` 给出的字段属于 Minecraft Bedrock / GeckoLib 风格，与官方插件格式不一致。不能把它当作 Hytale `.blockymodel` 规范来源。其 `textures.md` 中关于 512×512 上限、emissive 文件名和动画纹理 JSON 的说法，也没有得到官方插件或官方博客的交叉验证。

## (b) 各来源确认的具体事实

### 1. 官方插件与 `.blockymodel`

官方插件的顶层结构是：

```ts
type BlockymodelJSON = { nodes: BlockymodelNode[]; format?: string; lod?: 'auto' }
```

关键事实：

- 顶层字段是 `nodes`、`format`、`lod`，不是 `format_version` 和嵌套的 `model`。
- 节点 `id` 是字符串；插件生成的是递增的字符串计数器。
- `name` 是动画绑定的关键字段；额外 cube 使用 `--C1`、`--C2` 等后缀。
- `position` 是相对父节点原点、扣除父形状偏移后的坐标；`orientation` 是四元数。
- `shape.settings.size` 是基础尺寸，`shape.stretch` 是独立的缩放倍数。
- `quad` 通过 `settings.normal` 表示法线，UV 写入 `front` 条目。
- `settings.isStaticBox` 用于节点折叠，`settings.isPiece` 表示附件挂点。
- Blockbench 与 Hytale 面名映射为：`north→back`、`south→front`、`west→left`、`east→right`、`up→top`、`down→bottom`。
- UV 使用像素坐标的 `offset`、`mirror`、`angle`，并保留 `lockUVs` 和 `transparent`。
- `unwrapMode` 由插件写成固定值 `custom`。
- 贴图发现顺序包括模型目录中的 `Texture.png`、以模型名开头的 PNG，以及 `<ModelName>_Textures/` 中的 PNG。
- 开启自动加载后，插件会读取模型目录上一级 `Animations` 下的所有 `.blockyanim`。
- 附件按 `Collection` 单独导出；标有 `isPiece` 的节点在导入时挂到主体同名骨骼。

### 1b. 官方插件与 `.blockyanim`

```ts
const FPS = 60;
type IBlockyAnimJSON = { formatVersion: 1; duration: number; holdLastKeyframe: boolean;
                         nodeAnimations: Record<string, IAnimationObject> }
interface IAnimationObject { position?; orientation?; shapeStretch?; shapeVisible?; shapeUvOffset? }
```

- `duration` 和关键帧 `time` 的单位是帧，速率固定为 60 FPS。
- `holdLastKeyframe` 对应 Blockbench 的 `hold`；否则是循环动画。
- 通道包括 `position`、`orientation`、`shapeStretch`、`shapeVisible` 和 `shapeUvOffset`。
- 旋转使用四元数。
- 插值只输出 `smooth` 或 `linear`；Catmull-Rom 映射到 `smooth`，其他类型映射到 `linear`。
- `shapeUvOffset.y` 在导入导出时取反，数值取整。
- `shapeVisible` 是布尔值。

官方插件只负责导入、导出和 Blockbench 编辑器预览；它不提供 Character 运行时状态机。

### 2. 官方 Hytale 建模文章

- 只支持两种基本图元：六面体 Cube 和双面 Quad。
- 纹理可以非正方形，但宽高必须是 32 像素的倍数。
- Character/Attachment 使用 64px/unit；Prop/Block 使用 32px/unit。
- 官方建议单轴 stretch 大致保持在 `0.7x` 到 `1.3x`。
- Character 节点需要使用与动画系统兼容的名称。
- Hytale 使用 shading mode，而不是标准 PBR 工作流。
- 玩家贴图可以是灰度图，由游戏运行时动态着色。
- 插件仍处于 early access，官方文章明确提示可能存在缺陷。

### 3–6. 官方 Javadoc 与运行时动画

运行时 `protocol.Animation` 包含：

```text
name, speed, blendingDuration, looping, weight,
footstepIntervals, soundEventIndex, passiveLoopCount
```

运行时 `protocol.Model` 包含模型路径、纹理、渐变信息、相机、缩放、身体姿势偏移、`animationSets`、附件、碰撞盒等。

服务端 `ModelAsset.Animation` 还包含动画名称、播放速度、混合时长、循环标志、权重、脚步区间和声音事件。

**重要结论：** `speed`、`looping`、`blendingDuration`、`weight` 等不属于 `.blockyanim` 文件；它们属于服务端模型 JSON 的运行时动画配置，并通过动画名称引用 `.blockyanim`。

### 8–18. 其他来源确认的要点

- 模型通常不超过 255 个节点；节点计数包含组和形状，但主体组的首个 cube 会发生折叠。
- UV 必须匹配面尺寸，格式不支持任意自定义 UV 矩形尺寸。
- 动画使用 60 FPS、四元数插值和循环尾段回绕。
- `blockymodel-merger` 记录了 `assets/Characters/Player.blockymodel`、`Player_Textures/`、`Haircuts/`、`Body_Attachments/` 等资产树。
- Character 配置是 slot 到 `AccessoryId.Color.Variant` 的映射，例如 `bodyCharacteristic`、`haircut`、`underwear`、`cape`、`gloves`。
- `@hytale-tools/skin-viewer` 支持 `loop`、`fadeDuration`、`playbackRate` 和 `seek`，但这是该 viewer 的播放层，不等于官方客户端状态机规范。
- `blockymodel-merger` 的 `-pose` 只把动画第 0 帧应用为静态姿势；它不是运行时播放系统。

## (c) 与官方插件源码的冲突和缺口

### c.1 `hytale-docs.com` 的模型页面是错误格式

| 该页面声称 | 官方插件实际使用 |
|---|---|
| `format_version` + `model` 包装对象 | `{ nodes, format, lod }` |
| `model.identifier` | 没有 identifier 字段 |
| `texture_width` / `texture_height` 存在于模型 JSON | 模型文件不存图集尺寸 |
| `bones[]`、`pivot`、`cubes` | `nodes[]`、`position`、`orientation`、`shape` |
| `origin`、`inflate`、cube 级 mirror | `shape.offset`、`shape.stretch`、`textureLayout` |
| 每面 `[u1,v1,u2,v2]` | `{offset, mirror, angle}` |
| `visible_bounds_*`、`display.hand.*` | 不存在 |
| `mods/.../assets/models` | 游戏资产包中的 `Characters`、`Cosmetics`、`Blocks`、`Items` |

### c.2 只有插件源码中存在的事实

- `id` 计数器和额外 cube 的命名规则。
- `shape.offset` 的计算以及父节点偏移传递。
- `isStaticBox` 的节点折叠和 `isPiece` 附件语义。
- 精确的 Quad 法线、UV 旋转、镜像和透明面处理。
- 纹理自动发现规则。
- `../Animations/<folder>/*.blockyanim` 自动加载约定。
- `shapeUvOffset` 的 Y 取反和整数取整。
- 缺少 duration 时导出默认 120 帧。

### c.3 公开资料中存在、但不属于编解码器的事实

- 255 节点上限。
- 0.7–1.3x stretch 建议。
- 纹理宽高为 32 的倍数。
- 服务端模型 JSON 中的动画速度、混合、权重和循环配置。
- 运行时 `passiveLoopCount`。

### c.4 仍未被公开资料解决的歧义

- `format` 字段是否只影响导入器，还是会影响游戏运行时。
- `lod` 除 `auto` 外是否存在其他合法值。
- 运行时 `passiveLoopCount` 的真实含义。
- 节点计数在不同工具中的折叠口径。
- 服务端 `Assets/Server/Models/*.json` 的完整 JSON 结构。
- Character 状态机的状态、条件、优先级和混合策略。

## (d) 不存在的官方公开文档

### d.1 已确认缺失

- 没有 Hypixel Studios 发布的 `.blockymodel` / `.blockyanim` JSON Schema、IDL 或 RFC。
- `docs.hytale.com` 提供的是 Javadoc，而不是格式规范。
- 官方插件仓库没有完整 `docs/` 或 `SPEC.md`；主要事实位于源码和 `dist/about.md`。
- 社区资料转述过“正在准备公开创作者文档”，但当前没有完整官方技术文档。

### d.2 已验证不可用或不存在的页面

以下页面在调研时不可用或不在 sitemap 中：

- `https://www.blockbench.net/plugins/hytale_plugin`
- `https://hytale-docs.com/docs/modding/art-assets/blockymodel`
- `https://hytale-docs.com/docs/modding/art-assets/blockyanim`
- `https://hytale-docs.com/docs/tools/blockbench/file-format`
- `deepwiki.com` 对相关仓库的部分页面只有 JS 加载占位内容。

## (e) 对本仓库的结论

1. `.blockymodel` 和 `.blockyanim` 的格式事实以 `ref/hytale-blockbench-plugin` 为最高优先级；本仓库的两个 spec 文档应继续对齐它。
2. 官方公开资料足以确认模型格式、动画采样、附件和运行时动画参数的边界，但不足以重建完整 Character 状态机。
3. Character 的主体和附件应按 slot/registry/`isPiece` 组合，而不是按目录顺序猜测。
4. 播放器不应把 `.blockyanim` 文件列表当作官方状态机；文件顺序不是状态转移规则。
5. 循环状态如 `Sit`、`Fly` 应保持循环；一次性动作是否自动进入下一个状态，必须由外部状态配置决定，不能仅依据 `holdLastKeyframe` 推断官方客户端行为。
6. 要与官方客户端进一步保持一致，需要读取或建立等价的 `AnimationSet` 配置，支持状态名、动画名、速度、循环、混合时长和权重；状态选择则必须由移动、飞行、坐下、攻击等业务输入驱动。

