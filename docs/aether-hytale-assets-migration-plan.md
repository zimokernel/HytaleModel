# Aether 使用 Hytale 标准资产的可行性评估与开发计划

## 1. 结论

可行，但 `.vox` 与 Hytale `.blockymodel/.blockyanim` 应被视为服务不同渲染管线的两套资产格式，而不是互相替换的格式。

建议采用以下目标：

- Hytale 标准资产用于 Aether 的主角色、生物和 NPC。
- 地形结构、植物/装饰 Sprite、碰撞体等资产严格保留原始 `.vox` 格式和原有体素管线。
- 武器以及与生物/NPC 交互或被 NPC 拿持的物品，优先使用 Hytale 标准资产。
- 不会被拿到手上、作为独立物理物件表达的物品，优先继续使用 `.vox`；具体资产按渲染、物理和交互需求评估。

因此目标不是全量废弃 `.vox`，而是建立两条职责清晰、长期并存的渲染管线。

## 2. 当前基础

HytaleModel 已经具备独立的 Hytale 模型运行时基础：

- `.blockymodel` 解析、骨骼展开、绑定姿态和包围盒计算。
- Cube/quad 网格生成、UV 映射、法线和双面材质处理。
- `.blockyanim` 动画采样、循环、尾帧保持和未知骨骼忽略。
- 独立的 wgpu 播放器和真实 `Pets/Dog` 资产测试。

移植清单已经按照 Aether 的 `vek`、`wgpu` 和渲染约定编写，见 [`porting-to-aether-terrain.md`](./porting-to-aether-terrain.md)。

Aether 当前的 Figure 体系仍然是体素专用：

- `BodySpec` 返回固定 16 个 `(Segment, offset)` 部件，见 `Aether_terrain/vrage/src/scene/figure/load.rs`。
- 模型生成依赖 `Segment` 和 greedy meshing，见 `Aether_terrain/vrage/src/scene/figure/cache.rs`。
- 角色管线使用体素颜色/光照图集，见 `Aether_terrain/vrage/render/src/pipelines/figure.rs`。
- 动画骨骼上限为 16，见 `Aether_terrain/vrage/anim/src/lib.rs`。
- 资产系统目前直接注册 `.vox`，见 `Aether_terrain/common/assets/src/lib.rs`。

Hytale 标准 Dog 资产已经有 31 个节点，因此当前 16 骨骼限制不能直接使用 Hytale 角色资产。HytaleModel 的格式上限是 255 个节点，见 [`dog_asset.rs`](../crates/blockymodel/tests/dog_asset.rs)。

## 3. 适用范围

### 3.1 适合优先迁移的资产

- 玩家角色和人形 NPC。
- 动物、怪物等带骨骼的生物。
- 武器、护甲、背包、翅膀等与生物/NPC 绑定或被拿持的装备附件。
- 需要 Idle、Walk、Run、Attack、Death 等动画的生物或 NPC 物品。

### 3.2 严格保留 `.vox` 的资产

- 多体素建筑和场景结构。
- 地形植物和 Sprite manifest 中的体素模型。
- 直接参与体素碰撞或体积运算的模型。
- 需要按体素占用进行编辑、合并或布尔操作的模型。

### 3.3 需要逐项评估的资产

- 独立掉落物。
- 非持握武器展示物。
- 不会被拿到手上、以独立物理物件表达的物品，默认使用 `.vox`。
- 具备实体碰撞但不参与体素编辑的装饰物。
- 依赖调色板索引、材质索引或 hollowing 语义的模型。

评估维度包括：是否需要骨骼动画、是否需要挂接到生物骨骼、是否参与物理/体素运算、是否需要调色板重染色，以及两种管线的运行时成本。

## 4. 目标架构

```text
AssetManager
├── BlockyModelAsset       .blockymodel + Texture.png
├── BlockyAnimationAsset   .blockyanim
└── VoxelAsset             .vox，长期保留

FigureRenderer
├── BlockyModelPipeline    Hytale 骨骼模型
└── VoxelPipeline          .vox，长期保留

FigureModelCache
├── BlockyModelCache
└── VoxelCache
```

BlockyModel 应成为 Figure 的新数据模型，而不是先转换成 `Segment` 再进入旧的体素网格流程。这样可以保留：

- Hytale 原生 UV 和 PNG 纹理。
- Hytale 骨骼层级和动画语义。
- Cube/quad 的几何和双面属性。
- 骨骼附件和按骨骼名绑定的装备模型。

## 5. 开发计划

### 阶段 0：资产契约和样板验证

1. 定义 Hytale 资产目录和 manifest 格式。
2. 将 `.blockymodel`、`.blockyanim` 和 PNG 注册到 Aether 资产系统。
3. 在 Aether 中建立独立模型调试场景。
4. 接入 `Pets/Dog` 作为第一个标准资产。

验收标准：

- Dog 绑定姿态尺寸正确。
- 贴图方向、UV 和透明区域正确。
- Idle、Walk、Death 动画正确播放。
- 资产可热重载，加载失败有明确占位模型和错误日志。

