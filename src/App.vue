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

        <div ref="terminalRef" class="terminal-body"></div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import 'xterm/css/xterm.css'

const connections = ref([])
const selectedConnection = ref(null)
const connectionStatus = ref('disconnected')
const statusText = ref('已断开')
const terminalRef = ref(null)
let terminal = null
let fitAddon = null

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
  initTerminal()
}

const initTerminal = () => {
  if (terminal) {
    terminal.dispose()
  }
  
  terminal = new Terminal({
    cursorBlink: true,
    cursorStyle: 'block',
    fontFamily: '"Fira Code", "Consolas", monospace',
    fontSize: 14,
    lineHeight: 1.6,
    theme: {
      background: '#0d1117',
      foreground: '#c9d1d9',
      cursor: '#58a6ff',
      cursorAccent: '#58a6ff',
      selection: '#264f78',
      black: '#161b22',
      red: '#f85149',
      green: '#3fb950',
      yellow: '#d29922',
      blue: '#58a6ff',
      magenta: '#a371f7',
      cyan: '#56d4dd',
      white: '#c9d1d9',
      brightBlack: '#484f58',
      brightRed: '#ff7b72',
      brightGreen: '#56d364',
      brightYellow: '#e3b341',
      brightBlue: '#79c0ff',
      brightMagenta: '#d2a8ff',
      brightCyan: '#7ee787',
      brightWhite: '#f0f6fc',
    },
  })

  fitAddon = new FitAddon()
  terminal.loadAddon(fitAddon)

  terminal.open(terminalRef.value)
  fitAddon.fit()

  terminal.onData((data) => {
    handleTerminalInput(data)
  })

  terminal.write(`准备连接到 ${selectedConnection.value?.label || '未知'}...\r\n`)
}

const handleTerminalInput = async (data) => {
  if (connectionStatus.value !== 'connected') {
    terminal.write('\r\n请先连接 SSH 服务器\r\n')
    return
  }

  if (data === '\r') {
    terminal.write('\r\n')
    const command = currentCommand.trim()
    if (command) {
      terminal.write(`$ ${command}\r\n`)
      terminal.write('执行中...\r\n')
      currentCommand = ''
      
      try {
        console.log('执行命令:', command)
        const result = await invoke('execute_command', {
          ip: selectedConnection.value.ip,
          port: selectedConnection.value.port,
          username: selectedConnection.value.username,
          password: selectedConnection.value.pass,
          command: command
        })
        console.log('命令结果:', result)
        terminal.write(result)
      } catch (error) {
        console.error('命令执行错误:', error)
        terminal.write(`错误: ${error}\r\n`)
      }
      
      terminal.write('\r\n$ ')
    } else {
      terminal.write('$ ')
    }
  } else if (data === '\x7f') {
    if (currentCommand.length > 0) {
      currentCommand = currentCommand.slice(0, -1)
      terminal.write('\b \b')
    }
  } else {
    currentCommand += data
    terminal.write(data)
  }
}

let currentCommand = ''

const connect = async () => {
  if (!selectedConnection.value) return

  connectionStatus.value = 'connecting'
  statusText.value = '连接中...'
  
  initTerminal()
  terminal.write('连接中...\r\n')

  try {
    const result = await invoke('connect_ssh', {
      ip: selectedConnection.value.ip,
      port: selectedConnection.value.port,
      username: selectedConnection.value.username,
      password: selectedConnection.value.pass
    })

    connectionStatus.value = 'connected'
    statusText.value = '已连接'
    terminal.write(`${result}\r\n`)
    terminal.write('$ ')
  } catch (error) {
    connectionStatus.value = 'disconnected'
    statusText.value = '连接失败'
    terminal.write(`错误: ${error}\r\n`)
  }
}

const disconnect = () => {
  connectionStatus.value = 'disconnected'
  statusText.value = '已断开'
  if (terminal) {
    terminal.write('\r\n连接已断开\r\n')
  }
}

const handleResize = () => {
  if (fitAddon) {
    fitAddon.fit()
  }
}

onMounted(() => {
  loadConnections()
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  if (terminal) {
    terminal.dispose()
  }
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
  padding: 0;
}

.terminal-body :deep(.xterm) {
  height: 100%;
}
</style>
