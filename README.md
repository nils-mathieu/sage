# Information

`sage` is a simple Entity Component System (ECS) written in Rust.

## Usage

The first step to use this library is to create a bunch of components. Components are just
regular Rust types.

```rust
#[derive(Component, PartialEq)]
struct Position(pub f32, pub f32);

#[derive(Component, PartialEq)]
struct Velocity(pub f32, pub f32);
```

With that out of the way, it's possible to use those components in one of two ways:

1. Either a [`World`] can be created, which provides safe access to its content as long as
mutable or shared references to it can be provided (just like most regular Rust collections).

```rust
let mut world = World::new();

let entity = world.spawn(
    (
        Position(0.0, 0.0),
        Velocity(1.0, 1.0),
    )
);

assert!(entity.get().unwrap(), Position(0.0, 0.0));
assert!(entity.get().unwrap(), Velocity(1.0, 1.0));

*entity.get_mut().unwrap() = Position(1.0, 2.0);

assert!(entity.get().unwrap(), Position(1.0, 2.0));

entity.despawn();

world.spawn_batch([
    (
        "John",
        Position(0.0, 0.0),
        Velocity(1.0, 1.0),
    ),
    (
        "Jane",
        Position(1.0, 0.0),
        Velocity(1.0, 1.0),
    ),
]);
```

2. Or a raw [`Entities`] collection can be created, which provides unsafe shared access to every
   entity, allowing building more flexible abstractions on top of it, such as threaded
   schedulers, etc.

```rust
let mut entities = Entities::new();

let entity = entities.spawn(
    (
        Position(0.0, 0.0),
        Velocity(1.0, 1.0),
    )
);

assert!(entities.is_alive(entity));

// SAFETY:
//  We just spawned this entity.
let entity_ptr = unsafe { entities.get(entity.index()) };

// SAFETY:
//  No other thread is accessing this entity's `Position` component.
assert_eq!(unsafe { *entity_ptr.get() }, Position(0.0, 0.0));
```
