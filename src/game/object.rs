//! The Object entity system.
//!
//! This is the heart of the game. Every entity — Mario, enemies, platforms,
//! coins, bullets, particles — is an `Object`. Objects live in a fixed-size
//! pool (`OBJECT_POOL_CAPACITY`), are linked into per-list doubly-linked lists,
//! and are driven by *behavior scripts* (a tiny bytecode interpreter).
//!
//! Ported from:
//!   - `include/types.h`       (struct Object, struct ObjectNode)
//!   - `include/object_fields.h` (oPosX / oPosY / ... field accessors)
//!   - `include/object_constants.h` (ACTIVE_FLAG_*)
//!   - `src/game/object_list_processor.{h,c}` (object lists, pool)
//!   - `src/game/spawn_object.c` (allocate / create / unload lifecycle)
//!
//! The C version packs the object into a single flat struct with a union of
//! typed field arrays; we keep the same field layout and accessors so behavior
//! scripts that read e.g. `oPosX` behave identically.

use crate::math::{Mat4, Vec3f, Vec3s};
use crate::types::{f32, s16, s32, u8, u32};

/// Maximum number of objects that can exist simultaneously.
/// Mirrors `OBJECT_POOL_CAPACITY` in object_list_processor.h.
pub const OBJECT_POOL_CAPACITY: usize = 1200;

/// Depth of the behavior-script call stack per object.
pub const OBJECT_MAX_BHV_STACK: usize = 16;

/// Number of object fields (see object_fields.h).
pub const OBJECT_NUM_FIELDS: usize = 0x90;

/// Active-flag bits, from object_constants.h.
pub const ACTIVE_FLAG_DEACTIVATED: u32 = 0;
pub const ACTIVE_FLAG_ACTIVE: u32 = 1 << 0;
pub const ACTIVE_FLAG_FAR_AWAY: u32 = 1 << 1;
pub const ACTIVE_FLAG_UNK2: u32 = 1 << 2;
pub const ACTIVE_FLAG_IN_DIFFERENT_ROOM: u32 = 1 << 3;
pub const ACTIVE_FLAG_UNIMPORTANT: u32 = 1 << 4;
pub const ACTIVE_FLAG_INITIATED_TIME_STOP: u32 = 1 << 5;
pub const ACTIVE_FLAG_MOVE_THROUGH_GRATE: u32 = 1 << 6;
pub const ACTIVE_FLAG_DITHERED_ALPHA: u32 = 1 << 7;
pub const ACTIVE_FLAG_UNK8: u32 = 1 << 8;

/// The graphics node header that every object begins with.
/// Mirrors `struct GraphNodeObject` (include/types.h).
#[derive(Clone, Copy, Debug, Default)]
pub struct GfxNodeObject {
    // Flags controlling rendering (GRAPH_RENDER_*).
    pub flags: u32,
    pub inited: bool,
    pub shadowInvisible: bool,
    pub disableAutomaticShadowPos: bool,
    pub skipInViewCheck: bool,
    pub areaIndex: i8,
    pub activeAreaIndex: i8,

    pub pos: Vec3f,
    pub prevPos: Vec3f,
    pub angle: Vec3s,
    pub prevAngle: Vec3s,
    pub scale: Vec3f,
    pub prevScale: Vec3f,
    pub shadowPos: Vec3f,
    pub prevShadowPos: Vec3f,
    pub cameraToObject: Vec3f,
}

impl GfxNodeObject {
    /// The object is currently being rendered / active.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.flags & ACTIVE_FLAG_ACTIVE != 0
    }
}

/// The doubly-linked list node that prefixes every object.
/// Mirrors `struct ObjectNode` (include/types.h).
///
/// The list stores `Box<Object>`, and `ObjectNode` is the object's first
/// member, so `next`/`prev` point at sibling `Object`s (in C these are cast
/// between `ObjectNode*` and `Object*`; here they are simply `Object`).
#[derive(Clone, Debug, Default)]
pub struct ObjectNode {
    pub gfx: GfxNodeObject,
    pub next: Option<Box<Object>>,
    pub prev: Option<Box<Object>>,
}

