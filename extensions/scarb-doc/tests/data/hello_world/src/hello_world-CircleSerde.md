# CircleSerde

Fully qualified path: `hello_world::CircleSerde`

```rust
impl CircleSerde of core::serde::Serde<Circle>
```

## Impl Functions

### serialize

Fully qualified path: `hello_world::CircleSerde::serialize`

```rust
fn serialize(self: @Circle, ref output: core::array::Array<felt252>)
```


### deserialize

Fully qualified path: `hello_world::CircleSerde::deserialize`

```rust
fn deserialize(ref serialized: core::array::Span<felt252>) -> core::option::Option<Circle>
```


