<template>
  <div class="container">
    <a-button type="primary" @click="showModal">点击弹出 Hello World</a-button>
    <a-modal v-model:open="isModalOpen" title="提示">
      <p>Hello World</p>
    </a-modal>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const isModalOpen = ref(false)

const showModal = () => {
  isModalOpen.value = true
}
</script>

<style scoped>
.container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100vh;
}
</style>