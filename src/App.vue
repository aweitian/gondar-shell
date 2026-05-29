<template>
  <div class="app-container">
    <div class="sidebar">
      <div class="sidebar-header">
        <h2>SSH 连接</h2>
        <button class="refresh-btn" @click="loadConnections">刷新</button>
      </div>
      <div class="connection-list">
        <div
          v-for="conn in connections"
          :key="conn.label"
          class="connection-item"
          :class="{ active: selectedConnection?.label === conn.label }"
          @click="selectConnection(conn)"
        >
          <div class="connection-icon">
            <span class="icon">📡</span>
          </div>
          <div class="connection-info">
            <div class="connection-label">{{ conn.label }}</div>
            <div class="connection-host">{{ conn.ip }}:{{ conn.port }}</div>
          </div>
        </div>
      </div>
    </div>

    <div class="main-content">
      <div v-if="!selectedConnection" class="empty-state">
        <div class="empty-icon">🔌</div>
        <h3>请选择一个连接</h3>
        <p>从左侧列表中选择一个 SSH 连接</p>
      </div>

      <div v-else class="terminal-container">
        <div class="terminal-header">
          <div class="terminal-title">
            <span>{{ selectedConnection.label }}</span>
            <span class="status" :class="connectionStatus">{{ statusText }}</span>
          </div>
          <div class="terminal-actions">
            <button
              v-if="connectionStatus === 'disconnected'"
              class="btn-connect"
              @click="connect"
            >
              连接
            </button>
            <button
              v-if="connectionStatus === 'connected'"
              class="btn-disconnect"
              @click="disconnect"
            >
              断开
            </button>
          </div>
        </div>

        <div class="terminal-body">
          <div class="output-area" ref="outputRef">
            <div v-for="(line, index) in outputLines" :key="index" class="output-line">
              <span class="prompt" v-if="line.isPrompt">{{ line.text }}</span>
              <span class="command" v-else-if="line.isCommand">{{ line.text }}</span>
              <span v-else>{{ line.text }}</span>
            </div>
          </div>

          <div class="input-area">
            <span class="prompt">root@{{ selectedConnection.ip }}:~$</span>
            <input
              v-model="commandInput"
              @keydown.enter="sendCommand"
              class="command-input"
              :disabled="connectionStatus !== 'connected'"
              placeholder="输入命令..."
              ref="inputRef"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const connections = ref([])
const selectedConnection = ref(null)
const connectionStatus = ref('disconnected')
const statusText = ref('已断开')
const outputLines = ref([])
const commandInput = ref('')
const outputRef = ref(null)
const inputRef = ref(null)

const loadConnections = async () => {
  try {
    connections.value = await invoke('load_connections')
  } catch (error) {
    console.error('加载连接失败:', error)
  }
}

const selectConnection = (conn) => {
  if (connectionStatus.value === 'connected') {
    disconnect()
  }
  selectedConnection.value = conn
  outputLines.value = []
  outputLines.value.push({ text: `准备连接到 ${conn.label} (${conn.ip}:${conn.port})`, isPrompt: false })
}

const connect = async () => {
  if (!selectedConnection.value) return

  connectionStatus.value = 'connecting'
  statusText.value = '连接中...'
  outputLines.value = []

  try {
    const result = await invoke('connect_ssh', {
      ip: selectedConnection.value.ip,
      port: selectedConnection.value.port,
      username: selectedConnection.value.username,
      password: selectedConnection.value.pass
    })

    connectionStatus.value = 'connected'
    statusText.value = '已连接'
    outputLines.value.push({ text: result, isPrompt: false })
    outputLines.value.push({ text: `root@${selectedConnection.value.ip}:~$ `, isPrompt: true })
    
    await nextTick()
    inputRef.value?.focus()
  } catch (error) {
    connectionStatus.value = 'disconnected'
    statusText.value = '连接失败'
    outputLines.value.push({ text: `错误: ${error}`, isPrompt: false })
  }
}

const disconnect = () => {
  connectionStatus.value = 'disconnected'
  statusText.value = '已断开'
  outputLines.value.push({ text: '连接已断开', isPrompt: false })
}

