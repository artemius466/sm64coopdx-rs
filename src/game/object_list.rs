//! Object pool and per-list management.
//!
//! Ported from `src/game/spawn_object.c` and `src/game/object_list_processor.{h,c}`.
//!
//! Model of the upstream design:
//!   - A fixed pool of `OBJECT_POOL_CAPACITY` `Object`s.
//!   - Each object belongs to exactly one of `NUM_OBJ_LISTS` categories, which
//!     determine update order and collision groups.
//!   - A *free list* hands out object slots; freed objects return to it.
//!
//! The upstream stores each category as a doubly-linked list of `ObjectNode`s
//! threaded through a free list, for O(1) insert/remove. We keep the same
//! *architecture* (fixed pool, per-category lists, free list, behavior scripts)
//! but back each list with a `Vec` for a borrow-checker-friendly implementation.
//! The object lifecycle (allocate -> behave -> free) is identical.

use crate::game::object::{Object, OBJECT_POOL_CAPACITY};
use crate::types::{i16, u32};

/// A behavior update step: given the object being updated and the current frame,
/// return `true` to keep the object alive, `false` to deactivate it.
///
/// Mirrors the upstream behavior-script update functions (`const BehaviorScript *`
/// handlers). We store them as boxed `FnMut`s so behaviors can capture state,
/// like the upstream per-object behavior stacks.
pub type BehaviorStep = Box<dyn FnMut(&mut Object, u32) -> bool>;

/// A behavior script: an ordered list of update steps executed each frame.
///
/// Mirrors the upstream behavior scripts (small bytecode interpreted by
/// `bhv_*` functions). Each step receives the object and the frame index and
/// decides whether the object stays alive. This is enough to drive the object
/// lifecycle end-to-end.
pub struct Behavior {
    steps: Vec<BehaviorStep>,
}

impl Behavior {
    /// Create a behavior from an ordered list of update steps.
    pub fn new(steps: Vec<BehaviorStep>) -> Self {
        Behavior { steps }
    }

    /// Number of update steps.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the behavior has no steps.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Run one frame of the behavior against `obj`.
    ///
    /// Returns `true` if the object should remain active, `false` if it should
    /// be deactivated (mirroring the upstream "return 0" that deactivates).
    pub fn update(&mut self, obj: &mut Object, frame: u32) -> bool {
        for step in self.steps.iter_mut() {
            if !step(obj, frame) {
                return false;
            }
        }
        true
    }
}

/// The object lists, from the `enum ObjectList` in object_list_processor.h.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectList {
    Player = 0,      // Mario
    Ext = 1,         // extra list for Lua mods
    Destructive = 2, // bob-ombs, corkboxes
    Unused3 = 3,
    Genactor = 4, // most enemies (mips, bullet bills, bullies)
    Pushable = 5, // goombas, koopas, spinies
    Level = 6,    // hearts, stars, level props
    Unused7 = 7,
    Default = 8,  // fallback list
    Surface = 9,  // thwomps, whomps
    Polelike = 10, // hoot, whirlpools, poles
    Spawner = 11,
    Unimportant = 12, // unloaded to make room when the pool is exhausted
    NumObjectLists,
}

impl ObjectList {
    pub const NUM: usize = 13;

    /// Construct from a numeric index (mirrors the upstream enum).
    #[allow(dead_code)]
    pub fn from_index(idx: i16) -> Option<ObjectList> {
        match idx {
            0 => Some(ObjectList::Player),
            1 => Some(ObjectList::Ext),
            2 => Some(ObjectList::Destructive),
            3 => Some(ObjectList::Unused3),
            4 => Some(ObjectList::Genactor),
            5 => Some(ObjectList::Pushable),
            6 => Some(ObjectList::Level),
            7 => Some(ObjectList::Unused7),
            8 => Some(ObjectList::Default),
            9 => Some(ObjectList::Surface),
            10 => Some(ObjectList::Polelike),
            11 => Some(ObjectList::Spawner),
            12 => Some(ObjectList::Unimportant),
            _ => None,
        }
    }
}

/// A per-category list of objects.
///
/// Mirrors the upstream doubly-linked object list for one `ObjectList` category.
/// Backed by a `Vec` for a straightforward implementation; the semantics
/// (ordered membership, membership count) match.
#[derive(Debug)]
pub struct CategoryList {
    objects: Vec<Box<Object>>,
    name: &'static str,
}

impl CategoryList {
    fn new(name: &'static str) -> Self {
        CategoryList {
            objects: Vec::new(),
            name,
        }
    }

