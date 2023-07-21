<script setup>
import { data as rel } from "../../../github.data";
import { computed } from "vue";
import Snippet from "../Snippet.vue";

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
const ASDF = `
asdf plugin add scarb
asdf install scarb latest
asdf global scarb latest
`.trim();
</script>

<template>
  <template v-if="platform === 'macos' || platform === 'linux'">
    <div class="vp-doc download download-unix">
      <h2>
        Run the following in your terminal, then follow the onscreen
        instructions
      </h2>
      <div class="split">
        <div class="left">
          <h3>Install via quick installation script</h3>
          <Snippet :src="QUICK" class="snippet" lang="shell" />
        </div>
        <div class="right">
          <h3>
            Install via
            <a href="https://asdf-vm.com/" rel="noreferrer" target="_blank"
              >asdf</a
            >
            version manager
          </h3>
          <Snippet :src="ASDF" class="snippet" lang="shell" />
        </div>
      </div>
      <p class="notes">
        You appear to be running macOS or Linux. These commands will install the
        latest stable version of Scarb:
        <code>{{ rel.latestVersion }}</code
        >. For other Scarb versions, platforms or installation methods or
        general help, go to the <a href="/download">download page</a>.
      </p>
    </div>
  </template>
  <template v-else>
    <div class="vp-doc download download-other">
      For all Scarb versions, platforms or installation methods or general help,
      go to the
      <a href="/download">download page</a>.
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

.download-unix > .notes {
  font-size: 0.8em;
  margin-bottom: 0;
}

.download-other {
  font-size: 1.25em;
  text-align: center;
}

.split {
  display: grid;
  grid-gap: 1rem 3rem;
  grid-template-columns: minmax(0, 1fr);
}

@media (min-width: 768px) {
  .split {
    grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
  }
}

.snippet {
  box-shadow: var(--vp-shadow-2);
}
</style>
