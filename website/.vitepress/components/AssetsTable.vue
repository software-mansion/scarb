<script setup>
defineProps(["release"]);
</script>

<template>
  <table>
    <thead>
      <tr>
        <th>File name</th>
        <th>Kind</th>
        <th>OS</th>
        <th>Arch</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="asset in release.assets" :key="asset.name">
        <td>
          <a :href="asset.downloadUrl" target="_blank" rel="noreferrer">
            {{ asset.name }}
          </a>
        </td>
        <td>{{ asset.kind }}</td>
        <td>{{ asset.os }}</td>
        <td>{{ asset.arch }}</td>
      </tr>
    </tbody>
  </table>
</template>