    /// True if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Number of objects currently in the list.
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Iterate over the objects in order.
    pub fn iter(&self) -> impl Iterator<Item = &Object> {
        self.objects.iter().map(|o| o.as_ref())
    }

    /// Append an object to the list.
    pub fn push(&mut self, obj: Box<Object>) {
        self.objects.push(obj);
    }

    /// Mutable handle to the object at `idx`, if in range.
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut Object> {
        self.objects.get_mut(idx).map(|o| o.as_mut())
    }

    /// Remove and return the last object (mirrors popping from the list head).
    pub fn pop(&mut self) -> Option<Box<Object>> {
        self.objects.pop()
    }

    /// Remove a specific object (by identity) and return it.
    ///
    /// Mirrors the upstream removal: unlink the object from the list. Uses a
    /// swap-remove so it's O(1); the removed object is returned to the caller.
    pub fn remove(&mut self, obj: &Object) -> Option<Box<Object>> {
        let idx = self
            .objects
            .iter()
            .position(|o| std::ptr::eq(&**o, obj))?;
        Some(self.objects.swap_remove(idx))
    }
}

/// The fixed object pool plus all the per-category lists.
///
/// This is the single owner of every `Object`. It mirrors the upstream globals
/// `gObjectPool`, `gObjectLists`, and `gFreeObjectList`.
#[derive(Debug)]
pub struct ObjectPool {
    /// The free slots, handed out by `allocate`. Mirrors `gFreeObjectList`.
    free: Vec<Box<Object>>,

    /// The NUM_OBJ_LISTS categories.
    pub lists: Vec<CategoryList>,
}

impl ObjectPool {
    /// Initialise the pool and free list.
    /// Mirrors `init_free_object_list` + `clear_object_lists`.
    pub fn new() -> Self {
        let mut free = Vec::with_capacity(OBJECT_POOL_CAPACITY);
        for _ in 0..OBJECT_POOL_CAPACITY {
            free.push(Box::new(Object::new_fresh()));
        }

        let lists = (0..ObjectList::NUM)
            .map(|i| CategoryList::new(match i {
                0 => "Player",
                1 => "Ext",
                2 => "Destructive",
                3 => "Unused3",
                4 => "Genactor",
                5 => "Pushable",
                6 => "Level",
                7 => "Unused7",
                8 => "Default",
                9 => "Surface",
                10 => "Polelike",
                11 => "Spawner",
                12 => "Unimportant",
                _ => "?",
            }))
            .collect();

        ObjectPool { free, lists }
    }

    /// Number of free object slots.
    pub fn free_count(&self) -> usize {
        self.free.len()
    }

    /// Total objects allocated across all lists.
    pub fn used_count(&self) -> usize {
        self.lists.iter().map(|l| l.len()).sum()
    }

    /// Iterate over every object across all lists.
    pub fn iter_all(&self) -> impl Iterator<Item = &Object> {
        self.lists.iter().flat_map(|l| l.iter())
    }

    /// Allocate an object into the given category list.
    /// Mirrors `try_allocate_object` + `allocate_object`.
    /// Returns a mutable handle to the object now living in the list, or
    /// `None` if the pool is exhausted.
    pub fn allocate(&mut self, list: usize) -> Option<&mut Object> {
        let mut obj = self.free.pop()?;
        // Clear any stale state from a previous life (mirrors allocate_object).
        obj.header.next = None;
        obj.header.prev = None;
        obj.active_flags =
            crate::game::object::ACTIVE_FLAG_ACTIVE | crate::game::object::ACTIVE_FLAG_UNK8;
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

        let cat = &mut self.lists[list];
        cat.push(obj);
        let idx = cat.objects.len() - 1;
        Some(cat.objects[idx].as_mut())
    }

    /// Free an object back to the free list, mirroring `deallocate_object`.
    pub fn free_object(&mut self, list: usize, obj: &Object) {
        if let Some(cat) = self.lists.get_mut(list) {
            if let Some(boxed) = cat.remove(obj) {
                self.free.push(boxed);
            }
        }
    }

    /// Find and free one `Unimportant` object to reclaim a slot.
    /// Mirrors `find_unimportant_object` + `unload_object`.
    pub fn reclaim_unimportant(&mut self) {
        let list = ObjectList::Unimportant as usize;
        if let Some(boxed) = self.lists[list].pop() {
            self.free.push(boxed);
        }
    }

