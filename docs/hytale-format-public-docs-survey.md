# Public / authoritative documentation for `.blockymodel`, `.blockyanim` and the Hytale Blockbench workflow

**Survey date:** 2026-09-22
**Scope:** every publicly reachable source that makes factual claims about the two file formats or the Blockbench asset workflow, with an explicit list of documentation that does **not** exist.
**Relationship to this repo:** `docs/blockymodel-spec.md`, `docs/blockyanim-spec.md` and `ref/hytale-blockbench-plugin/` are the primary ground truth. This survey exists to (1) record the *public* evidence trail, (2) flag where public pages contradict the plugin source, and (3) prove the absence of an official published spec.

---

## 0. Method, and what could not be reached

* 10 `web_search` calls (≈35 distinct queries) and ~35 page fetches.
* `raw.githubusercontent.com` **does not resolve** from this sandbox (`getaddrinfo ENOENT`). Two working substitutes were used:
  * `api.github.com/repos/<r>/contents/<path>` → JSON with `base64` `content` (unusable for large files without a decoder step);
  * `https://cdn.jsdelivr.net/gh/<owner>/<repo>@<ref>/<path>` → **plain text**, works for `.ts` and `.md` (used for the plugin source and for `timiliris/Hytale-Docs`).
* `hytale-docs.com` is client-rendered: every page fetch returns only the `<title>` plus `Skip to main content`. Its **source markdown is public** in [`timiliris/Hytale-Docs`](https://github.com/timiliris/Hytale-Docs) (default branch `master`), which is what I quote below. `/sitemap.xml` is also fetchable and gives the complete page inventory.
* `blockbench.net/plugins/hytale_plugin` returned **404** through the fetch tool on this run (the URL linked from the official Hytale blog post). Treat the "official plugin blurb" as sourced from `dist/about.md` + Hytale Wiki instead.
* No page's content was treated as instructions; all external content below is quoted as data.

---

## (a) Source inventory — authority, one line each

### Tier 1 — First-party / Hypixel Studios

| # | URL | Authority |
|---|---|---|
| 1 | https://github.com/JannisX11/hytale-blockbench-plugin | **De-facto normative spec.** Official plugin; every source file is `Copyright (C) 2025 Hypixel Studios Canada inc.` `src/blockymodel.ts` (`compile()`/`parse()`) and `src/blockyanim.ts` (`compileAnimationFile()`/`parseAnimationFile()`) define the formats. |
| 2 | https://hytale.com/news/2025/12/an-introduction-to-making-models-for-hytale | **Official first-party art guide** (Thomas Frick, art director; posted 2025-12-22). The only Hypixel-authored prose about the modelling pipeline. |
| 3 | https://docs.hytale.com/api/com/hypixel/hytale/protocol/Animation | Official Javadoc for the runtime animation *packet* class (not the file format). |
| 4 | https://docs.hytale.com/api/com/hypixel/hytale/protocol/Model | Official Javadoc for the runtime model *packet* class. |
| 5 | https://docs.hytale.com/api/com/hypixel/hytale/server/core/asset/type/model/config/ModelAsset | Official Javadoc for the **server-side model definition asset** (`Assets/Server/Models/*.json`) that points at the `.blockymodel`. |
| 6 | https://docs.hytale.com/api/com/hypixel/hytale/server/core/asset/type/model/config/ModelAsset.Animation | Official Javadoc for the per-animation entry of that server JSON (`id`, `animation`, `speed`, `blendingDuration`, `looping`, `weight`, `footstepIntervals`, `soundEventId`). |
| 7 | https://hytale.com/news/2025/11/hytale-modding-strategy-and-status | Cited by Hytale Wiki: the team is *"working on public creator documentation hosted on GitBook"* but none exists yet. |
| 8 | `ref/hytale-blockbench-plugin/dist/about.md` (mirrors the plugin's in-app "about" text; also republished on https://hytalewiki.org/w/Blockbench) | Official plugin feature/guideline text, **including the 255-node rule and the node-counting rule**. |

### Tier 2 — Community documentation that is widely treated as canonical

| # | URL | Authority |
|---|---|---|
| 9 | https://hytalewiki.org/w/Blockbench | Community wiki "Adapted from the plugin's description"; adds history/dates. |
| 10 | https://hytalewiki.org/w/Technical:Documentation | Explicitly states *"there is currently no official technical documentation"* (ref. #7). |
| 11 | https://hytalewiki.org/w/Animation | Community wiki; *"all of this information takes place in 60 frames a second"* + per-weapon frame counts. |
| 12 | https://github.com/hytale-tools/blockymodel-merger | Community Go toolchain (merge, render, GLB export, static pose from frame 0). Documents **asset-tree and texture-naming conventions**. |
| 13 | https://doctale.dev/tutorials/blockbench/setup/ (+ `/asset-development/blocks/animations/`, `/asset-development/npcs/models/`) | Large community server-modding docs (ItsRiprod). Contains the 255-node / 32-64px / 0.7–1.3× table. LLM-assisted, secondary. |
| 14 | https://hytalemodding.dev/docs/guides/plugin/animated-block-textures (source: `HytaleModding/site`) | Community docs; contains a **real, runnable example** wiring a `.blockyanim` into a block JSON. |
| 15 | https://github.com/jasonjgardner/blockbench-mcp-project → `skills/blockbench-hytale/SKILL.md` | Agent-oriented operational reference written against the installed plugin ("Technical references checked 2026-09-13"). Rich, but explicitly second-hand. |

### Tier 3 — Third-party reimplementations that publish a schema

| # | URL | Authority |
|---|---|---|
| 16 | https://www.npmjs.com/package/blockymodel-web (+ `src/types/blockymodel.ts`, `https://cdn.jsdelivr.net/npm/blockymodel-web/src/types/blockymodel.ts`) | MIT TypeScript interface set for `.blockymodel` by ZacxDev (`hytalegarage`), MIT. The most complete *published* schema-like artefact for the model format. |
| 17 | https://www.npmjs.com/package/blockymodel-texture | Companion package; documents the UV algorithm and states it is *"Based on Hytale's Blockbench Plugin implementation"*. |
| 18 | https://www.npmjs.com/package/@hytale-tools/skin-viewer (+ `dist/types-*.d.mts`) | GPL-3.0 viewer; ships **the clearest published `.blockyanim` interface** (`BlockyAnimData`, `Keyframe<T>`, channel list) plus playback semantics. |
| 19 | https://github.com/ZacxDev/blockymodel-web · `blockymodel-prefab` | Same author's related tooling (prefab renderer). |
| 20 | https://hytl.tools / https://hytl.skin (via #12 and #18) | Hosted renderers; the merger's README links `https://hytl.skin/` as an online/API front-end. |

### Tier 4 — Pages that *look* authoritative about the format but are not

| # | URL | Why it is listed |
|---|---|---|
| 21 | https://hytale-docs.com/docs/modding/art-assets/models (source: `timiliris/Hytale-Docs@master:content/docs/en/modding/art-assets/models.md`) | **Contradicts the real format in every field name** — see §c.1. Most-searched result for "Hytale model format documentation". |
| 22 | https://hytale-docs.com/docs/modding/art-assets/textures | Repeats the official blog plus unverified additions (512×512 cap, emissive textures, animated-texture JSON). |
| 23 | https://hytale-docs.pages.dev (repo `vulpeslab/hytale-docs`) | A *different* unofficial docs site. Has **no** model-format page; `/modding/npc-ai/animations/` is about the server-side runtime animation system only. |

---

## (b) Concrete facts each source states

### 1. `hytale-blockbench-plugin` — `.blockymodel` (`src/blockymodel.ts`)

Type declarations as written in the source:

```ts
type BlockymodelJSON = { nodes: BlockymodelNode[]; format?: string; lod?: 'auto' }
type BlockymodelNode = {
  id: string; name: string;
  position: IVector; orientation: IQuaternion;
  shape?: {
    offset: IVector; stretch: IVector;
    textureLayout: Record<string, IUvFace>;
    type: 'box' | 'none' | 'quad';
    settings: { size?: IVector; normal?: QuadNormal; isPiece?: boolean; isStaticBox?: true };
    unwrapMode: "custom"; visible: boolean; doubleSided: boolean;
    shadingMode: 'flat' | 'standard' | 'fullbright' | 'reflective';
  }
  children?: BlockymodelNode[]
}
type IUvFace  = { offset:{x,y}; mirror:{x,y}; angle: 0|90|180|270; transparent?: boolean; lockUVs?: boolean }
type QuadNormal = '+X'|'+Y'|'+Z'|'-X'|'-Y'|'-Z'
type IVector = {x,y,z}; type IQuaternion = {x,y,z,w}
```

Facts derivable from `compile()` / `parse()`:

* Top level is always `{ nodes, format, lod }`; `format` is written as `'prop'` for `Formats.hytale_prop`, otherwise `'character'`; `lod` is always `'auto'`.
* `id` is a **stringified counter starting at `"1"`**, incremented per emitted node (not a UUID).
* `name` is `element.name` with any `namespace:` prefix stripped: `name.replace(/^.+:/, '')`. Extra cubes inside a group get `--C1`, `--C2` … suffixes (and the parser strips `--C\d+$` again).
* `position` = element origin **relative to the parent's origin and minus the parent's main-shape offset**; quaternion `orientation` is built from the Blockbench euler rotation and parsed back with euler order `'ZYX'`.
* `shape.offset` = the **centre of the main cube minus the element origin** (`getNodeOffset`); a group with no cube falls back to `group.original_offset`; leaf cubes get `[0,0,0]`.
* `shape.settings.size` = the cube's **base** size; `shape.stretch` is stored separately (`[1,1,1]` default) — this is what makes the "stretch without changing UV" behaviour possible.
* Quads: `type:'quad'`, `settings.normal` derived from which face has a texture (`west→-X, east→+X, down→-Y, up→+Y, north→-Z, south→+Z`), `size.z` is deleted and folded into `size.x`/`size.y`; a quad's UV is always written to the `front` entry.
* `settings.isStaticBox = true` when the cube *is* the group (node folding); on import such a node does **not** create a Blockbench group.
* Face-key mapping Blockbench → Hytale: `north→back, south→front, west→left, east→right, up→top, down→bottom`.
* UV: `offset` is `(min(u1,u3), min(v1,v3))`; `mirror.x` when `u1>u3`, `mirror.y` when `v1>v3`; Blockbench `rotation` maps **90→270, 180→180, 270→90** with conditional extra flips; `lockUVs` from the face's `uv_lock`; `transparent` from the face's `transparent`. All UV numbers are `Math.round()`ed.
* `unwrapMode` is only ever written as the literal `'custom'`.
* `isPiece` is set from the group's custom `is_piece` flag (attachments).
* **Texture discovery on import:** PNGs in the model's directory whose filename `startsWith(modelName)` **or** equals `Texture.png`, plus every PNG in `<modelName>_Textures/`.
* **Format detection on import:** `model.format == 'prop'` → prop format; otherwise if any path segment is `Blocks` → prop format.
* **Animation auto-load:** with the `auto_load_hytale_animations` setting on, entering Animate mode loads every `*.blockyanim` found in `<model dir>/../Animations/<subfolder>/`.
* Attachment export is per-`Collection` with `collection.export_codec == "blockymodel"`; nodes marked `isPiece` are attached to same-named bones of the base model on import.

### 1b. `hytale-blockbench-plugin` — `.blockyanim` (`src/blockyanim.ts`)

```ts
const FPS = 60;
type IBlockyAnimJSON = { formatVersion: 1; duration: number; holdLastKeyframe: boolean;
                         nodeAnimations: Record<string, IAnimationObject> }
interface IAnimationObject { position?; orientation?; shapeStretch?; shapeVisible?; shapeUvOffset? }  // IKeyframe[]
interface IKeyframe { time: number; delta: {x,y,z,w?} | boolean;
                      interpolationType?: 'smooth' | 'linear' }
```

* `formatVersion` is the literal `1`.
* `duration` is in **frames**; the editor animation length is `duration / FPS` and snapping is set to `FPS`; on export `duration = Math.round(animation.length * FPS) || FPS * 2` (default 2 s = 120 frames).
* `holdLastKeyframe` = `animation.loop == 'hold'`; import maps it back to loop mode `hold` vs `loop`.
* Keyframe `time` is an **integer frame number**: export `Math.round(kf.time * FPS)`, import `kf_data.time / FPS`.
* Channel names (Hytale) vs Blockbench: `position`→`position`, `orientation`↔`rotation`, `shapeStretch`↔`scale`, `shapeVisible`↔`visibility`, `shapeUvOffset`↔`uv_offset`.
* `orientation` deltas are **quaternions** `{x,y,z,w}` (Blockbench euler order via `Format.euler_order`).
* `interpolationType` is **only `'smooth'` or `'linear'`**: export maps `catmullrom → 'smooth'`, *everything else → `'linear'`*; import maps non-`smooth` → `linear`.
* `shapeUvOffset` deltas are `{x, y}` and **`y` is negated** on both export and import; values are rounded to integers.
* Export always materialises a `shapeUvOffset: []` array for any animator that has data.
* `shapeVisible` deltas are plain booleans.
* Export action id `export_blockyanim`; drag-and-drop / import filter is `extensions: ['blockyanim']`.

### 2. Official Hytale blog post (hytale.com, 2025-12-22)

* *"Today, we are so happy to finally share some tips and tricks to get you started on making models in the Style of Hytale."*
* Two primitives only: *"Cubes (6 sides)"* and *"Quads (2 sides)"*. *"No edge loops, no special topology, no triangles, pyramids…"* and *"**No spheres allowed!**"*
* *"Textures can be non-square and must be multiples of 32px (32, 64, 96, 128, etc.)"*
* Density decision: *"Make a Character/Attachment (Cosmetic, Tool, Weapon, Food item) = Density will be 64px per unit"* / *"Make a Prop/block (anything else from cubes to furniture) = Density will be 32px per unit"*; *"the engine automatically scales characters/attachments down to match the world size."*
* Stretch: *"we avoid going under 0.7X and over 1.3X the stretch for a node in one axis."*
* *"When creating characters, we carefully name each node to make it compatible with our animation system."*
* Shading: *"we work with a tight set of material types. We call them shading modes. Blockbench isn't able to display these shading modes yet, but you are able to set them and export them for Hytale, for each node of your model!"*
* No PBR: *"We aren't using the industry standard PBR workflows (roughness, normal maps, displacement, etc.)"*
* *"If you're wondering why the player texture is grey, it's because we dynamically tint it in-game!"*
* Explicitly early access: *"Please note that this plugin is also in early access, and you might encounter bugs."*
* Downloads: Blockbench, plugin, plugin source, and a model-samples archive at `https://cdn.hytale.com/Hytale%20Model%20Examples.zip`.

### 3–6. Official Javadoc (docs.hytale.com)

`protocol.Animation` (runtime packet) fields: `String name` (nullable), `float speed`, `float blendingDuration`, `boolean looping`, `float weight`, `int[] footstepIntervals`, `int soundEventIndex`, `int passiveLoopCount`; constants `NULLABLE_BIT_FIELD_SIZE`, `FIXED_BLOCK_SIZE`, `VARIABLE_FIELD_COUNT`, `VARIABLE_BLOCK_START`, `MAX_SIZE`.

`protocol.Model`: `assetId`, `path`, `texture`, `gradientSet`, `gradientId`, `camera`, `scale`, `eyeHeight`, `crouchOffset`, `sittingOffset`, `sleepingOffset`, `animationSets : Map<String, AnimationSet>`, `attachments`, `hitbox`, `particles`, `trails`, `light`, `detailBoxes`, `phobia`, `phobiaModel`.

`server...ModelAsset` (the server JSON asset): getters for `Model`, `Texture`, `GradientId`, `GradientSet`, `EyeHeight`, `CrouchOffset`, `SittingOffset`, `SleepingOffset`, `AnimationSetMap`, `Camera`, `BoundingBox`, `Light`, `Particles`, `Trails`, `PhysicsValues`, `DefaultAttachments`, `RandomAttachmentSets`, `MinScale`/`MaxScale`, `IconProperties`, `Icon`, `DetailBoxes`, `Phobia`. Nested `ModelAsset.Animation` carries `animation` (the animation *name*), `speed`, `blendingDuration`, `looping`, `weight`, `footstepIntervals`, `soundEventId` and `toPacket()` → `protocol.Animation`.

⇒ **Consequence:** `speed`, `looping`, `blendingDuration`, `weight`, footstep intervals and sound events are **not** part of `.blockyanim`; they live in the server-side model JSON and are attached by animation *name*.

### 8 / 9. Plugin "about" text and Hytale Wiki

* *"Models should not have more than 255 nodes. Nodes are a concept in the Hytale model format, each group counts as a node, and each cube/quad counts except if it's the first cube in a respective group."* Node count is visible by clicking the element counter above the outliner.
* *"As a size reference, the center grid in a new Hytale project represents one block in-game. This measures either 32 pixels for props and blocks or 64 pixels for characters and attachments."*
* *"UVs must always match the dimensions of their face, the format does not consider custom UV sizes."*
* Node/shape semantics: *"Hytale uses 'Nodes', which are a combination of a group (for the transformation and hierarchy) and a cube (for the visual part). Position and Rotation animations target the group… However, Scale, Visibility and UV Offset only target the Shape itself… This cube gets exported as the shape of a node and is targeted by its animations and not counted as an extra node towards the node count limit."*
* Animation features: visibility keyframes, **UV Offset keyframes**, quaternion-based interpolation, keyframe-wrapping for looping animations; attachments built on collections.
* Wiki-only facts: automatic UV sizes locked to face dimensions; stretch tool adjusts cube size *without* affecting UV; plugin history dates (2025-11-17 collaboration announced; 2025-11-26 JannisX11 clarifies modelling details on X; 2025-12-22 plugin released with the blog post).

### 11. Hytale Wiki `Animation`

* *"Note that all of this information takes place in 60 frames a second."*
* Frame budgets: weapon switch = 1 frame; return to idle ≈ 12 frames; shortsword attacks ≈ 24/26/25; axe ≈ 40/39; warhammer ≈ 23/45.

### 12. `blockymodel-merger` (Go) — layout & naming conventions

* Texture naming: each item folder contains `X.blockymodel` plus the matching **`X_Texture.png`**; `-texture` overrides for variants that reuse another model's texture (e.g. `Adamantite_Triple.blockymodel` uses `Adamantite_Texture.png`).
* Asset tree after extraction: `assets/Characters/Player.blockymodel`, `assets/Characters/Player_Textures/`, `Haircuts/`, `Body_Attachments/`, `assets/Cosmetics/`, `assets/TintGradients/`, `assets/BlockTextures/`, `assets/Blocks/`, `assets/Items/` (Armors/, Weapons/, Tools/, Consumables/, Trinkets/, Deployables/, Vehicles/, Projectiles/, Instruments/, Ingredients/, Torch/…).
* Registry data in `data/`: `Haircuts.json`, `Faces.json`, `Eyes.json`, `Pants.json`, `Overalls`/`Overtops`-style clothing files, `GradientSets.json`; source is the **server** `assets.zip` (`Common/Characters`, `Common/Cosmetics`, `Common/TintGradients`, `Common/BlockTextures`, `Common/Blocks`, `Cosmetics/CharacterCreator`, `Server/Item/Items`).
* `-pose <file.blockyanim>` *"applies any animation's first frame as a static pose"* ⇒ frame 0 = a static pose is a shared convention.
* Character config JSON is a flat map of slot → `"AccessoryId.Color.Variant"` (e.g. `bodyCharacteristic`, `haircut`, `undertop`, `overtop`, `cape`, `gloves`, …), matching the in-game `UserData/CachedPlayerSkins/*.json`.
* Output: `<name>.glb`, `<name>.blockymodel`, `<name>_atlas.png`.
* Footnote in-repo: *"This project is provided as-is. No support or assistance will be provided."*

### 13. Doctale (community server docs)

* Constraint table: max nodes 255 (*"Includes cubes, groups, and bones"*); geometry *"Cubes, Flat Quads"*; texture sizes multiples of 32; grid scale 32px props / 64px characters; *"Geometry Stretching 0.7x - 1.3x — Limited scaling per axis"*.
* Block animations live in `Assets/Common/Blocks/Animations/<Category>/<Name>.blockyanim` (real examples listed: `Door/Door_Open_In.blockyanim`, `Chest/Chest_Open.blockyanim`, `Candle/Candle_Burn.blockyanim`, `Fire/Fire_Burn.blockyanim`, …).
* NPC model definitions live in `Assets/Server/Models/` and *"link to client-side `.blockymodel` and `.blockyanim` files"*; they carry `HitBox`, `Camera`, `IconProperties`, `Attachments`, `AnimationSets`.
* Workflow: `File > New > Hytale Model`, props 32px = 1 block, characters 64px = 1 block, *"Only cubes and flat quads are supported"*, *"Maximum 255 nodes per model"*, texture *"must be multiples of 32px"*, `File > Export > Export Hytale Model`, *"Export texture as `.png` separately"*.

### 14. HytaleModding — real block wiring (most useful concrete integration facts)

```json
{ "BlockType": {
    "DrawType": "Model",
    "CustomModel": "VFX/Blue_Fire/Blue_Fire.blockymodel",
    "CustomModelAnimation": "Blocks/Animations/Blue_Fire/Blue_Fire_Burn.blockyanim",
    "CustomModelTexture": [ { "Texture": "VFX/Blue_Fire/Blue_Fire.png", "Weight": 1 } ],
    "Looping": true,
    "RequiresAlphaBlending": false },
  "PlayerAnimationsId": "Block" }
```
Asset paths quoted: `Common/VFX/Fire/Fire.png`, `Common/VFX/Fire/Fire.blockymodel`, `Common/Blocks/Animations/Fire/Fire_Burn.blockyanim`, `Server/Item/Item/Deco/Deco_Fire.json`.

### 15. MCP `SKILL.md` (secondary, but the only source that discusses *both* formats + editor behaviour together)

* Formats: `hytale_character` = **64 units per world block**, `hytale_prop` = **32**; *"These numbers describe geometry/texel density, not maximum atlas resolution. Texture width and height may differ and must each be multiples of 32."*
* *"Exported models have a 255-node limit; exporter folding of a group's main cube affects the actual count."*
* Stretch: *"the art guide recommends roughly 0.7–1.3 per axis for its style, which is a visual recommendation rather than the MCP schema's validity range."*
* Shading names: `standard`, `flat`, `fullbright`, `reflective`. *"The exported property is not proof of matching viewport appearance; verify materials in Hytale."*
* UV: *"`set_cube_uv` preserves Hytale's `autouv=1` and dimension-linked rectangle sizes… changing those extents is rejected before Undo. Stretch changes the visible shape, not that base UV size."* / *"Hytale quads cannot convert to box UV."*
* Texture assignment: *"Texture selection follows an attachment collection or the project default, even when the format reports `single_texture=false`; non-null per-face texture assignments are rejected."*
* **Animation:** *"Hytale animation files use 60 frame units per second. MCP keyframe times are seconds; `time=0.5` represents frame 30 at export."* / *"Hytale uses quaternion interpolation and wraps looping clips."* / *"The upstream `.blockyanim` exporter writes `holdLastKeyframe`, and maps Catmull-Rom to smooth and other interpolation to linear. The editor's `once`, step or Bezier state therefore does not establish equivalent runtime playback."* / *"Native UV-offset channels exist, but the current MCP family has no dedicated UV-offset-keyframe tool."*
* Validation: node count is taken from *"the installed `blockymodel` compiler's main-model output"* and checks both texture dimensions are positive multiples of 32; reports `main_model_node_count_and_texture_dimensions`; *"Older bundles used heuristic node counts and incorrect density-based texture checks."*
* Resources: `hytale://format`, `hytale://attachments/{id}`, `hytale://pieces/{id}`, `hytale://cubes/{id}`; cross-references `src/formats.ts`, `src/element.ts`, `src/attachment_texture.ts`, `src/blockymodel.ts`, `src/blockyanim.ts`.

### 16. `blockymodel-web` — published TS types (`src/types/blockymodel.ts`)

Matches the plugin with three deviations: `unwrapMode: "custom" | "auto"`; `BlockyNode.position`/`orientation` optional; and exported defaults `DEFAULT_POSITION {0,0,0}`, `DEFAULT_ORIENTATION {w:1,x:0,y:0,z:0}`, `DEFAULT_STRETCH {1,1,1}`. `TextureLayout` face keys are `front/back/left/right/top/bottom`; `FaceUV = { offset: Vec2; mirror:{x,y}; angle: 0|90|180|270 }` (it omits `lockUVs`/`transparent`); `ShapeSettings = { size?, normal?, isPiece?, isStaticBox? }`.

### 17. `blockymodel-texture` — UV semantics

* *"Offset: Specifies top-left corner of texture region in pixel coordinates."*
* *"The V coordinate is flipped during application because Hytale uses image coordinates (top-left origin, Y increases downward) while Three.js UV uses bottom-left origin (V increases upward)."*
* Angle semantics: 0 none; 90 *"Swap dimensions, adjust mirrors"*; 180 flip both; 270 *"Swap dimensions, flip different axis"*.
* Face-axis mapping: `front = +Z`, `back = -Z`, `left = -X`, `right = +X`, `top = +Y`, `bottom = -Y`.
* Pixel-art rendering defaults: nearest-neighbour, alpha test 0.1.

### 18. `@hytale-tools/skin-viewer` — published `.blockyanim` interface

```ts
interface BlockyAnimData { formatVersion: number; duration: number;
                           holdLastKeyframe: boolean;
                           nodeAnimations: Record<string, NodeAnimation> }
interface Keyframe<T> { time: number; delta: T; interpolationType?: "smooth"|"linear"|"step" }
interface NodeAnimation { position?: Keyframe<Vec3Delta>[]; orientation?: Keyframe<QuatDelta>[];
                          shapeStretch?: Keyframe<Vec3Delta>[]; shapeVisible?: Keyframe<boolean>[];
                          shapeUvOffset?: Keyframe<{x:number;y:number}>[] }
```
* *"position: additive offset from rest pose (in Blockbench pixels, scaled for GLB)"* / *"orientation: quaternion multiplied onto rest pose"* / *"shapeStretch: scale multiplier applied to the node"* / *"shapeUvOffset: UV coordinate shift (in texture pixels) for sprite-sheet animations"* / *"shapeVisible: visibility toggle (boolean)"*.
* *"Keyframe times are frames at 60 fps (the Blockbench Hytale plugin's rate). Override with `parseOptions: { fps }`."*
* *"Positions are additive deltas in blockymodel pixels, scaled by 1/16 into GLB units"* with the default documented as *"a GLB unit is a quarter block"*.
* *"Node names in `nodeAnimations` must equal the GLB's bone names… Tracks for bones the model lacks are dropped silently."*
* *"Sprite-sheet channels (`shapeUvOffset`, `shapeStretch`, `shapeVisible`) target the mesh of a group: for a group node `Foo` the viewer looks for a mesh named `Foo_1` first (the GLB export convention), then `Foo` itself."*
* *"Missing channel arrays are treated as empty, and files without a numeric `duration` and `nodeAnimations` record throw a descriptive error."*
* Playback: `loop` flag per emote, crossfade `fadeDuration`, `playbackRate`, `seek` in **seconds** (`viewer.seek(0.5) // seconds`).
* Its 34 bundled emotes are described as *"derived from Hytale animation assets and remains the property of Hypixel Studios"* — i.e. real `.blockyanim` files exist publicly as keyframe data inside `dist/emotes.mjs` (639 kB).

### 21–22. `hytale-docs.com` art-assets pages — **what they actually claim**

`models.md` presents this schema:

```json
{ "format_version": "1.0",
  "model": { "identifier": "custom:my_creature", "texture_width": 64, "texture_height": 64,
             "bones": [ { "name": "root", "pivot": [0,0,0], "cubes": [...], "children": [...] } ] } }
```
with a property table (`format_version`, `identifier`, `texture_width`, `texture_height`, `bones`), cube fields `origin`/`size`/`uv`/`inflate`/`mirror`, `faces.{north,south,east,west,up,down}.uv = [u1,v1,u2,v2]`, `visible_bounds_width/height/offset`, `display.hand.rotation/translation/scale`, and a `filetree` placing models under `mods/my-mod/assets/models/{blocks,items,entities}/`.

`textures.md` claims: PNG 32-bit RGBA; multiples of 32; **max 512×512**; stretch 0.7–1.3×; "power-of-two textures" to avoid bleeding; `_emissive.png` convention; animated textures via `{"animation":{"frames":8,"speed":2,"interpolate":false}}` as a vertical strip.

`art-assets/animations.md` is a 7-line stub that states only *"Animations use `.blockyanim` files"* and a 5-step Blockbench workflow.

`art-assets/overview.md` gives the correct density table (64px/unit characters, 32px/unit blocks), "use cuboids and quads only", and a bone hierarchy using `elbow_left`/`knee_left` names.

---

## (c) Conflicts and gaps versus "read the plugin source"

### c.1 `hytale-docs.com` models page is not merely incomplete — it is wrong

Every distinguishing identifier it prints belongs to the **Minecraft Bedrock / GeckoLib** family, not to Hytale:

| hytale-docs.com claims | Plugin source actually has |
|---|---|
| top-level `format_version` + nested `model` object | `{ nodes, format, lod }` — no `format_version`, no wrapper |
| `model.identifier` (`namespace:name`) | no identifier field at all; identity comes from the file path / server asset id |
| `model.texture_width` / `texture_height` | **absent**; the model file never stores atlas dimensions |
| `model.bones[]` with `pivot`/`cubes`/`children` | `nodes[]` with `position`(relative origin)/`orientation`(quaternion)/`shape`/`children` |
| cube `origin`, `size`, `uv`, `inflate`, `mirror` | `shape.offset`, `shape.settings.size`, `shape.stretch`, `shape.textureLayout[face].{offset,mirror,angle}` — **no `inflate`, no per-cube `mirror` boolean** |
| per-face `uv: [u1,v1,u2,v2]` | per-face `{offset:{x,y}, mirror:{x,y}, angle}` (a *point* + rotation, not a rectangle) |
| `visible_bounds_*`, `display.hand.*` | absent |
| `mods/<mod>/assets/models/…` | `assets/Characters|Cosmetics|Blocks|Items/…` inside the game's asset pack (`Common/…`, `Server/Item/Item/…`) |
| scale claim *"1 pixel = 1/16 block"* | density is **32 px/unit (props)** or **64 px/unit (characters)**; no 1/16 relationship |

**Recommendation:** never cite this page as a format source; if it must be referenced, cite it only as an example of a hallucinated schema. The same repo's `textures.md` numbers (512×512 cap, emissive/animated texture JSON, power-of-two) are **not** corroborated by the plugin, the official blog, or any other source, and should be treated as unattributed at best.

### c.2 Facts that exist **only** in the plugin source (no public documentation anywhere)

* the `id` counter starting at `"1"`; the `--C<n>` naming for extra cubes; `name.replace(/^.+:/,'')`
* `shape.offset` = main-cube centre minus element origin, and the parent-offset subtraction chain
* `settings.isStaticBox` node folding; `settings.isPiece` attachment semantics
* `unwrapMode: "custom"` as the only emitted value
* the exact quad normal derivation and the `size.z` fold
* the full UV rotation/mirror algorithm (90↔270 swapping) and `lockUVs`/`transparent`
* import-side texture discovery rules (`startsWith(modelName)`, `Texture.png`, `<Model>_Textures/`) and the `Blocks` path heuristic for `format: "prop"`
* the `../Animations/<folder>/*.blockyanim` auto-load convention
* `uv_offset` Y negation and integer rounding; the "always emit `shapeUvOffset: []`" quirk
* `duration… || FPS*2` default

### c.3 Facts that exist publicly but are **not** in the plugin's model/animation codecs

* the **255-node limit** — stated in `about.md`, Hytale Wiki, Doctale and the MCP skill, but it is enforced (if at all) in `src/validation.ts`/`src/element.ts`, not in the codec. The precise counting rule (*"each cube/quad counts except if it's the first cube in a respective group"*) is only in `about.md`/wiki.
* the **0.7–1.3× stretch** guideline — purely advisory; the schema accepts any float.
* texture **multiples of 32** — from the blog + MCP validation, not the codec.
* `unwrapMode: "auto"` — invented by `blockymodel-web`; the plugin never writes it.
* `interpolationType: "step"` — invented by `@hytale-tools/skin-viewer`; the plugin's own union is `'smooth' | 'linear'` and it downgrades everything non-Catmull-Rom to `linear`. Any documentation that advertises step/Bezier playback is describing the *editor*, not the file.
* `positionScale = 1/16` — a three.js viewer convention (`"a GLB unit is a quarter block"`), not a statement about Hytale world units.

### c.4 Genuine ambiguity that no public source resolves

* **`format` values.** Plugin writes `'prop'`/`'character'`; `blockymodel-web` types the union as `"character" | "prop"`; the official blog frames it as *density* (32 vs 64 px/unit). Whether the engine keys any behaviour off this string (beyond importer heuristics) is undocumented.
* **`lod`.** Always `'auto'` in practice; the type allows arbitrary strings; no public explanation of the other legal values.
* **`passiveLoopCount`** (protocol `Animation`) has no counterpart in any public asset documentation.
* **Node-count accounting** differs between the wiki ("each group counts, each cube/quad counts except the first cube in a group") and the MCP note ("exporter folding of a group's main cube affects the actual count"). Both describe the same fold but neither is normative.
* **Coordinate handedness / units** are not stated by any official doc; only the plugin's `ZYX` euler round-trip and third-party GLB scaling (1/16) hint at it.
* **Attachment/collection export** (`export_codec == "blockymodel"`, `isPiece`) has no public spec; only the plugin source and one wiki bullet.
* The **server-side JSON schema** for `Assets/Server/Models/*.json` is documented only as Javadoc getters (`ModelAsset`, `ModelAsset.Animation`); no field-name/JSON-shape reference exists publicly.

---

## (d) Documentation that does **not** exist (so the format has no official published spec)

### d.1 Officially confirmed absence

* **Hytale Wiki states it outright:** *"The Hytale team has stated that they are 'working on public creator documentation hosted on GitBook' but there is currently no official technical documentation."* (`https://hytalewiki.org/w/Technical:Documentation`, citing the 2025-11-20 modding strategy post.)
* **No JSON Schema / `$schema` / IDL / RFC** for `.blockymodel` or `.blockyanim` is published by Hypixel Studios — not in `docs.hytale.com` (Javadoc only), not on `hytale.com`, not in the plugin repo.
* **The plugin repo has no documentation directory and no wiki:** `README.md` is 483 bytes and says only *"This plugin adds support for the `.blockymodel` and `.blockyanim` file formats…"*. `src/` has no `.md` other than the build output `dist/about.md`. There is no `docs/` folder, no `SPEC.md`, no schema file.

### d.2 Pages I verified as non-existent / unavailable

| URL | Status |
|---|---|
| `https://www.blockbench.net/plugins/hytale_plugin` | **404** via fetch on 2026-09-22 (URL is still linked from the official blog post; likely moved or JS-gated — worth a manual browser check). |
| `https://raw.githubusercontent.com/…` (any repo) | unresolvable from this environment (`getaddrinfo ENOENT`) — not a missing page, an environment limit. |
| `https://hytale-docs.com/docs/modding/art-assets/blockymodel` | Not in `sitemap.xml`. |
| `https://hytale-docs.com/docs/modding/art-assets/blockyanim` | Not in `sitemap.xml`. |
| `https://hytale-docs.com/docs/tools/blockbench/file-format` (or `/blockymodel`, `/schema`, `/uv-layout`, `/attachments`) | Not in `sitemap.xml`. |
| `https://deepwiki.com/JannisX11/hytale-blockbench-plugin`, `https://deepwiki.com/vulpeslab/hytale-docs` | Pages exist but are JS-rendered (`Loading…`) and returned no content. |
| `https://www.npmjs.com/package/blockymodel-web` | HTTP 403 (Cloudflare interstitial); use `cdn.jsdelivr.net/npm/…` instead. |
| `https://api.github.com/repos/timiliris/Hytale-Docs/git/trees/main` | 404 — the default branch is **`master`**. |
| `https://grep.app`-style or GitHub code search for a schema file | Not usable unauthenticated; scoped out. |

### d.3 Complete inventory of the pages that *do* exist (from `hytale-docs.com/sitemap.xml`)

Format-relevant pages only — **eight**, and everything they contain is quoted in §b above:

* `/docs/modding/art-assets/overview`, `/models`, `/animations`, `/textures`
* `/docs/tools/blockbench/installation`, `/plugin-setup`, `/modeling`, `/animation`

Notably: `/docs/modding/art-assets/animations` is a **stub**, and `/docs/tools/blockbench/modeling` + `/animation` could not be retrieved as text (the site is client-rendered and the two source `.md` files would not come back through jsDelivr even though the tree listing confirms they exist at `content/docs/en/tools/blockbench/{modeling,animation}.md`). Their existence is confirmed; their content is unverified.

### d.4 Other doc sets that explicitly have **no** model/format coverage

* `vulpeslab/hytale-docs` (`hytale-docs.pages.dev`): no `art-assets`, no `tools/blockbench`. Full tree inspected.
* `HytaleModding/site`: guides cover plugins/ECS/NPC inner workings; the only `.blockyanim` mention is the block-texture guide. No model-format page.
* `corentingosselin/hytale-dev-doc`: plugin/server-setup content only; no art or model pages. Full tree inspected.
* Doctale: has an NPC-models page and a block-animations page with a "File Format" section, but no field-level `.blockymodel` schema.

---

## (e) Bottom line for this repo

1. The **only** normative source is `ref/hytale-blockbench-plugin` — consistent with the note already in `docs/blockymodel-spec.md` §"关于「专业格式文档」的说明".
2. The public prose sources that are *safe* to cite are: the official blog post (#2), the plugin `about.md`/Hytale Wiki (#8/#9), `docs.hytale.com` Javadoc for the *server* side (#3–#6), and — with the caveats in §c — the merger README (#12), HytaleModding's block guide (#14), the two npm schema packages (#16–#18) and the MCP skill (#15).
3. **`hytale-docs.com/docs/modding/art-assets/models` must be marked as unreliable**; the currently-linked "Hytale Docs 3D Models page" in `docs/blockymodel-spec.md` should either be dropped or annotated as contradicted by the plugin source.
4. Everything needed to write a *correct* spec is (a) the plugin source and (b) observed game assets — there is no shortcut through published documentation.
