//! The game module — mirrors `src/game/` in the upstream repo.
//!
//! Owns the object pool and drives the per-frame update of every object.

pub mod object;
pub mod object_list;
pub mod game_loop;

use crate::game::object::{BehaviorScript, Object};
use crate::game::object_list::{Behavior, ObjectList, ObjectPool};
use crate::types::u32;
use crate::game::object::OBJECT_POOL_CAPACITY;

/// The running game state that owns all objects.
///
/// Mirrors the upstream globals `gObjectPool`, `gObjectLists`, and the frame
/// counter driven by `thread5_game_loop` / `game_loop_one_iteration`.
pub struct Game {
    pub pool: ObjectPool,
    /// The current behavior scripts, keyed by index.
    pub behaviors: Vec<Behavior>,
    /// Frame counter, incremented once per fixed-timestep tick.
    pub frame: u32,
}

impl Game {
    /// Create a new game with a fresh object pool.
    pub fn new() -> Self {
        Game {
            pool: ObjectPool::new(),
            behaviors: Vec::new(),
            frame: 0,
        }
    }

    /// Register a behavior script and return its index handle.
    pub fn register_behavior(&mut self, behavior: Behavior) -> BehaviorScript {
        let idx = self.behaviors.len();
        self.behaviors.push(behavior);
        BehaviorScript::new(idx)
    }

    /// Spawn an object with a given behavior into a given list.
    /// Mirrors `create_object` + `allocate_object`.
    /// Returns a mutable handle to the object now living in the list.
    pub fn spawn_object(
        &mut self,
        list: ObjectList,
        behavior: BehaviorScript,
    ) -> Option<&mut Object> {
        let list_idx = list as usize;
        let obj = self.pool.allocate_or_reclaim(list_idx)?;
        obj.behavior = behavior;
        obj.init_bhv_command = behavior;
        obj.cur_bhv_command = behavior;
        Some(obj)
    }

    /// Update every object in a list for one frame.
    /// Mirrors `update_objects_in_list` / `update_objects_starting_at`.
    ///
    /// The object lives in `self.pool`; its behavior lives in `self.behaviors`.
    /// These are disjoint fields, so we can borrow both at once.
    pub fn update_list(&mut self, list: ObjectList) -> usize {
        let list_idx = list as usize;
        let cat = &mut self.pool.lists[list_idx];
        let n = cat.len();
        let mut active = 0;
        for i in 0..n {
            let Some(obj) = cat.get_mut(i) else {
                break;
            };
            if obj.is_deactivated() {
                continue;
            }
            let behavior_idx = obj.behavior.index();
            if let Some(behavior) = self.behaviors.get_mut(behavior_idx) {
                if behavior.update(obj, self.frame) {
                    active += 1;
                } else {
                    obj.mark_for_deletion();
                }
            }
        }
        active
    }

    /// Update all objects for one frame. Mirrors `update_objects`.
    pub fn update_all(&mut self) {
        for i in 0..ObjectList::NUM {
            let list = ObjectList::from_index(i as i16).unwrap();
            self.update_list(list);
        }
        // Unlink objects that were deactivated this frame (mirrors the upstream
        // post-update cleanup that frees dead objects back to the pool).
        for i in 0..ObjectList::NUM {
            let list = ObjectList::from_index(i as i16).unwrap();
            self.pool.purge_deactivated(list as usize);
        }
    }

    /// Advance the frame counter (one fixed-timestep tick).
    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        self.update_all();
    }
}

impl Default for Game {
    fn default() -> Self {
        Game::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial behavior that lives for `n` frames then dies.
    fn lifetime(n: u32) -> Behavior {
        Behavior::new(vec![Box::new(move |_obj, frame: u32| frame < n)])
    }

    #[test]
    fn spawn_and_update_object() {
        let mut game = Game::new();
        let bhv = game.register_behavior(lifetime(3));
        let obj = game.spawn_object(ObjectList::Level, bhv).unwrap();
        assert_eq!(game.pool.used_count(), 1);

        // Frame 0, 1: active. Frame 2: still active (frame < 3).
        game.tick();
        game.tick();
        assert_eq!(game.pool.lists[ObjectList::Level as usize].len(), 1);
        // Frame 3: behavior returns false -> object dies.
        game.tick();
        assert_eq!(game.pool.used_count(), 0);
    }

    #[test]
    fn pool_capacity_respected() {
        let mut game = Game::new();
        let bhv = game.register_behavior(lifetime(100));
        let mut spawned = 0;
        for _ in 0..OBJECT_POOL_CAPACITY {
            if game.spawn_object(ObjectList::Level, bhv).is_some() {
                spawned += 1;
            }
        }
        assert_eq!(spawned, OBJECT_POOL_CAPACITY);
        // Pool exhausted; next spawn fails.
        assert!(game.spawn_object(ObjectList::Level, bhv).is_none());
    }
}
