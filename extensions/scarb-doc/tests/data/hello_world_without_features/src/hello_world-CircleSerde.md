# CircleSerde

Fully qualified path: `hello_world::CircleSerde`

<pre><code class="language-rust">impl CircleSerde of core::serde::Serde&lt;Circle&gt;</code></pre>

## Impl functions

### serialize

Fully qualified path: `hello_world::CircleSerde::serialize`

<pre><code class="language-rust">fn serialize(self: @Circle, ref output: core::array::Array&lt;felt252&gt;)</code></pre>

### deserialize

Fully qualified path: `hello_world::CircleSerde::deserialize`

<pre><code class="language-rust">fn deserialize(ref serialized: core::array::Span&lt;felt252&gt;) -&gt; core::option::Option&lt;Circle&gt;</code></pre>