### 阶段 1：升级骨骼数据容量

1. 将 `MAX_BONE_COUNT` 从 16 提升到至少 255。
2. 修改所有 `[FigureBoneData; 16]`、`[Option<BoneMesh>; 16]` 等固定数组。
3. 检查 wgpu uniform 缓冲区上限和对齐规则。
4. 保留骨骼名称到索引的映射，支持重复骨骼名。
5. 对动画引用但模型不存在的骨骼进行静默忽略并记录诊断信息。

验收标准：

- 31 骨骼 Dog 可以完整上传和渲染。
- 255 骨骼上限有独立单元测试或容量测试。
- 现有 Aether 动画和角色行为不回归。

### 阶段 2：引入 Hytale 运行时资产

1. 将 `crates/blockymodel` 移入 Aether 的 `vrage/anim/blockymodel/`。
2. 增加 `.blockymodel` / `.blockyanim` 的 `FileAsset` 实现。
3. 接入 Aether 已有的 `AssetHandle`、缓存和热重载机制。
4. 统一 Hytale 坐标系、单位换算、根节点变换和包围盒。
5. 将 HytaleModel 的真实资产测试迁移到 Aether。

### 阶段 3：建立 BlockyModel 渲染管线

新增 `BlockyModelPipeline`，顶点至少包含：

- position
- normal
- UV
- bone index
- unlit/shading 信息

渲染管线需要：

1. 使用 Hytale PNG 纹理，不转换成体素颜色图集。
2. 支持 nearest 采样和 alpha cutout。
3. 支持 `flat`、`fullbright`、`standard`、`reflective` 的最小语义。
4. 复用 Aether 全局光照、阴影和相机。
5. 将动画采样结果上传到骨骼 uniform 或 storage buffer。

### 阶段 4：接入 Hytale 角色模型系统

1. 将 Humanoid 的身体、头部和 NPC 生物模型接入 BlockyModel 主模型加附件。
2. 按骨骼名称实现武器、护甲、背包和翅膀挂接。
3. 扩展 `FigureKey`，纳入模型 ID、装备组合、材质变体和动画状态。
4. 将被生物/NPC 拿持或与其动画绑定的武器、装备和物品接入 BlockyModel。
5. 对独立物理物品保留 `.vox` 实现，并建立按资产类型选择管线的规则。
6. Ship 和 Volume 暂不纳入首个里程碑。

### 阶段 5：动画状态适配

保留 Aether 现有状态机作为决策层，Hytale 动画文件只负责骨骼采样。

建立以下状态到 clip 名称的映射：

- idle
- walk
- run
- jump
- fall
- attack
- hurt
- death

同时处理循环、`holdLastKeyframe`、播放速度、混合和未知骨骼。

### 阶段 6：分批接入并固定资产边界

接入顺序建议为：

1. 玩家、人形 NPC 和非人形生物的 Hytale 标准资产。
2. 与生物/NPC 绑定或被拿持的武器、护甲和物品。
3. 对独立物理物品逐项评估，默认保留 `.vox`。
4. 地形结构、植物/装饰 Sprite 和碰撞体继续使用 `.vox`，不进行格式替换。
5. 删除“Legacy”命名，分别稳定维护 `BlockyModelPipeline` 和 `VoxelPipeline`。

## 6. 主要风险

### 6.1 当前 Figure 管线不能直接复用

它依赖体素网格、颜色图集和 greedy meshing。Hytale 模型需要真实 UV 纹理、骨骼索引和独立网格，因此必须新增或重构渲染管线。

### 6.2 16 骨骼上限是硬阻塞

Dog 已经超过当前上限。骨骼容量升级必须先于正式角色迁移。

### 6.3 装备和物品需要分类

Aether 当前依赖多个 `.vox` 部件、偏移和调色板重染色。被生物/NPC 拿持或绑定的装备适合通过 Hytale 骨骼附件、纹理变体和模型 manifest 组合；独立物理物品则应优先保留 `.vox`，避免为了统一格式而损失体素碰撞和物理语义。

### 6.4 两套格式需要长期并存

Aether 的 `.vox` 用于结构、Sprite、体积和部分碰撞相关逻辑；Hytale BlockyModel 用于角色、生物和骨骼动画。两者应分别拥有资产加载、缓存、网格生成和渲染管线，不应将其中一方包装成另一方的兼容层。

### 6.5 资产授权

Hytale 插件的许可证不等于 Hytale 游戏资产的再分发授权。若 Aether 要随项目发布这些标准资产，必须先确认合法使用和再分发范围。

## 7. 最终建议

采用“BlockyModel 服务角色/生物/NPC，Voxel 服务体素结构/植物/装饰/碰撞，两条管线长期并存”的路线。

第一阶段先完成 Dog 端到端渲染、255 骨骼支持和 Hytale 角色管线；随后迁移 Humanoid、生物 NPC 以及与其绑定或被拿持的武器和物品。地形结构、植物/装饰 Sprite、碰撞体和独立物理物品继续使用 `.vox`，不以删除 `dot_vox` 为项目目标。
