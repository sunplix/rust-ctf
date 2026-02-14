<template>
  <section class="hero">
    <p class="eyebrow">Initialization Complete</p>
    <h1>Rust + Vue3 CTF Platform</h1>
    <p class="summary">
      This is the starter frontend shell for the multi-container CTF platform.
      Next we will implement auth, contest flows, challenge pages, environment control,
      and realtime scoreboard updates.
    </p>

    <div class="cards">
      <article class="card">
        <h2>Player Portal</h2>
        <p>Challenge browsing, flag submit, environment start/reset, live scoreboard.</p>
      </article>
      <article class="card">
        <h2>Admin Console</h2>
        <p>Challenge lifecycle, contest control, team management, runtime monitoring.</p>
      </article>
      <article class="card">
        <h2>Realtime Engine</h2>
        <p>WebSocket updates for rankings, system notices, and environment status.</p>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts"></script>
