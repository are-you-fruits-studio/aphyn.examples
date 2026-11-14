//! The ECS on its own, with no rendering: creatures are spawned, lose health, some are poisoned and
//! cured, and the dead are despawned. All output goes to the console; the window stays empty.
//!
//! Shows entities and components, structural changes (create/destroy entities, add/remove
//! components) through `EntitiesHolderMut`, or `WorldDataMut` when the same system also inserts a
//! resource, queries with an optional component and with a filter, a resource inserted at runtime
//! and read through `Option<Res<_>>`, per-system state in `Local<_>`, and `Drop` running when an
//! entity is despawned.
//!
//! Engine modules: `aphyn_ecs` (world, components, resources, systems, schedules), `aphyn_app`
//! (`AppInitCtx`, `TimeResource`), `aphyn_window` (`WindowState`, for the window title).

use aphyn::*;

const SPAWN_INTERVAL: f64 = 0.5;
const REPORT_INTERVAL: f64 = 1.0;
const START_HEALTH: f32 = 6.0;
const AGING_PER_SECOND: f32 = 1.0;
const POISON_PER_SECOND: f32 = 2.0;
const CURE_BELOW: f32 = 2.0;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data,
        mut behavior,
        window,
        ..
    } = ctx;

    window.title = "Aphyn ecs example (output is in the console)".into();

    data.as_mut().resources_mut().insert(Spawner::default());

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(spawn_creatures);
    update.insert_system(lose_health);
    update.insert_system(cure_weak_creatures);
    update.insert_system(despawn_dead);
    update.insert_system(report);

    update.order_system(spawn_creatures).before_system(lose_health);
    update.order_system(lose_health).before_system(cure_weak_creatures);
    update.order_system(cure_weak_creatures).before_system(despawn_dead);
    update.order_system(despawn_dead).before_system(report);
}

#[derive(Component)]
struct Health(f32);

#[derive(Component)]
struct Poisoned;

// Prints when the entity that owns it is destroyed: components are dropped with their entity.
#[derive(Component)]
struct Creature {
    id: u32,
}

impl Drop for Creature {
    fn drop(&mut self) {
        println!("creature #{} despawned", self.id);
    }
}

#[derive(Resource, Default)]
struct Spawner {
    next_id: u32,
    next_spawn_time: f64,
}

// Inserted by `despawn_dead` the first time a creature dies, so it does not exist at startup.
#[derive(Resource)]
struct FirstDeath {
    time: f64,
}

fn spawn_creatures(time: Res<TimeResource>, mut spawner: ResMut<Spawner>, mut entities: EntitiesHolderMut) {
    let now = time.frame_time;
    if now < spawner.next_spawn_time {
        return;
    }
    spawner.next_spawn_time = now + SPAWN_INTERVAL;
    let id = spawner.next_id;
    spawner.next_id += 1;

    let creature = entities.create_entity();
    entities.set_components(creature, (Creature { id }, Health(START_HEALTH)));

    let poisoned = id.is_multiple_of(2);
    if poisoned {
        entities.add_component(creature, Poisoned).ok().unwrap();
    }

    println!("creature #{id} spawned{}", if poisoned { " (poisoned)" } else { "" });
}

fn lose_health(time: Res<TimeResource>, mut creatures: WorldQuery<(&mut Health, Option<&Poisoned>)>) {
    for (health, poisoned) in creatures.iter_mut() {
        let per_second = AGING_PER_SECOND + if poisoned.is_some() { POISON_PER_SECOND } else { 0.0 };
        health.0 -= per_second * time.delta_time;
    }
}

fn cure_weak_creatures(mut entities: EntitiesHolderMut) {
    let weak = entities
        .query_filtered::<(EntityId, &Creature, &Health), WithFilter<Poisoned>>()
        .iter()
        .filter(|(_, _, health)| health.0 < CURE_BELOW)
        .map(|(entity, creature, _)| (entity, creature.id))
        .collect::<Vec<_>>();

    for (entity, id) in weak {
        entities.remove_component::<Poisoned>(entity);
        println!("creature #{id} cured");
    }
}

fn despawn_dead(mut world: WorldDataMut) {
    let now = world.as_ref().resources().get::<TimeResource>().unwrap().frame_time;

    let mut entities = world.entities_mut();
    let dead = entities
        .query::<(EntityId, &Health)>()
        .iter()
        .filter(|(_, health)| health.0 <= 0.0)
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();

    for entity in &dead {
        entities.destroy_entity(*entity);
    }

    let mut resources = world.resources_mut();
    if !dead.is_empty() && resources.get::<FirstDeath>().is_none() {
        resources.insert(FirstDeath { time: now });
    }
}

fn report(
    time: Res<TimeResource>,
    mut next_report_time: Local<f64>,
    creatures: WorldQuery<&Creature>,
    poisoned: WorldQuery<&Creature, WithFilter<Poisoned>>,
    first_death: Option<Res<FirstDeath>>,
) {
    if time.frame_time < *next_report_time {
        return;
    }
    *next_report_time = time.frame_time + REPORT_INTERVAL;

    let first_death = match first_death {
        Some(first_death) => format!("first death at {:.1} s", first_death.time),
        None => String::from("no deaths yet"),
    };

    println!(
        "[{:5.1} s] alive: {}, poisoned: {}, {first_death}",
        time.frame_time,
        creatures.len(),
        poisoned.len()
    );
}