/// The behavior script entry point type.
/// In C this is `uintptr_t`; in Rust we model it as an opaque handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct BehaviorScript(pub usize);

impl BehaviorScript {
    #[inline]
    pub fn new(idx: usize) -> Self {
        BehaviorScript(idx)
    }

    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

/// The Object — the fundamental entity of the game.
///
/// Mirrors `struct Object` (include/types.h). The `header` is the first member,
/// so `ObjectNode` and `Object` share storage; we keep that layout.
#[derive(Clone, Debug)]
pub struct Object {
    pub header: ObjectNode,

    // --- Linkage ---
    pub prev_obj: Option<Box<Object>>,
    pub parent_obj: Option<Box<Object>>,
    pub using_obj: Option<Box<Object>>,
    pub platform: Option<Box<Object>>,
    pub collided_objs: [Option<Box<Object>>; 4],
    pub num_collided_objs: i16,

    // --- Behavior ---
    pub behavior: BehaviorScript,
    pub init_bhv_command: BehaviorScript,
    pub cur_bhv_command: BehaviorScript,
    pub bhv_stack: [usize; OBJECT_MAX_BHV_STACK],
    pub bhv_stack_index: u32,
    pub bhv_delay_timer: i16,

    // --- State ---
    pub active_flags: u32,
    pub collided_obj_interact_types: u32,
    pub respawn_info_type: i16,

    // --- Hitboxes ---
    pub hitbox_radius: f32,
    pub hitbox_height: f32,
    pub hurtbox_radius: f32,
    pub hurtbox_height: f32,
    pub hitbox_down_offset: f32,

    // --- Area timers ---
    pub area_timer: u32,
    pub area_timer_duration: u32,
    pub area_timer_type: i16,

    // --- Transform ---
    pub transform: Mat4,

    // --- Surfaces ---
    pub first_surface: u32,
    pub num_surfaces: u32,
    pub held_by_player_index: u32,

    // --- Misc flags ---
    pub set_home: u8,
    pub ctx: u8,
    pub allow_remote_interactions: u8,
    pub global_player_index: u8,
    pub coop_flags: u8,
    pub hook_render: u8,

    // --- Object fields (the big union from include/types.h) ---
    // Access via the `o_*` methods below, which map to the exact same
    // indices as the C macros in object_fields.h. `asF32` is a 1D array of
    // OBJECT_NUM_FIELDS f32s; `oPosX`/`oPosY`/`oPosZ` are consecutive entries.
    pub raw_data: [f32; OBJECT_NUM_FIELDS],

