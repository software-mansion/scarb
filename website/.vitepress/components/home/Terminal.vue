<template>
  <div class="dark window">
    <div class="traffic-lights">
      <div class="traffic-light red" />
      <div class="traffic-light yellow" />
      <div class="traffic-light green" />
    </div>
    <p>scarb init --name hello_world</p>
    <p>
      scarb add alexandria_math@0.1.0
    </p>
    <p>scarb build</p>
    <p>scarb fmt</p>
    <p>scarb test</p>
  </div>
</template>

<style scoped>
.window {
  background-color: var(--vp-c-bg-alt);
  border-radius: 0.5rem;
  box-shadow: var(--vp-shadow-5);
  color: var(--vp-c-text-1);
  font-family: var(--vp-font-family-mono);
  font-size: var(--vp-code-font-size);
  -webkit-font-smoothing: auto;
  height: 100%;
  line-height: 1.5;
  -moz-osx-font-smoothing: auto;
  overflow: hidden;
  padding: 1rem 1.25rem 1.5rem;
  text-align: left;
}

.traffic-lights {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
  top: 0;
}

.traffic-light {
  border-radius: 9999px;
  height: 0.75rem;
  width: 0.75rem;
}

.red {
  background-color: var(--swm-c-red-100);
}

.yellow {
  background-color: var(--swm-c-yellow-100);
}

.green {
  background-color: var(--swm-c-green-100);
}

.window > p:before {
  color: var(--swm-c-green-100);
  content: "$ ";
  user-select: none;
}

.br {
  display: none;
  user-select: none;
}

@media (max-width: 1024px) {
  .br {
    display: unset;
  }
}
</style>
