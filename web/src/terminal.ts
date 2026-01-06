import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';

class WebTerminal {
  private terminal: Terminal;
  private fitAddon: FitAddon;
  private socket: WebSocket | null = null;
  private statusEl: HTMLElement;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;

  constructor() {
    this.statusEl = document.getElementById('status')!;

    this.terminal = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#d4d4d4',
        cursorAccent: '#1e1e1e',
        selectionBackground: '#264f78',
      },
    });

    this.fitAddon = new FitAddon();
    this.terminal.loadAddon(this.fitAddon);

    const container = document.getElementById('terminal')!;
    this.terminal.open(container);
    this.fitAddon.fit();

    // Handle window resize
    window.addEventListener('resize', () => {
      this.fitAddon.fit();
      this.sendResize();
    });

    // Handle terminal input
    this.terminal.onData((data) => {
      this.send(data);
    });

    // Connect to WebSocket
    this.connect();
  }

  private getAuthToken(): string {
    // Get credentials from browser's basic auth or prompt
    // The credentials are passed via query parameter
    const stored = sessionStorage.getItem('termAuth');
    if (stored) {
      return stored;
    }

    const username = prompt('Username:');
    const password = prompt('Password:');

    if (!username || !password) {
      this.terminal.writeln('\r\n\x1b[31mAuthentication required\x1b[0m');
      throw new Error('Authentication required');
    }

    const token = btoa(`${username}:${password}`);
    sessionStorage.setItem('termAuth', token);
    return token;
  }

  private connect(): void {
    try {
      const auth = this.getAuthToken();
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${protocol}//${window.location.host}/ws?auth=${encodeURIComponent(auth)}`;

      this.setStatus('connecting');
      this.socket = new WebSocket(wsUrl);
      this.socket.binaryType = 'arraybuffer';

      this.socket.onopen = () => {
        this.setStatus('connected');
        this.reconnectAttempts = 0;
        this.terminal.focus();
        this.sendResize();
      };

      this.socket.onmessage = (event) => {
        if (event.data instanceof ArrayBuffer) {
          const data = new Uint8Array(event.data);
          this.terminal.write(data);
        } else {
          this.terminal.write(event.data);
        }
      };

      this.socket.onclose = (event) => {
        this.setStatus('disconnected');

        if (event.code === 1008 || event.code === 4001) {
          // Auth failure
          sessionStorage.removeItem('termAuth');
          this.terminal.writeln('\r\n\x1b[31mAuthentication failed. Refresh to try again.\x1b[0m');
          return;
        }

        this.terminal.writeln('\r\n\x1b[33mConnection closed. Reconnecting...\x1b[0m');
        this.attemptReconnect();
      };

      this.socket.onerror = () => {
        this.setStatus('disconnected');
      };
    } catch (e) {
      this.setStatus('disconnected');
    }
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.terminal.writeln('\r\n\x1b[31mMax reconnect attempts reached. Refresh the page.\x1b[0m');
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * this.reconnectAttempts;

    setTimeout(() => {
      this.connect();
    }, delay);
  }

  private send(data: string): void {
    if (this.socket?.readyState === WebSocket.OPEN) {
      // Send as binary for proper terminal handling
      const encoder = new TextEncoder();
      this.socket.send(encoder.encode(data));
    }
  }

  private sendResize(): void {
    if (this.socket?.readyState === WebSocket.OPEN) {
      const dims = this.fitAddon.proposeDimensions();
      if (dims) {
        this.socket.send(`resize:${dims.cols}:${dims.rows}`);
      }
    }
  }

  private setStatus(status: 'connected' | 'disconnected' | 'connecting'): void {
    this.statusEl.className = status;
    this.statusEl.textContent = status.charAt(0).toUpperCase() + status.slice(1);
  }
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  new WebTerminal();
});