const sendCommand = async () => {
  if (!commandInput.value.trim() || connectionStatus.value !== 'connected') return

  const cmd = commandInput.value.trim()
  outputLines.value.push({ text: cmd, isCommand: true })

  try {
    const result = await invoke('execute_command', {
      ip: selectedConnection.value.ip,
      port: selectedConnection.value.port,
      username: selectedConnection.value.username,
      password: selectedConnection.value.pass,
      command: cmd
    })

    outputLines.value.push({ text: result, isPrompt: false })
  } catch (error) {
    outputLines.value.push({ text: `错误: ${error}`, isPrompt: false })
  }

  outputLines.value.push({ text: `root@${selectedConnection.value.ip}:~$ `, isPrompt: true })
  commandInput.value = ''
  
  await nextTick()
  const output = outputRef.value
  if (output) {
    output.scrollTop = output.scrollHeight
  }
  const input = inputRef.value
  if (input) {
    input.focus()
  }
}

onMounted(() => {
  loadConnections()
})
</script>

<style scoped>
.app-container {
  display: flex;
  height: 100vh;
  background: #1a1a2e;
}

.sidebar {
  width: 280px;
  background: #16213e;
  border-right: 1px solid #2d3a4f;
  display: flex;
  flex-direction: column;
}

.sidebar-header {
  padding: 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #2d3a4f;
}

.sidebar-header h2 {
  color: #fff;
  font-size: 16px;
  margin: 0;
}

.refresh-btn {
  background: #0f3460;
  border: none;
  color: #fff;
  padding: 6px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}

.refresh-btn:hover {
  background: #1a4d7a;
}

.connection-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.connection-item {
  display: flex;
  align-items: center;
  padding: 12px;
  border-radius: 8px;
  cursor: pointer;
  margin-bottom: 4px;
  transition: background 0.2s;
}

.connection-item:hover {
  background: #0f3460;
}

.connection-item.active {
  background: #0f3460;
  border-left: 3px solid #00d9ff;
}

.connection-icon {
  width: 36px;
  height: 36px;
  background: #0f3460;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 12px;
}

.icon {
  font-size: 18px;
}

.connection-info {
  flex: 1;
}

.connection-label {
  color: #fff;
  font-size: 14px;
  font-weight: 500;
}

.connection-host {
  color: #8892a6;
  font-size: 12px;
  margin-top: 2px;
}

.main-content {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-state {
  text-align: center;
  color: #8892a6;
}

.empty-icon {
  font-size: 64px;
  margin-bottom: 16px;
}

.empty-state h3 {
  font-size: 20px;
  margin-bottom: 8px;
}

.terminal-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #0d1117;
}

.terminal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: #161b22;
  border-bottom: 1px solid #30363d;
}

.terminal-title {
  display: flex;
  align-items: center;
  gap: 12px;
}

.terminal-title span:first-child {
  color: #fff;
  font-weight: 500;
}

.status {
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 12px;
}

.status.disconnected {
  background: #f85149;
  color: #fff;
}

.status.connecting {
  background: #d29922;
  color: #fff;
}

.status.connected {
  background: #3fb950;
  color: #fff;
}

.terminal-actions {
  display: flex;
  gap: 8px;
}

.btn-connect,
.btn-disconnect {
  padding: 6px 16px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}

.btn-connect {
  background: #238636;
  color: #fff;
}

.btn-connect:hover {
  background: #2ea043;
}

.btn-disconnect {
  background: #da3633;
  color: #fff;
}

.btn-disconnect:hover {
  background: #f85149;
}

.terminal-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 16px;
}

.output-area {
  flex: 1;
  overflow-y: auto;
  margin-bottom: 16px;
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 14px;
  line-height: 1.6;
  color: #c9d1d9;
}

.output-line {
  margin-bottom: 4px;
}

.prompt {
  color: #58a6ff;
}

.command {
  color: #d2a8ff;
}

.input-area {
  display: flex;
  align-items: center;
  gap: 8px;
  background: #161b22;
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid #30363d;
}

.input-area .prompt {
  color: #58a6ff;
  font-family: 'Fira Code', 'Consolas', monospace;
}

.command-input {
  flex: 1;
  background: transparent;
  border: none;
  color: #c9d1d9;
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 14px;
  outline: none;
}

.command-input::placeholder {
  color: #484f58;
}

.command-input:disabled {
  opacity: 0.5;
}
</style>