    // --- Collision / physics ---
    pub collision_data: Option<Box<Object>>,
    pub respawn_info: Option<Box<Object>>,
    pub o_intangible_timer: i32,
    pub o_damage_or_coin_value: i16,
    pub o_health: i16,
    pub o_collision_distance: f32,
    pub o_drawing_distance: f32,
    pub o_distance_to_mario: f32,
    pub o_room: i16,
}

impl Default for Object {
    /// All fields zeroed. `new_fresh` then sets the active defaults used by
    /// `allocate_object` in spawn_object.c.
    fn default() -> Self {
        Object {
            header: ObjectNode::default(),
            prev_obj: None,
            parent_obj: None,
            using_obj: None,
            platform: None,
            collided_objs: [const { None }; 4],
            num_collided_objs: 0,
            behavior: BehaviorScript::default(),
            init_bhv_command: BehaviorScript::default(),
            cur_bhv_command: BehaviorScript::default(),
            bhv_stack: [0; OBJECT_MAX_BHV_STACK],
            bhv_stack_index: 0,
            bhv_delay_timer: 0,
            active_flags: 0,
            collided_obj_interact_types: 0,
            respawn_info_type: 0,
            hitbox_radius: 0.0,
            hitbox_height: 0.0,
            hurtbox_radius: 0.0,
            hurtbox_height: 0.0,
            hitbox_down_offset: 0.0,
            area_timer: 0,
            area_timer_duration: 0,
            area_timer_type: 0,
            transform: Mat4::default(),
            first_surface: 0,
            num_surfaces: 0,
            held_by_player_index: 0,
            set_home: 0,
            ctx: 0,
            allow_remote_interactions: 0,
            global_player_index: 0,
            coop_flags: 0,
            hook_render: 0,
            raw_data: [0.0; OBJECT_NUM_FIELDS],
            collision_data: None,
            respawn_info: None,
            o_intangible_timer: 0,
            o_damage_or_coin_value: 0,
            o_health: 0,
            o_collision_distance: 0.0,
            o_drawing_distance: 0.0,
            o_distance_to_mario: 0.0,
            o_room: 0,
        }
    }
}

impl Object {
    /// Allocate a fresh object with the default field values used by
    /// `allocate_object` in spawn_object.c.
    pub fn new_fresh() -> Self {
        let mut obj = Object::default();
        obj.active_flags = ACTIVE_FLAG_ACTIVE | ACTIVE_FLAG_UNK8;
        obj.parent_obj = None;
        obj.prev_obj = None;
        obj.collided_obj_interact_types = 0;
        obj.num_collided_objs = 0;
        obj.hitbox_radius = 50.0;
        obj.hitbox_height = 100.0;
        obj.hurtbox_radius = 0.0;
        obj.hurtbox_height = 0.0;
        obj.hitbox_down_offset = 0.0;
        obj.held_by_player_index = 0;
        obj.platform = None;
        obj.collision_data = None;
        obj.respawn_info = None;
        obj.o_intangible_timer = -1;
        obj.o_health = 2048;
        obj.o_collision_distance = 1000.0;
        obj.o_drawing_distance = 4000.0;
        obj.o_distance_to_mario = 19000.0;
        obj.o_room = -1;
        obj
    }

    // --- Object field accessors (object_fields.h) ---
    // These map 1:1 to the C macros so behavior scripts read the same memory.

    #[inline]
    pub fn o_pos(&self) -> Vec3f {
        // oPosX/asF32[0x06], oPosY/asF32[0x07], oPosZ/asF32[0x08]
        Vec3f::new(
            self.raw_data[0x06],
            self.raw_data[0x07],
            self.raw_data[0x08],
        )
    }

    #[inline]
    pub fn set_o_pos(&mut self, p: Vec3f) {
        self.raw_data[0x06] = p.x;
        self.raw_data[0x07] = p.y;
        self.raw_data[0x08] = p.z;
    }

    #[inline]
    pub fn o_vel(&self) -> Vec3f {
        // oVelX/asF32[0x09], oVelY/asF32[0x0a], oVelZ/asF32[0x0b]
        Vec3f::new(
            self.raw_data[0x09],
            self.raw_data[0x0a],
            self.raw_data[0x0b],
        )
    }

    #[inline]
    pub fn set_o_vel(&mut self, v: Vec3f) {
        self.raw_data[0x09] = v.x;
        self.raw_data[0x0a] = v.y;
        self.raw_data[0x0b] = v.z;
    }

    #[inline]
    pub fn o_flags(&self) -> u32 {
        // oFlags/asU32[0x01]: reinterpret the f32 bits as u32 (same memory as
        // oPosX..oPosZ region in the flat union).
        self.raw_data[0x01].to_bits()
    }

    #[inline]
    pub fn set_o_flags(&mut self, v: u32) {
        self.raw_data[0x01] = f32::from_bits(v);
    }

    #[inline]
    pub fn o_health(&self) -> i16 {
        self.o_health
    }

    #[inline]
    pub fn set_o_health(&mut self, v: i16) {
        self.o_health = v;
    }

    #[inline]
    pub fn is_deactivated(&self) -> bool {
        self.active_flags == ACTIVE_FLAG_DEACTIVATED
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.active_flags & ACTIVE_FLAG_ACTIVE != 0
    }

    /// Mark for deletion at end of frame (mark_obj_for_deletion).
    pub fn mark_for_deletion(&mut self) {
        self.active_flags = ACTIVE_FLAG_DEACTIVATED;
    }
}
