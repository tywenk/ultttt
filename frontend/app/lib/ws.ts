// websocketService.ts
type MessageHandler = (data: string) => void;

class WebSocketService {
  private static instance: WebSocketService;
  private ws: WebSocket | null = null;
  private isConnecting = false;
  private messageHandlers: Set<MessageHandler> = new Set();
  private reconnectTimeout: number | null = null;

  private constructor() {}

  static getInstance() {
    if (!WebSocketService.instance) {
      WebSocketService.instance = new WebSocketService();
    }
    return WebSocketService.instance;
  }

  public get isConnected() {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  connect() {
    if (this.isConnected || this.isConnecting) {
      return;
    }

    this.isConnecting = true;

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.ws = new WebSocket(`${import.meta.env.VITE_API_WS_BASE_URL}/ws`);

    this.ws.onopen = () => {
      console.log("WebSocket connected");
      this.isConnecting = false;
      if (this.reconnectTimeout) {
        clearTimeout(this.reconnectTimeout);
        this.reconnectTimeout = null;
      }
    };

    this.ws.onmessage = (event) => {
      this.messageHandlers.forEach((handler) => handler(event.data));
    };

    this.ws.onerror = (error) => {
      console.error("WebSocket error:", error);
    };

    this.ws.onclose = () => {
      console.log("WebSocket closed, attempting reconnect...");
      this.isConnecting = false;
      this.reconnectTimeout = window.setTimeout(() => this.connect(), 5000);
    };
  }

  subscribe(handler: MessageHandler) {
    this.messageHandlers.add(handler);
  }

  unsubscribe(handler: MessageHandler) {
    this.messageHandlers.delete(handler);
  }

  send(data: string) {
    if (this.isConnected) {
      this.ws?.send(data);
    }
  }

  disconnect() {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
    this.ws?.close();
    this.ws = null;
    this.isConnecting = false;
  }
}

export const wsService = WebSocketService.getInstance();
