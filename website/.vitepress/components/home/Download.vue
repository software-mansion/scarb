<script setup>
import { data as rel } from "../../../github.data";
import { computed } from "vue";
import Snippet from "../Snippet.vue";
import { withBase } from "vitepress";

const platform = computed(() => {
  const platform = window.navigator?.platform;
  if (platform.startsWith("Win")) {
    return "windows";
  } else if (platform.startsWith("Mac")) {
    return "macos";
  } else if (platform.startsWith("Linux")) {
    return "linux";
  } else {
    return "unknown";
  }
});

const QUICK = `curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | sh`;
const STARKUP = `curl --proto '=https' --tlsv1.2 -sSf https://sh.starkup.dev | sh`;
const ASDF = `
asdf plugin add scarb
asdf install scarb latest
asdf set -u scarb latest
`.trim();
</script>

<template>
  <template v-if="platform === 'macos' || platform === 'linux'">
    <div class="vp-doc download download-unix">
      <h2>
        Run the following in your terminal, then follow the onscreen
        instructions
      </h2>
      <div class="installation-method">
        <h3>
          Install via
          <a
            href="https://github.com/software-mansion/starkup"
            rel="noreferrer"
            target="_blank"
            >starkup</a
          >
        </h3>
        <Snippet :src="STARKUP" lang="shell" />
      </div>
      <div class="installation-method">
        <h3>
          Or via
          <a href="https://asdf-vm.com/" rel="noreferrer" target="_blank"
            >asdf</a
          >
          version manager
        </h3>
        <Snippet :src="ASDF" lang="shell" />
      </div>
      <p class="notes">
        You appear to be running macOS or Linux. These commands will install the
        latest stable version of Scarb:
        <code>{{ rel.latestVersion }}</code
        >. For other Scarb versions, platforms or installation methods or
        general help, go to the
        <a :href="withBase('./download')">download page</a>.
      </p>
    </div>
  </template>
  <template v-else>
    <div class="vp-doc download download-other">
      For all Scarb versions, platforms or installation methods or general help,
      go to the
      <a :href="withBase('./download')">download page</a>.
    </div>
  </template>
</template>

<style scoped>
h2,
h3 {
  border: 0;
  margin-top: 0;
  padding-top: 0;
}

h3 {
  font-weight: normal;
}

.download {
  padding: 0;
}

.download-unix {
  text-align: center;
}

.installation-method {
  margin-bottom: 1.5rem;
}

.download-unix > .notes {
  font-size: 0.8em;
  margin-bottom: 0;
}

.download-other {
  font-size: 1.25em;
  text-align: center;
}

@media (min-width: 768px) {
  .split {
    grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
  }
}
</style>
