# CircleSerde

Fully qualified path: [hello_world](./hello_world.md)::[CircleSerde](./hello_world-CircleSerde.md)

<pre><code class="language-cairo">impl CircleSerde of Serde&lt;<a href="hello_world-Circle.html">Circle</a>&gt;;</code></pre>

## Impl functions

### serialize

Fully qualified path: [hello_world](./hello_world.md)::[CircleSerde](./hello_world-CircleSerde.md)::[serialize](./hello_world-CircleSerde.md#serialize)

<pre><code class="language-cairo">fn serialize(self: Circle, ref output: Array&lt;felt252&gt;)</code></pre>


### deserialize

Fully qualified path: [hello_world](./hello_world.md)::[CircleSerde](./hello_world-CircleSerde.md)::[deserialize](./hello_world-CircleSerde.md#deserialize)

<pre><code class="language-cairo">fn deserialize(ref serialized: Span&lt;felt252&gt;) -&gt; Option&lt;Circle&gt;</code></pre>


