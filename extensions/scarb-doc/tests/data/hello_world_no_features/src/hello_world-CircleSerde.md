# CircleSerde

Fully qualified path: `hello_world::CircleSerde`

```cairo
impl CircleSerde of core::serde::Serde<Circle>
```

## Impl functions

### serialize

Fully qualified path: `hello_world::CircleSerde::serialize`

```cairo
fn serialize(self: @Circle, ref output: core::array::Array<felt252>)
```


### deserialize

Fully qualified path: `hello_world::CircleSerde::deserialize`

```cairo
fn deserialize(ref serialized: core::array::Span<felt252>) -> core::option::Option<Circle>
```


