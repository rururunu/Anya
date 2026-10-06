<script setup lang="ts">
import { useUserAvatar } from "@/services/chat/userAvatar";
import AvatarFrame from "./AvatarFrame.vue";
const source = useUserAvatar();
</script>

<template>
  <AvatarFrame class="user-avatar">
    <img
      class="user-avatar-artwork"
      :class="{ custom: !source.startsWith('data:image/svg+xml') }"
      :src="source"
      alt=""
      draggable="false"
    />
  </AvatarFrame>
</template>

<style scoped>
.user-avatar-artwork {
  display: block;
  flex: none;
  width: 100%;
  height: 100%;
  background: #f0efeb;
  object-fit: cover;
}
.user-avatar-artwork.custom {
  transform: none;
}
</style>
