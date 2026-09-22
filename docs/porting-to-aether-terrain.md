# 移植到 Aether_terrain

本文的目标：把本仓库的两块代码搬进 `D:\work\ttc\Aether_terrain`，**不需要改数学类型、不需要升/降 wgpu 版本**。

因为两边已经对齐，移植主要是「接进它们既有的约定」，而不是「适配 API」。

---

## 1. 已经对齐的部分

| 项 | 本仓库 | Aether_terrain | 状态 |
|---|---|---|---|
| 工具链 | `nightly-2026-06-13` | `nightly-2026-06-13` | ✅ 相同 |
| edition | 2024 | 2024 | ✅ |
| `wgpu` | `27` | `27.0.1`（`yakui-wgpu` 锁定） | ✅ |
| `winit` | `0.30.12` | `0.30.12` | ✅ |
| `vek` | `0.17.1` + `repr_c` | `0.17.1` + `repr_c` | ✅ 类型同一 |
| 矩阵表示 | `mat::repr_c::column_major::Mat4<f32>` | 同 | ✅ |
| `bytemuck` | `1.7` + derive | 同 | ✅ |
| `image` | `0.25` + png | 同 | ✅ |
| 骨骼数据 | uniform 数组，`[[f32;4];4]` × 2 | `figure::BoneData` | ✅ 同构 |

因为用 `repr_c` 的显式路径，`Skeleton` 产出的 `Vec3<f32>` / `Quaternion<f32>` / `Mat4<f32>` 与 `vrage_anim::vek::*` 是**同一个类型**，不需要任何 `.into()`。

---

## 2. 落位建议

```
D:\work\ttc\Aether_terrain\
├── vrage/anim/blockymodel/          ← crates/blockymodel 搬到这里
│   ├── Cargo.toml                   （name = "vrage_blockymodel"）
│   ├── src/{json,math,model,anim,skeleton,mesh,lib}.rs
│   └── tests/dog_asset.rs           （资产路径要改，见 §4）
└── vrage/render/src/pipelines/
    └── blockymodel.rs               ← crates/player/src/renderer.rs 改写而来
```

`blockymodel` 放进 `vrage/anim/` 下而不是 `vrage/render/` 下，因为它**不含任何 GPU 类型**，和 `vrage_anim` 是同层的纯数据/数学代码。

---

## 3. 逐步清单

### 3.1 搬 `blockymodel` crate

1. 复制 `crates/blockymodel/src` 到 `vrage/anim/blockymodel/src`。
2. `Cargo.toml` 改成工作区风格：

```toml
[package]
name = "vrage_blockymodel"
version.workspace = true
edition.workspace = true
publish.workspace = true

[dependencies]
vek = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
bytemuck = { workspace = true, features = ["derive"] }
thiserror = "2"

[lints]
workspace = true
```

3. 加进根 `Cargo.toml` 的 `members`：`"vrage/anim/blockymodel"`。
4. `math.rs` 可以直接删掉，改成复用它已有的 `vrage/anim/src/vek.rs`：

```rust
use crate::anim::vek::{Mat4, Quaternion, Vec2, Vec3};
```

   `min3` / `max3` / `normalize_or_zero` / `recip_or_zero` 这 4 个小工具留下来即可（它们只是组件级操作）。
5. `lib.rs` 里对 `vek` 的再导出删掉，避免两个 crate 各自导出同名类型。

### 3.2 改写渲染管线

`crates/player/src/renderer.rs` → `vrage/render/src/pipelines/blockymodel.rs`，按 `pipelines/figure.rs` 的骨架重排：

| 本仓库 | 目标项目写法 |
|---|---|
| `create_bind_group_layout(...)` 裸调 | `FigureLayout` / `LocalsLayout` 风格：把布局收进一个 `struct BlockyModelLayout` |
| 手搓 `wgpu::Buffer` + `write_buffer` | `Consts<T>` + `Bound<T>`（见 `vrage/render/src/consts.rs`、`bound.rs`） |
| `BoneData`（本仓库定义） | 直接换成 `crate::pipelines::figure::BoneData` |
| 单独的 `camera_buffer` | 用它们全局的 `globals` / `Locals::model_mat` |
| `Mat4::perspective_rh_zo` + `look_at_rh` | 用 `vrage_render` 已有的相机；不要自己再建一套 |

**`BoneData` 的字段裁剪**：本仓库的 `BoneData` 比目标多一个 `uv_offset: [f32;4]`（为了支持 `shapeUvOffset` 通道）。
不需要这个通道的话，直接删掉该字段用目标版本即可 —— 注意 **`bone_mat` / `normals_mat` 的语义完全相同**，一个字节都不用改。

> ⚠️ 如果你决定保留 `uv_offset`，**必须保持它是 `vec4`**。WGSL/GLSL 的 uniform 布局都会把结构体成员顶到 16 字节边界，写成 `vec2` 会让 CPU 与 GPU 的结构体大小不一致。见 `docs/blockymodel-format.md` §12.7。

### 3.3 着色器：WGSL → GLSL

目标项目用 `shaderc`（`wgpu` 特性 `["spirv", "glsl"]`）。转换是机械的：

```glsl
// layout (set = 3, binding = 1) uniform u_bones { BoneData bones[255]; };
struct BoneData {
    mat4 bone_mat;
    mat4 normals_mat;
    vec4 uv_offset;
};

vec4 world = bones[in.bone].bone_mat * vec4(in.position, 1.0);
vec3 n     = (bones[in.bone].normals_mat * vec4(in.normal, 0.0)).xyz;
vec2 uv    = in.uv + bones[in.bone].uv_offset.xy;
```

数学约定一一对应：列主序矩阵、`M * v`、`Mat4::into_col_arrays()` 与 GLSL `mat4` 的内存布局一致。

### 3.4 收尾

1. 删掉本仓库 `Cargo.toml` 里重复的 `wgpu` / `winit` / `vek` / `bytemuck` / `image` / `serde` 版本声明。
2. `camera.rs` 丢掉（用目标项目的相机）。
3. `assets.rs` / `app.rs` / `main.rs` 是播放器外壳，不需要移植。
4. 格式化：`cargo +nightly fmt --all`（本仓库已经按其 `rustfmt.toml` 格式化过）。

---

## 4. 测试

`crates/blockymodel/tests/dog_asset.rs` 里有 13 个针对真实资产的断言，**建议一起搬过去**。它钉住的是最容易在后续重构中悄悄坏掉的东西：

- bind pose 包围盒（父 `shape.offset` 传播规则）
- `R-Eye` 这个 quad 的 UV 矩形精确到 1/8 像素
- 148 个面的绕序与声明法线一致
- 循环动画首尾姿势接缝 < 1e-3
- `holdLastKeyframe` 的尾段保持最后一帧
- 动画引用不存在的骨骼时不 panic
- 所有骨骼矩阵都是有限值

唯一要改的是资产路径。目前是：

```rust
Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(relative)
```

搬到 `vrage/anim/blockymodel/` 后深度变成 3 层，或者更稳妥地改成从环境变量取：

```rust
fn asset(relative: &str) -> PathBuf {
    std::env::var_os("HYTALE_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .join(relative)
}
```

---

## 5. 验收

移植完成后，最小验收路径：

1. `cargo check --workspace` 通过。
2. 原来的 13 个资产测试通过。
3. 在它们的调试场景里挂一个 `Model.blockymodel`，确认：
   - 狗的样子对（贴图不糊、不镜像、不旋转）；
   - 切到 `Walk` 时四条腿交替摆动；
   - 切到 `Death` 时结束后停在倒地姿势（不回弹）。