    /// Remove all deactivated objects from a list and free them.
    /// Mirrors the post-update cleanup that unlinks dead objects from their list.
    pub fn purge_deactivated(&mut self, list: usize) {
        if let Some(cat) = self.lists.get_mut(list) {
            cat.objects.retain(|o| o.active_flags & crate::game::object::ACTIVE_FLAG_ACTIVE != 0);
        }
    }

    /// Allocate an object, falling back to reclaiming an unimportant one.
    /// Mirrors `allocate_object`.
    pub fn allocate_or_reclaim(&mut self, list: usize) -> Option<&mut Object> {
        // `try_allocate_object` pops a free slot; if the pool is exhausted
        // (no free slots), reclaim an unloaded object first, then allocate.
        if self.free.is_empty() {
            self.reclaim_unimportant();
        }
        self.allocate(list)
    }

    /// Reset all lists and return every object to the free list.
    /// Mirrors `clear_objects` + `init_free_object_list`.
    pub fn reset(&mut self) {
        for list in &mut self.lists {
            list.objects.clear();
        }
        // Re-fill the free list with all pool objects.
        let all: Vec<Box<Object>> = self
            .lists
            .iter_mut()
            .flat_map(|l| l.objects.drain(..))
            .collect();
        self.free = all;
    }
}

impl Default for ObjectPool {
    fn default() -> Self {
        ObjectPool::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_allocates_and_frees() {
        let mut pool = ObjectPool::new();
        assert_eq!(pool.free_count(), OBJECT_POOL_CAPACITY);
        assert_eq!(pool.used_count(), 0);

        // `allocate` returns a `&mut Object` that borrows the pool, so we can't
        // touch the pool while holding it. Drop the reference (via `let _`) to
        // release the borrow, then check the counts.
        let _ = pool.allocate(ObjectList::Default as usize).unwrap();
        assert_eq!(pool.free_count(), OBJECT_POOL_CAPACITY - 1);
        assert_eq!(pool.lists[ObjectList::Default as usize].len(), 1);
    }

    #[test]
    fn pool_exhaustion_reclaims_unimportant() {
        let mut pool = ObjectPool::new();
        // Put one object in the Unimportant list first (simulating an object
        // that was unloaded to it — this is what actually happens upstream when
        // the pool is exhausted mid-frame).
        {
            let _ = pool.allocate(ObjectList::Unimportant as usize).unwrap();
        }
        // Fill the rest of the pool completely with Default-list objects.
        for _ in 0..(OBJECT_POOL_CAPACITY - 1) {
            let _ = pool.allocate(ObjectList::Default as usize).unwrap();
        }
        assert_eq!(pool.free_count(), 0);

        // Exhaustion: allocate_or_reclaim should reclaim the unimportant object
        // and hand back a slot.
        let reclaimed = pool
            .allocate_or_reclaim(ObjectList::Level as usize)
            .is_some();
        assert!(reclaimed);
    }

    #[test]
    fn list_len_tracks_objects() {
        let mut pool = ObjectPool::new();
        for _ in 0..5 {
            let _ = pool.allocate(ObjectList::Level as usize).unwrap();
        }
        assert_eq!(pool.lists[ObjectList::Level as usize].len(), 5);
        assert_eq!(pool.used_count(), 5);
    }

    #[test]
    fn remove_specific_preserves_others() {
        let mut pool = ObjectPool::new();
        // Allocate three objects. Capture their stable addresses (the pool never
        // reallocates, so these stay valid) in per-allocation scopes so each
        // `&mut Object` borrow is released before the next allocation.
        let a_ptr = { pool.allocate(ObjectList::Level as usize).unwrap() as *const Object };
        let b_ptr = { pool.allocate(ObjectList::Level as usize).unwrap() as *const Object };
        let c_ptr = { pool.allocate(ObjectList::Level as usize).unwrap() as *const Object };
        assert_eq!(pool.lists[ObjectList::Level as usize].len(), 3);

        // Mark the middle object (index 1) as deactivated, then purge it. a and c
        // must remain.
        let list = ObjectList::Level as usize;
        pool.lists[list].objects[1].active_flags = 0;
        pool.purge_deactivated(list);
        assert_eq!(pool.lists[ObjectList::Level as usize].len(), 2);

        let ids: Vec<usize> = pool
            .lists[ObjectList::Level as usize]
            .iter()
            .map(|o| o as *const Object as usize)
            .collect();
        assert!(ids.iter().any(|&i| i == a_ptr as usize));
        assert!(ids.iter().any(|&i| i == c_ptr as usize));
        assert!(!ids.iter().any(|&i| i == b_ptr as usize));
    }
}
