use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use sage::{Component, World};

struct DropMe(Arc<AtomicUsize>);

impl Component for DropMe {}

impl Drop for DropMe {
    #[inline]
    fn drop(&mut self) {
        self.0.fetch_add(1, Relaxed);
    }
}

#[test]
fn create_world() {
    let _world = World::new();
}

#[test]
fn spawn_one_empty_entity() {
    let mut world = World::new();
    let entity = world.spawn(());
    let id = entity.id();
    assert_eq!(entity.component_count(), 0);
    assert!(world.is_alive(id));
    world.entity_mut(id).despawn();
    assert!(!world.is_alive(id));
}

#[test]
fn spawn_batch_empty() {
    let mut world = World::new();
    let mut batch = world.spawn_batch([(), (), ()]);
    let e1 = batch.next().unwrap();
    let e2 = batch.next().unwrap();
    let e3 = batch.next().unwrap();
    assert_eq!(batch.next(), None);

    assert_ne!(e1, e2);
    assert_ne!(e1, e3);
    assert_ne!(e2, e3);

    assert_eq!(world.entity(e1).component_count(), 0);
    assert_eq!(world.entity(e2).component_count(), 0);
    assert_eq!(world.entity(e3).component_count(), 0);
}

#[test]
fn spawn_components() {
    let mut world = World::new();

    let e = world.spawn((1u32, "hello"));
    assert_eq!(e.get::<u32>(), Some(&1u32));
    assert_eq!(e.get::<&'static str>(), Some(&"hello"));
    assert_eq!(e.get::<i32>(), None);
}

#[test]
fn replace_components() {
    let mut world = World::new();

    let mut e = world.spawn(1u32);
    assert_eq!(e.get::<u32>(), Some(&1u32));
    *e.get_mut().unwrap() = 2u32;
    assert_eq!(e.get::<u32>(), Some(&2u32));
}

#[test]
fn component_dropped() {
    let drop_counter = Arc::new(AtomicUsize::new(0));

    let mut world = World::new();
    let e = world.spawn(DropMe(drop_counter.clone())).id();
    assert_eq!(drop_counter.load(Relaxed), 0);
    world.entity_mut(e).despawn();
    assert_eq!(drop_counter.load(Relaxed), 1);
}

#[test]
fn add_component() {
    let mut world = World::new();

    let mut e = world.spawn(1u32);
    assert_eq!(e.get::<u32>(), Some(&1u32));
    assert_eq!(e.get::<i32>(), None);
    assert_eq!(e.component_count(), 1);
    e.add(4i32);
    assert_eq!(e.get::<u32>(), Some(&1u32));
    assert_eq!(e.get::<i32>(), Some(&4i32));
    assert_eq!(e.component_count(), 2);
}

#[test]
fn add_replace() {
    let mut world = World::new();

    let mut e = world.spawn((1u32, 2i32));
    assert_eq!(e.get::<u32>(), Some(&1u32));
    assert_eq!(e.get::<i32>(), Some(&2i32));
    assert_eq!(e.component_count(), 2);
    e.add(4u32);
    assert_eq!(e.get::<i32>(), Some(&2i32));
    assert_eq!(e.get::<u32>(), Some(&4u32));
    assert_eq!(e.component_count(), 2);
}

#[test]
fn remove_component() {
    let mut world = World::new();

    let mut e = world.spawn((1u32, 4i32));
    assert_eq!(e.get::<u32>(), Some(&1u32));
    assert_eq!(e.get::<i32>(), Some(&4i32));
    assert_eq!(e.component_count(), 2);
    e.remove::<u32>();
    assert_eq!(e.get::<u32>(), None);
    assert_eq!(e.get::<i32>(), Some(&4i32));
    assert_eq!(e.component_count(), 1);
}
