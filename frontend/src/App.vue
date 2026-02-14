<template>
  <div class="app-shell">
    <header class="topbar">
      <div class="brand">Rust-CTF</div>
      <nav class="nav">
        <RouterLink to="/">Overview</RouterLink>
      </nav>
    </header>

    <main class="content">
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts"></script>
