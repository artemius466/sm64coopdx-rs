//! Entry point for `sm64coopdx-rs`.
//!
//! Mirrors `src/pc/pc_main.c::main()`: set up the platform, graphics, audio,
//! network, then run the main loop. This proof-of-concept is headless — it runs
//! the fixed-timestep game loop against the ported entity system and prints a
//! summary, so it can be exercised without a display or audio device.

use sm64coopdx_rs::game::game_loop::{GameLoop, HeadlessCallbacks};
use sm64coopdx_rs::game::object::Object;
use sm64coopdx_rs::game::object_list::{Behavior, ObjectList};
use sm64coopdx_rs::netcode::network::{LoopbackNetwork, Network, NetworkMode};

/// A simple "gravity" behavior: an object falls and settles on the floor.
/// Demonstrates an object whose state evolves over frames.
fn gravity_behavior(floor: f32) -> Behavior {
    Behavior::new(vec![Box::new(
        move |obj: &mut Object, _frame: u32| {
            let pos = obj.o_pos();
            let vel = obj.o_vel();
            let new_vel_y = vel.y - 9.8 * 0.016;
            let new_y = pos.y + new_vel_y * 0.016;
            if new_y < floor {
                obj.set_o_pos(sm64coopdx_rs::math::Vec3f::new(pos.x, floor, pos.z));
                obj.set_o_vel(sm64coopdx_rs::math::Vec3f::new(vel.x, 0.0, vel.z));
            } else {
                obj.set_o_vel(sm64coopdx_rs::math::Vec3f::new(vel.x, new_vel_y, vel.z));
            }
            true // stay alive
        },
    )])
}

/// A "coin" behavior: spins for a few frames then disappears.
fn coin_behavior(lifespan: u32) -> Behavior {
    Behavior::new(vec![Box::new(move |_obj: &mut Object, frame: u32| frame < lifespan)])
}

fn main() {
    println!("sm64coopdx-rs — Rust rewrite of the SM64 multiplayer game core");
    println!("Upstream: https://github.com/coop-deluxe/sm64coopdx");
    println!();

    // --- Set up the network (the project's defining feature) ---
    let mut net = LoopbackNetwork::new();
    net.init(
        NetworkMode::Server,
        sm64coopdx_rs::netcode::network::NetworkSystem::Socket,
    );
    println!("[net] server initialised, connected = {}", net.is_connected());

    // --- Set up the game (owns the object pool) ---
    let mut loop_ctx = GameLoop::new(HeadlessCallbacks);
    let game = &mut loop_ctx.game;

    let gravity = game.register_behavior(gravity_behavior(0.0));
    let coin = game.register_behavior(coin_behavior(60));

    // Spawn a "Mario" on the player list and some coins on the level list.
    game.spawn_object(ObjectList::Player, gravity)
        .expect("failed to spawn mario");
    println!("[game] spawned mario (player list)");

    for _ in 0..5 {
        game.spawn_object(ObjectList::Level, coin);
    }
    println!("[game] spawned 5 coins (level list)");
    println!(
        "[game] pool used = {}, free = {}",
        game.pool.used_count(),
        game.pool.free_count()
    );
    println!();

    // --- Run the fixed-timestep game loop ---
    let frames = 60u32;
    for _ in 0..frames {
        loop_ctx.step();
    }

    println!("[loop] ran {} fixed-timestep frames", frames);
    println!("[loop] game frame counter = {}", loop_ctx.game.frame);
    println!(
        "[game] pool used = {}, free = {}",
        loop_ctx.game.pool.used_count(),
        loop_ctx.game.pool.free_count()
    );

    // --- Report synced objects over the network ---
    let mut synced = 0;
    for obj in loop_ctx.game.pool.iter_all() {
        if obj.is_active() {
            net.send_object(&sm64coopdx_rs::netcode::network::SyncObject::new(
                obj as *const Object as usize as u32,
                loop_ctx.game.frame,
            ));
            synced += 1;
        }
    }
    let received = net.receive_objects();
    println!("[net] sent {} object states, received {}", synced, received);

    println!();
    println!("OK — game loop + entity system running.");
}
