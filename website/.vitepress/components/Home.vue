<script setup>
import Terminal from "./home/Terminal.vue";
import Features from "./home/Features.vue";
import { defineClientComponent } from "vitepress";

const Download = defineClientComponent(() => import("./home/Download.vue"));
</script>

<template>
  <div class="container hero">
    <h1 class="tagline">
      The build toolchain and package manager for Cairo and Starknet ecosystems
    </h1>
    <Download />
  </div>

  <div class="features-bg">
    <div class="container features-container">
      <Terminal class="terminal" />
      <Features />
    </div>
  </div>

  <div class="container vp-doc">
    <h2 id="community" tabindex="-1">Community</h2>
    <p>
      Scarb is created by the same team at
      <a href="https://swmansion.com" rel="noreferrer" target="_blank"
        >Software Mansion</a
      >
      that is behind
      <a
        href="https://foundry-rs.github.io/starknet-foundry"
        rel="noreferrer"
        target="_blank"
        >Starknet Foundry</a
      >, the Cairo and Starknet testing framework. We partnered with
      <a href="https://starkware.co" rel="noreferrer" target="_blank"
        >Starkware</a
      >, the creators of
      <a href="https://www.cairo-lang.org" rel="noreferrer" target="_blank"
        >Cairo</a
      >
      and
      <a href="https://www.starknet.io" rel="noreferrer" target="_blank"
        >Starknet</a
      >, while designing this tool.
    </p>
    <p>
      Feel free chat with us on our channel on
      <a href="https://t.me/+G_YxIv-XTFlhNWU0" rel="noreferrer" target="_blank"
        >Telegram</a
      >
      or Starknet's
      <a href="https://discord.gg/rKzsYaTMvA" rel="noreferrer" target="_blank"
        >Discord</a
      >!
    </p>
  </div>
</template>

<style scoped>
.container {
  margin: 0 auto;
  padding: 32px 24px;
  width: 100%;
}

@media (min-width: 640px) {
  .container {
    max-width: 640px;
  }
}

@media (min-width: 768px) {
  .container {
    max-width: 768px;
  }
}

@media (min-width: 1024px) {
  .container {
    max-width: 1024px;
  }
}

/*
@media (min-width: 1280px) {
  .container {
    max-width: 1280px;
  }
}

@media (min-width: 1440px) {
  .container {
    max-width: 1440px;
  }
}
*/

.hero {
  display: flex;
  flex-direction: column;
  gap: 40px;
}

.features-bg {
  background: linear-gradient(
    to bottom,
    transparent 4.5rem,
    var(--vp-c-bg-alt) 4.5rem
  );
}

.features-container {
  display: flex;
  flex-direction: column;
  gap: 40px;
}

.tagline {
  color: var(--vp-c-text-2);
  font-family: var(--swm-font-family-geometric-humanist);
  font-size: 2.5em;
  font-weight: 400;
  line-height: 1.2;
  margin: 0 auto;
  max-width: 48rem;
  text-align: center;
}

.vp-doc {
  padding: 32px 24px 96px;
}

.vp-doc > h2 {
  border: none;
}

.vp-doc > :first-child {
  margin-top: 0;
}
</style>
