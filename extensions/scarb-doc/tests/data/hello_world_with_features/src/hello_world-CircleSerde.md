# CircleSerde

Fully qualified path: `hello_world::CircleSerde`

```rust
impl CircleSerde of Serde<Circle>;
```

## Impl functions

### serialize

Fully qualified path: `hello_world::CircleSerde::serialize`

```rust
fn serialize(self: Circle, ref output: Array<felt252>)
```


### deserialize

Fully qualified path: `hello_world::CircleSerde::deserialize`

```rust
fn deserialize(ref serialized: Span<felt252>) -> Option<Circle>
```


