import type { Entry, EntryKindWidget, WidgetStateDownload } from "$lib/api/types";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

let messages = $state<Entry[]>([]);

listen<Entry[]>("message_history", (event) => {
  messages = event.payload;
  messages.reverse();
}).catch((error) => {
  console.error(`Failed to register users listener: ${error}`);
});

listen<Entry>("message_created", (event) => {
  messages.unshift(event.payload);
}).catch((error) => {
  console.error(`Failed to register users listener: ${error}`);
});

listen<Entry>("widget_updated", (event) => {
  const newMsg = event.payload;
  const oldMsg = messages?.find((m) => m.id === newMsg.id);
  if (oldMsg) {
    Object.assign(oldMsg, newMsg);
  }
});

listen<Entry>("widget_done", (event) => {
  const newMsg = event.payload;
  const oldMsg = messages?.find((m) => m.id === newMsg.id);
  if (oldMsg) {
    Object.assign(oldMsg, newMsg);
  }
});

export const messageStore = {
  get messages(): Entry[] {
    return messages;
  },
  async requestMessageHistory() {
    try {
      await invoke("request_message_history");
    } catch (error) {
      console.log(`Error when requesting message history: ${error}`);
    }
  },
  async sendMessage(content: String) {
    try {
      await invoke("send_chat_message", { content: content });
    } catch (error) {
      console.log(`Error when sending message: ${error}`);
    }
  },
  get activeDownloads(): WidgetStateDownload[] {
    return messages
      .filter((m) => m.kind.type === "widget" && !m.kind.done)
      .map((m) => (m.kind as EntryKindWidget).state)
      .filter((s): s is WidgetStateDownload => s.kind === "download");
  },
};
