const READ_CHARACTERISTIC_IOCTL = 0x80018483;
const EXPECTED_OUTPUT_LENGTH = 9;
const HEARTBEAT_INTERVAL_MS = 5000;
const RECONNECT_DELAY_MS = 1000;

let host = "127.0.0.1";
let port = 30684;
let connection = null;
let output = null;
let writeChain = Promise.resolve();
let reconnectTimer = null;
let hookInstalled = false;

function asciiBytes(text) {
  const result = [];
  for (let index = 0; index < text.length; index++) {
    result.push(text.charCodeAt(index) & 0xff);
  }
  return result;
}

function hex(pointer, length) {
  if (pointer.isNull() || length <= 0) return "";
  const bytes = new Uint8Array(pointer.readByteArray(length));
  let result = "";
  for (let index = 0; index < bytes.length; index++) {
    result += bytes[index].toString(16).padStart(2, "0");
  }
  return result;
}

function scheduleReconnect() {
  if (reconnectTimer !== null) return;
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connectToHub();
  }, RECONNECT_DELAY_MS);
}

function markDisconnected(currentOutput) {
  if (output !== currentOutput) return;
  output = null;
  connection = null;
  scheduleReconnect();
}

function emit(payload) {
  const currentOutput = output;
  if (currentOutput === null) {
    scheduleReconnect();
    return;
  }
  const line = JSON.stringify(payload) + "\n";
  writeChain = writeChain
    .then(() => currentOutput.writeAll(asciiBytes(line)))
    .catch(() => markDisconnected(currentOutput));
}

async function connectToHub() {
  if (output !== null) return;
  try {
    const currentConnection = await Socket.connect({
      family: "ipv4",
      host: host,
      port: port
    });
    connection = currentConnection;
    output = currentConnection.output;
    emit({ kind: "ready", pid: Process.id, hook_installed: hookInstalled });
  } catch (_error) {
    connection = null;
    output = null;
    scheduleReconnect();
  }
}

function installHook() {
  if (hookInstalled) return;
  const ntdll = Process.findModuleByName("ntdll.dll");
  const target = ntdll ? ntdll.findExportByName("NtDeviceIoControlFile") : null;
  if (target === null) {
    emit({ kind: "error", message: "NtDeviceIoControlFile export not found" });
    return;
  }
  Interceptor.attach(target, {
    onEnter(args) {
      this.capture = args[5].toUInt32() === READ_CHARACTERISTIC_IOCTL;
      if (this.capture) {
        this.output = args[8];
        this.outputLength = args[9].toUInt32();
      }
    },
    onLeave(retval) {
      if (!this.capture || retval.toUInt32() !== 0 || this.output.isNull()) return;
      try {
        if (this.outputLength === EXPECTED_OUTPUT_LENGTH) {
          // 预抑制：先解析 pressed usages，发 gatt_prearm 让 Rust 提前建立抑制窗口，
          // 再发 gatt_read 触发注入。阻止固件原生 VK 与应用注入双触发。
          const forwarded = new Set([
            0x00f1, 0x0028, 0x0035, 0x004a,
            0x004f, 0x0050, 0x0051, 0x0052,
            0x0065, 0x0066, 0x007f, 0x0080, 0x0081,
            0x00e2, 0x00e9, 0x00ea,
          ]);
          const pressedUsages = [];
          for (let offset = 3; offset + 1 < EXPECTED_OUTPUT_LENGTH; offset += 2) {
            const usage = this.output.add(offset).readU16();
            if (usage !== 0 && forwarded.has(usage)) pressedUsages.push(usage);
          }
          if (pressedUsages.length > 0) {
            emit({ kind: "gatt_prearm", raw: hex(this.output, this.outputLength) });
          }
          emit({
            kind: "gatt_read",
            raw: hex(this.output, this.outputLength)
          });
          // zeroing 保留作双保险：阻止固件原生 VK 生成
          // hub 已连接时清掉「应用负责注入」的全部 usage（menu 0x65 翻译成 VK_APPS 会弹
          // 右键菜单、back 0xF1、方向 0x4F-52、OK 0x28、TV 0x35、主页 0x4A、电源 0x66、
          // 音量/静音 0x7F/80/81 及 Consumer 0xE2/E9/EA），改由应用 SendInput 注入单一动作。
          // onLeave 在返回调用方之前执行，此处改缓冲有效。
          if (output !== null) {
            for (let offset = 3; offset + 1 < EXPECTED_OUTPUT_LENGTH; offset += 2) {
              const usage = this.output.add(offset).readU16();
              const mapped =
                usage === 0x00f1 || // Back
                usage === 0x0028 || // OK
                usage === 0x0035 || // TV
                usage === 0x004a || // Home
                usage === 0x004f || usage === 0x0050 || usage === 0x0051 || usage === 0x0052 || // D-pad
                usage === 0x0065 || // Menu (VK_APPS → 右键菜单，必清)
                usage === 0x0066 || // Power
                usage === 0x007f || usage === 0x0080 || usage === 0x0081 || // volume/mute
                usage === 0x00e2 || usage === 0x00e9 || usage === 0x00ea;    // consumer vol/mute
              if (mapped) {
                this.output.add(offset).writeU16(0);
              }
            }
          }
        }
      } catch (error) {
        emit({ kind: "error", message: String(error) });
      }
    }
  });
  hookInstalled = true;
}

setInterval(() => {
  if (output === null) {
    scheduleReconnect();
  } else {
    emit({ kind: "heartbeat", pid: Process.id });
  }
}, HEARTBEAT_INTERVAL_MS);

rpc.exports = {
  async init(_stage, parameters) {
    host = (parameters && parameters.host) || host;
    port = (parameters && parameters.port) || port;
    installHook();
    await connectToHub();
  }
};

// LoadLibrary 注入后部分 Gadget 版本不会立刻调 init：脚本加载时主动挂钩并连 hub
try {
  installHook();
  connectToHub();
} catch (_error) {
  // init 路径仍会重试
}

